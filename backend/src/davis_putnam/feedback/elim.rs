use super::invariants::{find_pure, find_unit, unsat_guard};
use crate::state::{Clause, ClauseSet, Lit, canonical_set, norm_clause};

pub fn elim_applicable(s: &ClauseSet, p: usize) -> bool {
    let pos = Lit(2 * p);
    let neg = pos.compl();
    !unsat_guard(s)
        && s.iter().flatten().any(|&x| x == pos)
        && s.iter().flatten().any(|&x| x == neg)
}

// None is ElimRes's "Res(c,d) is not a tautology" side condition.
pub fn resolve(c: &Clause, d: &Clause, p: usize) -> Option<Clause> {
    let pos = Lit(2 * p);
    let neg = pos.compl();
    let r: Clause = c
        .iter()
        .chain(d.iter())
        .copied()
        .filter(|&x| x != pos && x != neg)
        .collect();
    norm_clause(r)
}

// A clause holding both p and pc would fall in neither set; it cannot exist,
// since canonical_set drops tautologies.
pub fn elim_effect(s: &ClauseSet, p: usize) -> ClauseSet {
    let pos = Lit(2 * p);
    let neg = pos.compl();

    let mut out: ClauseSet = s
        .iter()
        .filter(|c| !c.contains(&pos) && !c.contains(&neg))
        .cloned()
        .collect();

    for c in s.iter().filter(|c| c.contains(&pos)) {
        for d in s.iter().filter(|d| d.contains(&neg)) {
            if let Some(r) = resolve(c, d, p) {
                out.push(r);
            }
        }
    }

    canonical_set(out)
}

pub fn elim_guard(s: &ClauseSet, p: usize) -> bool {
    elim_applicable(s, p) && find_unit(s).is_none() && find_pure(s).is_none()
}
