use super::invariants::{fail_guard, find_conflict, partial_eval};
use crate::dpll::state::{Trail, TrailEntry};
use crate::state::ClauseSet;

// UnitGuard(c) = not(Success) AND not(Fail) AND forall c': not(BacktrackGuard(c'))
// AND |c|M| = 1. The third conjunct is priority; not(Success) follows from the last.
pub fn unit_applicable(phi: &ClauseSet, m: &Trail, c: usize) -> bool {
    phi.get(c)
        .is_some_and(|cl| partial_eval(cl, m).unit().is_some())
        && !fail_guard(phi, m)
}

// M := M + {l} as a non-decision, l the unit literal of c|M.
pub fn unit_effect(phi: &ClauseSet, m: &Trail, c: usize) -> Option<Trail> {
    let lit = partial_eval(phi.get(c)?, m).unit()?;
    let mut out = m.clone();
    out.push(TrailEntry {
        lit,
        decision: false,
    });
    Some(out)
}

pub fn unit_guard(phi: &ClauseSet, m: &Trail, c: usize) -> bool {
    unit_applicable(phi, m, c) && find_conflict(phi, m).is_none()
}
