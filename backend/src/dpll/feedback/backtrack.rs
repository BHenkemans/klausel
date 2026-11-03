use super::invariants::{has_decision, last_decision_slot, partial_eval};
use crate::dpll::state::{Trail, TrailEntry};
use crate::state::ClauseSet;

// BacktrackGuard(c) = c|M = box AND exists a decision. Top priority, so the
// full guard adds nothing to applicability.
pub fn backtrack_applicable(phi: &ClauseSet, m: &Trail, c: usize) -> bool {
    phi.get(c)
        .is_some_and(|cl| partial_eval(cl, m).is_conflict())
        && has_decision(m)
}

// M := M_<q + {l_q^c} as a non-decision, for q the last decision slot.
// c does not appear in the thesis effect.
pub fn backtrack_effect(m: &Trail) -> Option<Trail> {
    let q = last_decision_slot(m)?;
    let mut out: Trail = m[..q].to_vec();
    out.push(TrailEntry {
        lit: m[q].lit.compl(),
        decision: false,
    });
    Some(out)
}

pub fn backtrack_guard(phi: &ClauseSet, m: &Trail, c: usize) -> bool {
    backtrack_applicable(phi, m, c)
}
