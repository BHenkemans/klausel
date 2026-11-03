pub mod backtrack;
pub mod decide;
pub mod invariants;
pub mod unit;

use crate::dpll::state::Trail;
use crate::dpll::trace::{Rule, Step};
use crate::feedback::Algorithm;
use crate::state::{ClauseSet, Lit, Sat, literals};
use invariants::success_guard;

pub struct Dpll {
    pub phi: ClauseSet,
}

impl Algorithm for Dpll {
    type State = Trail;
    type Step = Step;
    type Rule = Rule;

    fn steps(&self, _: &Trail) -> Vec<Step> {
        let clauses: Vec<usize> = (0..self.phi.len()).collect();
        let lits: Vec<usize> = literals(&self.phi).into_iter().map(|l| l.0).collect();
        [
            (Rule::Backtrack, &clauses),
            (Rule::UnitProp, &clauses),
            (Rule::Decide, &lits),
        ]
        .into_iter()
        .flat_map(|(rule, params)| params.iter().map(move |&param| Step { rule, param }))
        .collect()
    }

    fn rule(t: Step) -> Rule {
        t.rule
    }

    fn guard(&self, m: &Trail, t: Step) -> bool {
        match t.rule {
            Rule::Backtrack => backtrack::backtrack_guard(&self.phi, m, t.param),
            Rule::UnitProp => unit::unit_guard(&self.phi, m, t.param),
            Rule::Decide => decide::decide_guard(&self.phi, m, Lit(t.param)),
        }
    }

    fn applicable(&self, m: &Trail, t: Step) -> bool {
        match t.rule {
            Rule::Backtrack => backtrack::backtrack_applicable(&self.phi, m, t.param),
            Rule::UnitProp => unit::unit_applicable(&self.phi, m, t.param),
            Rule::Decide => decide::decide_applicable(&self.phi, m, Lit(t.param)),
        }
    }

    fn effect(&self, m: &Trail, t: Step) -> Option<Trail> {
        match t.rule {
            Rule::Backtrack => backtrack::backtrack_effect(m),
            Rule::UnitProp => unit::unit_effect(&self.phi, m, t.param),
            Rule::Decide => Some(decide::decide_effect(m, Lit(t.param))),
        }
    }

    fn result(&self, m: &Trail) -> Sat {
        if success_guard(&self.phi, m) {
            Sat::Sat
        } else {
            Sat::Unsat
        }
    }
}
