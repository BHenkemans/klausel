pub mod answer;
pub mod conflict;
pub mod decide;
pub mod invariants;
pub mod unit;

use crate::cdcl::state::State;
use crate::cdcl::trace::{Rule, Step};
use crate::feedback::Algorithm;
use crate::state::{Lit, Sat, literals};
use invariants::success_guard;

pub struct Cdcl;

impl Algorithm for Cdcl {
    type State = State;
    type Step = Step;
    type Rule = Rule;

    fn steps(&self, s: &State) -> Vec<Step> {
        let clauses: Vec<usize> = (0..s.phi.len()).collect();
        let lits: Vec<usize> = literals(&s.phi).into_iter().map(|l| l.0).collect();
        [
            (Rule::Conflict, &clauses),
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

    fn guard(&self, s: &State, t: Step) -> bool {
        match t.rule {
            Rule::Conflict => conflict::conflict_guard(&s.phi, &s.trail, t.param),
            Rule::UnitProp => unit::unit_guard(&s.phi, &s.trail, t.param),
            Rule::Decide => decide::decide_guard(&s.phi, &s.trail, Lit(t.param)),
        }
    }

    fn applicable(&self, s: &State, t: Step) -> bool {
        match t.rule {
            Rule::Conflict => conflict::conflict_applicable(&s.phi, &s.trail, t.param),
            Rule::UnitProp => unit::unit_applicable(&s.phi, &s.trail, t.param),
            Rule::Decide => decide::decide_applicable(&s.phi, &s.trail, Lit(t.param)),
        }
    }

    fn effect(&self, s: &State, t: Step) -> Option<State> {
        match t.rule {
            Rule::Conflict => conflict::conflict_effect(s, t.param),
            Rule::UnitProp => unit::unit_effect(s, t.param),
            Rule::Decide => Some(decide::decide_effect(s, Lit(t.param))),
        }
    }

    fn result(&self, s: &State) -> Sat {
        if success_guard(&s.phi, &s.trail) {
            Sat::Sat
        } else {
            Sat::Unsat
        }
    }
}
