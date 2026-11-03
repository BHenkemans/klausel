use crate::cdcl::state::Trail;

pub use crate::dpll::feedback::invariants::{
    PartialEval, fail_guard, find_conflict, find_unit_clause, has_decision, partial_eval,
    success_guard,
};

pub fn levels(m: &Trail) -> Vec<usize> {
    m.iter()
        .scan(0, |level, e| {
            *level += usize::from(e.decision);
            Some(*level)
        })
        .collect()
}

pub fn current_level(m: &Trail) -> usize {
    m.iter().filter(|e| e.decision).count()
}
