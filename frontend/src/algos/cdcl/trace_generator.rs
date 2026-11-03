use rand::Rng;

use crate::algos::trace_generator::TraceGenerator;
use backend::cdcl::trace::{Rule, Step, Trace};
use backend::state::Sat;

#[derive(Clone, Debug)]
struct Slot {
    var: Option<usize>,
    decision: bool,
    satisfies: Vec<usize>,
    implied: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Run {
    vars: usize,
    clauses: usize,
    learned: usize,
    trail: Vec<Slot>,
    pub steps: Trace,
    conflict_possible: bool,
    cooling: Vec<usize>,
}

impl Run {
    pub fn new(vars: usize, clauses: usize) -> Self {
        Run {
            vars,
            clauses,
            learned: 0,
            trail: Vec::new(),
            steps: Vec::new(),
            conflict_possible: false,
            cooling: Vec::new(),
        }
    }

    pub fn learned(&self) -> usize {
        self.learned
    }

    pub fn decisions(&self) -> usize {
        self.trail.iter().filter(|s| s.decision).count()
    }

    fn free_clauses(&self) -> Vec<usize> {
        (0..self.clauses + self.learned)
            .filter(|c| !self.trail.iter().any(|s| s.satisfies.contains(c)))
            .collect()
    }

    fn next_var(&self) -> Option<usize> {
        (0..self.vars).find(|&v| !self.trail.iter().any(|s| s.var == Some(v)))
    }

    fn room(&self, slots: usize) -> bool {
        self.trail.len() + slots <= self.vars
    }

    fn unit(&self, c: usize) -> Self {
        let mut next = self.clone();
        if let Some(last) = next.trail.last_mut().filter(|s| s.decision) {
            last.implied = Some(c);
        }
        next.steps.push(Step {
            rule: Rule::UnitProp,
            param: c,
        });
        next.trail.push(Slot {
            var: None,
            decision: false,
            satisfies: vec![c],
            implied: None,
        });
        next.conflict_possible = true;
        next.cooling.clear();
        next
    }

    fn decide(&self, v: usize, neg: bool) -> Self {
        let mut next = self.clone();
        next.steps.push(Step {
            rule: Rule::Decide,
            param: 2 * v + usize::from(neg),
        });
        next.trail.push(Slot {
            var: Some(v),
            decision: true,
            satisfies: vec![],
            implied: None,
        });
        next.conflict_possible = false;
        next.cooling.clear();
        next
    }

    fn learn(&self, c: usize, level: usize) -> Self {
        let mut next = self.clone();
        let asserted = self
            .trail
            .iter()
            .rev()
            .find(|s| s.decision)
            .cloned()
            .expect("a decision to learn from");
        let keep = self
            .trail
            .iter()
            .scan(0, |d, s| {
                *d += usize::from(s.decision);
                Some(*d <= level)
            })
            .take_while(|&k| k)
            .count();
        let removed = next.trail.split_off(keep);
        let learned = self.clauses + self.learned;
        next.learned += 1;
        next.steps.push(Step {
            rule: Rule::Conflict,
            param: c,
        });
        next.steps.push(Step {
            rule: Rule::UnitProp,
            param: learned,
        });
        next.trail.push(Slot {
            var: asserted.var,
            decision: false,
            satisfies: [Some(learned), Some(c), asserted.implied]
                .into_iter()
                .flatten()
                .collect(),
            implied: None,
        });
        next.cooling = removed
            .into_iter()
            .filter(|s| !s.decision)
            .flat_map(|s| s.satisfies)
            .collect();
        next.conflict_possible = false;
        next
    }
}

impl TraceGenerator for Run {
    fn len(&self) -> usize {
        self.steps.len()
    }

    fn successors(&self, rng: &mut impl Rng) -> Vec<Vec<Self>> {
        let free = self.free_clauses();
        let units = free
            .iter()
            .filter(|c| self.room(1) && !self.cooling.contains(c))
            .map(|&c| self.unit(c))
            .collect();
        let decides = self
            .next_var()
            .filter(|_| self.room(2))
            .map(|v| self.decide(v, rng.random_bool(0.5)))
            .into_iter()
            .collect();
        let conflicts = if self.conflict_possible {
            free.iter()
                .flat_map(|&c| (0..self.decisions()).map(move |level| (c, level)))
                .map(|(c, level)| self.learn(c, level))
                .collect()
        } else {
            vec![]
        };
        vec![units, decides, conflicts]
    }

    fn closing(&self, target: Sat) -> Option<usize> {
        let backjump = if self.decisions() > 0 { 3 } else { 0 };
        let free = self.free_clauses().len();
        match target {
            Sat::Sat => Some(usize::from(self.trail.is_empty())),
            Sat::Unsat if self.conflict_possible && free >= 1 => Some(backjump),
            Sat::Unsat => (self.room(1) && free >= 2).then_some(1 + backjump),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_conflict_asserts_the_negated_decision_and_propagates_the_learned_clause() {
        let run = Run::new(4, 3)
            .decide(0, false)
            .decide(1, false)
            .unit(0)
            .unit(1)
            .learn(2, 1);
        assert_eq!(
            run.steps.last().map(|s| (s.rule, s.param)),
            Some((Rule::UnitProp, 3))
        );
        assert_eq!(run.decisions(), 1);
        assert_eq!(run.free_clauses(), vec![1]);
        assert_eq!(run.cooling, vec![0, 1]);
        assert_eq!(run.next_var(), Some(2));
        assert!(!run.conflict_possible);
    }
}
