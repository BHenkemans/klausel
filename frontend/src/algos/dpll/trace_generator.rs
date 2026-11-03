use rand::Rng;

use crate::algos::trace_generator::TraceGenerator;
use backend::dpll::trace::{Rule, Step, Trace};
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
    trail: Vec<Slot>,
    pub steps: Trace,
    conflict_possible: bool,
    cooling: Vec<usize>,
    peak: usize,
}

impl Run {
    pub fn new(vars: usize, clauses: usize) -> Self {
        Run {
            vars,
            clauses,
            trail: Vec::new(),
            steps: Vec::new(),
            conflict_possible: false,
            cooling: Vec::new(),
            peak: 0,
        }
    }

    pub fn fills_every_variable(&self) -> bool {
        self.peak == self.vars
    }

    fn decisions(&self) -> usize {
        self.trail.iter().filter(|s| s.decision).count()
    }

    fn free_clauses(&self) -> Vec<usize> {
        (0..self.clauses)
            .filter(|c| !self.trail.iter().any(|s| s.satisfies.contains(c)))
            .collect()
    }

    fn next_var(&self) -> Option<usize> {
        (0..self.vars).find(|&v| !self.trail.iter().any(|s| s.var == Some(v)))
    }

    fn room(&self, slots: usize) -> bool {
        self.trail.len() + slots <= self.vars
    }

    fn then(&self, step: Step, slot: Slot) -> Self {
        let mut next = self.clone();
        if let Some(last) = next
            .trail
            .last_mut()
            .filter(|s| s.decision && step.rule == Rule::UnitProp)
        {
            last.implied = Some(step.param);
        }
        next.steps.push(step);
        next.cooling.clear();
        if step.rule == Rule::Backtrack {
            let q = next
                .trail
                .iter()
                .rposition(|s| s.decision)
                .expect("a decision to undo");
            next.cooling = next.trail.drain(q..).flat_map(|s| s.satisfies).collect();
            next.cooling.push(step.param);
        }
        next.trail.push(slot);
        next.peak = next.peak.max(next.trail.len());
        next.conflict_possible = step.rule == Rule::UnitProp;
        next
    }

    fn unit(&self, c: usize) -> Self {
        self.then(
            Step {
                rule: Rule::UnitProp,
                param: c,
            },
            Slot {
                var: None,
                decision: false,
                satisfies: vec![c],
                implied: None,
            },
        )
    }

    fn decide(&self, v: usize, neg: bool) -> Self {
        self.then(
            Step {
                rule: Rule::Decide,
                param: 2 * v + usize::from(neg),
            },
            Slot {
                var: Some(v),
                decision: true,
                satisfies: vec![],
                implied: None,
            },
        )
    }

    fn backtrack(&self, c: usize) -> Self {
        let q = self
            .trail
            .iter()
            .rposition(|s| s.decision)
            .expect("a decision to undo");
        let decision = &self.trail[q];
        let conflict_contains_negated_decision = self.trail.len() - q == 2;
        let satisfies = decision
            .implied
            .into_iter()
            .chain(conflict_contains_negated_decision.then_some(c))
            .collect();
        self.then(
            Step {
                rule: Rule::Backtrack,
                param: c,
            },
            Slot {
                var: decision.var,
                decision: false,
                satisfies,
                implied: None,
            },
        )
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
        let backtracks = if self.conflict_possible && self.decisions() > 0 {
            free.iter().map(|&c| self.backtrack(c)).collect()
        } else {
            vec![]
        };
        vec![units, decides, backtracks]
    }

    fn closing(&self, target: Sat) -> Option<usize> {
        let unwind = 2 * self.decisions();
        let free = self.free_clauses().len();
        match target {
            Sat::Sat => Some(usize::from(self.trail.is_empty())),
            Sat::Unsat if self.conflict_possible && free >= 1 => Some(unwind),
            Sat::Unsat => (self.room(1) && free >= 2).then_some(1 + unwind),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clause_implied_by_a_decision_stays_blocked_by_its_flip() {
        let run = Run::new(4, 4)
            .unit(0)
            .decide(1, false)
            .unit(1)
            .unit(2)
            .backtrack(3);
        assert_eq!(run.free_clauses(), vec![2, 3]);
        assert_eq!(run.cooling, vec![1, 2, 3]);
        assert!(!run.conflict_possible);
    }
}
