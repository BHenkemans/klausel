use rand::Rng;

use crate::algos::trace_generator::TraceGenerator;
use backend::davis_putnam::generate::trace::step_var;
use backend::davis_putnam::trace::{Rule, Step, Trace};
use backend::state::Sat;

#[derive(Clone, Debug)]
pub struct Run {
    vars: usize,
    pub steps: Trace,
}

impl Run {
    pub fn new(vars: usize) -> Self {
        Run {
            vars,
            steps: Vec::new(),
        }
    }

    fn unspent(&self) -> Vec<usize> {
        (0..self.vars)
            .filter(|&v| !self.steps.iter().any(|s| step_var(s) == v))
            .collect()
    }

    fn then(&self, rule: Rule, param: usize) -> Self {
        let mut next = self.clone();
        next.steps.push(Step { rule, param });
        next
    }

    fn literal(v: usize, rng: &mut impl Rng) -> usize {
        2 * v + usize::from(rng.random_bool(0.5))
    }
}

impl TraceGenerator for Run {
    fn len(&self) -> usize {
        self.steps.len()
    }

    fn successors(&self, rng: &mut impl Rng) -> Vec<Vec<Self>> {
        let unspent = self.unspent();
        let shared = if unspent.len() >= 2 {
            unspent.clone()
        } else {
            vec![]
        };
        let after_pure = self.steps.last().is_some_and(|s| s.rule == Rule::Pure);
        let unit_vars = if after_pure { vec![] } else { unspent.clone() };
        let units = unit_vars
            .iter()
            .map(|&v| self.then(Rule::Unit, Self::literal(v, rng)))
            .collect();
        let pures = shared
            .iter()
            .map(|&v| self.then(Rule::Pure, Self::literal(v, rng)))
            .collect();
        let elims = shared.iter().map(|&v| self.then(Rule::Elim, v)).collect();
        vec![units, pures, elims]
    }

    fn closing(&self, target: Sat) -> Option<usize> {
        let last = self.steps.last().map(|s| s.rule);
        let unspent = self.unspent().len();
        match (target, last) {
            (Sat::Sat, None) => Some(1),
            (Sat::Sat, Some(_)) => Some(0),
            (Sat::Unsat, Some(Rule::Unit)) => Some(0),
            (Sat::Unsat, Some(Rule::Pure)) => (unspent >= 2).then_some(2),
            (Sat::Unsat, _) => (unspent >= 1).then_some(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::trace_generator::walk;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn every_walk_spends_each_variable_once_and_ends_unsat_on_a_unit() {
        let mut rng = StdRng::seed_from_u64(0xD9);
        for vars in 2..=8 {
            for len in 1..=vars {
                for target in [Sat::Sat, Sat::Unsat] {
                    for _ in 0..50 {
                        let Some(run) = walk(Run::new(vars), len, target, &mut rng) else {
                            continue;
                        };
                        let mut used: Vec<usize> = run.steps.iter().map(step_var).collect();
                        used.sort();
                        used.dedup();
                        assert_eq!(used.len(), len);
                        for (k, s) in run.steps.iter().enumerate() {
                            assert!(s.rule == Rule::Unit || vars - k >= 2, "{:?}", run.steps);
                        }
                        for w in run.steps.windows(2) {
                            assert!(
                                !(w[0].rule == Rule::Pure && w[1].rule == Rule::Unit),
                                "{:?}",
                                run.steps
                            );
                        }
                        if target == Sat::Unsat {
                            assert_eq!(run.steps.last().map(|s| s.rule), Some(Rule::Unit));
                        }
                    }
                }
            }
        }
    }
}
