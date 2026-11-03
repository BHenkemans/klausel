use super::super::state::{Trail, TrailEntry, on_trail};
use crate::state::{Clause, ClauseSet, Lit};

/// Partial evaluation c|M. `True` when some literal of c is on the trail,
/// otherwise the literals of c not yet falsified. Empty Unresolved is the box.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PartialEval {
    True,
    Unresolved(Vec<Lit>),
}

impl PartialEval {
    pub fn is_conflict(&self) -> bool {
        matches!(self, PartialEval::Unresolved(ls) if ls.is_empty())
    }

    /// The single survivor when |c|M| = 1.
    pub fn unit(&self) -> Option<Lit> {
        match self {
            PartialEval::Unresolved(ls) if ls.len() == 1 => Some(ls[0]),
            _ => None,
        }
    }
}

pub fn partial_eval(c: &Clause, m: &[TrailEntry]) -> PartialEval {
    if c.iter().any(|&l| on_trail(m, l)) {
        return PartialEval::True;
    }
    PartialEval::Unresolved(
        c.iter()
            .copied()
            .filter(|&l| !on_trail(m, l.compl()))
            .collect(),
    )
}

pub fn success_guard(phi: &ClauseSet, m: &Trail) -> bool {
    phi.iter().all(|c| partial_eval(c, m) == PartialEval::True)
}

pub fn fail_guard(phi: &ClauseSet, m: &Trail) -> bool {
    find_conflict(phi, m).is_some() && !has_decision(m)
}

pub fn find_conflict(phi: &ClauseSet, m: &Trail) -> Option<usize> {
    phi.iter().position(|c| partial_eval(c, m).is_conflict())
}

pub fn find_unit_clause(phi: &ClauseSet, m: &Trail) -> Option<usize> {
    phi.iter().position(|c| partial_eval(c, m).unit().is_some())
}

pub fn has_decision(m: &Trail) -> bool {
    m.iter().any(|e| e.decision)
}

pub fn last_decision_slot(m: &Trail) -> Option<usize> {
    m.iter().rposition(|e| e.decision)
}
