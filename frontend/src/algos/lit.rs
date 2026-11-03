//! The one statement of the frontend↔backend literal encoding.
//!
//! The views carry literals as 1-based signed `i32` (`-2` is ¬β); the backends
//! index them as `2·var + polarity` over 0-based variables. Getting the
//! polarity backwards here would silently corrupt every generated formula and
//! every graded step, so the convention is written once rather than per
//! `#[server]` function.

#![cfg(feature = "server")]

use backend::state::{ClauseSet, Lit};

pub fn to_lit(l: i32) -> Lit {
    Lit(2 * (l.unsigned_abs() as usize - 1) + usize::from(l < 0))
}

pub fn from_lit(l: Lit) -> i32 {
    let v = (l.atom() + 1) as i32;
    if l.0.is_multiple_of(2) { v } else { -v }
}

/// A signed clause set to the backend's. `0` is not a literal under this
/// encoding, so it is dropped rather than wrapped to variable `-1`.
pub fn to_phi(set: &[Vec<i32>]) -> ClauseSet {
    set.iter()
        .map(|c| c.iter().copied().filter(|l| *l != 0).map(to_lit).collect())
        .collect()
}

pub fn from_phi(phi: &ClauseSet) -> Vec<Vec<i32>> {
    phi.iter()
        .map(|c| c.iter().copied().map(from_lit).collect())
        .collect()
}
