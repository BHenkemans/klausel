use crate::cdcl::state::State;
use crate::dpll::feedback::unit as dpll_unit;

pub use dpll_unit::{unit_applicable, unit_guard};

pub fn unit_effect(s: &State, c: usize) -> Option<State> {
    let trail = dpll_unit::unit_effect(&s.phi, &s.trail, c)?;
    let mut ant = s.ant.clone();
    ant.push(Some(c));
    Some(State {
        phi: s.phi.clone(),
        trail,
        ant,
    })
}
