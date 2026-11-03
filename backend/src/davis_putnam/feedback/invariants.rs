use crate::state::{ClauseSet, Lit, literals};

pub fn unsat_guard(s: &ClauseSet) -> bool {
    s.iter().any(|c| c.is_empty())
}

// Smallest literal with a unit clause, so the choice does not depend on the
// order clauses happen to sit in the Vec.
pub fn find_unit(s: &ClauseSet) -> Option<Lit> {
    s.iter().filter(|c| c.len() == 1).map(|c| c[0]).min()
}

pub fn find_pure(s: &ClauseSet) -> Option<Lit> {
    let lits = literals(s);
    lits.iter()
        .copied()
        .find(|l| lits.binary_search(&l.compl()).is_err())
    // You fail binary search -> You found a pure
}
