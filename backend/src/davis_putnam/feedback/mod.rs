pub mod elim;
pub mod invariants;
pub mod pure;
pub mod unit;

use crate::davis_putnam::trace::{Rule, Step};
use crate::feedback::Algorithm;
use crate::state::{ClauseSet, Lit, Sat, literals};
use invariants::unsat_guard;

pub struct Dp;

pub fn apply_rule(s: &ClauseSet, t: Step) -> ClauseSet {
    match t.rule {
        Rule::Unit => unit::unit_effect(s, Lit(t.param)),
        Rule::Pure => pure::pure_effect(s, Lit(t.param)),
        Rule::Elim => elim::elim_effect(s, t.param),
    }
}

impl Algorithm for Dp {
    type State = ClauseSet;
    type Step = Step;
    type Rule = Rule;

    fn steps(&self, s: &ClauseSet) -> Vec<Step> {
        let lits: Vec<usize> = literals(s).into_iter().map(|l| l.0).collect();
        let mut atoms: Vec<usize> = lits.iter().map(|l| l / 2).collect();
        atoms.dedup();
        [
            (Rule::Unit, &lits),
            (Rule::Pure, &lits),
            (Rule::Elim, &atoms),
        ]
        .into_iter()
        .flat_map(|(rule, params)| params.iter().map(move |&param| Step { rule, param }))
        .collect()
    }

    fn rule(t: Step) -> Rule {
        t.rule
    }

    fn guard(&self, s: &ClauseSet, t: Step) -> bool {
        match t.rule {
            Rule::Unit => unit::unit_guard(s, Lit(t.param)),
            Rule::Pure => pure::pure_guard(s, Lit(t.param)),
            Rule::Elim => elim::elim_guard(s, t.param),
        }
    }

    fn applicable(&self, s: &ClauseSet, t: Step) -> bool {
        match t.rule {
            Rule::Unit => unit::unit_applicable(s, Lit(t.param)),
            Rule::Pure => pure::pure_applicable(s, Lit(t.param)),
            Rule::Elim => elim::elim_applicable(s, t.param),
        }
    }

    fn effect(&self, s: &ClauseSet, t: Step) -> Option<ClauseSet> {
        Some(apply_rule(s, t))
    }

    fn result(&self, s: &ClauseSet) -> Sat {
        if unsat_guard(s) { Sat::Unsat } else { Sat::Sat }
    }
}
