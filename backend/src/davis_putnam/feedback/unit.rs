use super::invariants::unsat_guard;
use crate::state::{ClauseSet, Lit, canonical_set};

pub fn unit_applicable(s: &ClauseSet, l: Lit) -> bool {
    !unsat_guard(s) && s.iter().any(|c| c.len() == 1 && c[0] == l)
}

pub fn unit_effect(s: &ClauseSet, l: Lit) -> ClauseSet {
    let not_l = l.compl();
    let out: ClauseSet = s
        .iter()
        .filter(|c| !c.contains(&l))
        .map(|c| c.iter().copied().filter(|&x| x != not_l).collect())
        .collect();
    canonical_set(out)
}

pub fn unit_guard(s: &ClauseSet, l: Lit) -> bool {
    unit_applicable(s, l)
}
