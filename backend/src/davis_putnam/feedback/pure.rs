use super::invariants::{find_unit, unsat_guard};
use crate::state::{ClauseSet, Lit, canonical_set};

pub fn pure_applicable(s: &ClauseSet, l: Lit) -> bool {
    let not_l = l.compl();
    !unsat_guard(s)
        && s.iter().flatten().any(|&x| x == l)
        && s.iter().flatten().all(|&x| x != not_l)
}

pub fn pure_effect(s: &ClauseSet, l: Lit) -> ClauseSet {
    canonical_set(s.iter().filter(|c| !c.contains(&l)).cloned().collect())
}

pub fn pure_guard(s: &ClauseSet, l: Lit) -> bool {
    pure_applicable(s, l) && find_unit(s).is_none()
}
