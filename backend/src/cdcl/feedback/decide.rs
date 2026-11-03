use crate::cdcl::state::State;
use crate::dpll::feedback::decide as dpll_decide;
use crate::state::Lit;

pub use dpll_decide::{decide_applicable, decide_guard};

pub fn decide_effect(s: &State, l: Lit) -> State {
    let mut ant = s.ant.clone();
    ant.push(None);
    State {
        phi: s.phi.clone(),
        trail: dpll_decide::decide_effect(&s.trail, l),
        ant,
    }
}
