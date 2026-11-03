use super::invariants::{fail_guard, find_conflict, find_unit_clause, success_guard};
use crate::dpll::state::{Trail, TrailEntry, on_trail};
use crate::state::{ClauseSet, Lit};

// DecideGuard(l) = not(Success) AND not(Fail) AND forall c: not(BacktrackGuard(c))
// AND forall c: not(UnitGuard(c)) AND l not in M AND l^c not in M.
// The middle two conjuncts are priority.
pub fn decide_applicable(phi: &ClauseSet, m: &Trail, l: Lit) -> bool {
    !on_trail(m, l) && !on_trail(m, l.compl()) && !success_guard(phi, m) && !fail_guard(phi, m)
}

// M := M + {l} as a decision.
pub fn decide_effect(m: &Trail, l: Lit) -> Trail {
    let mut out = m.clone();
    out.push(TrailEntry {
        lit: l,
        decision: true,
    });
    out
}

pub fn decide_guard(phi: &ClauseSet, m: &Trail, l: Lit) -> bool {
    decide_applicable(phi, m, l)
        && find_conflict(phi, m).is_none()
        && find_unit_clause(phi, m).is_none()
}
