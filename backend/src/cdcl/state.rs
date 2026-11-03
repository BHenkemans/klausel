use crate::state::ClauseSet;

pub use crate::dpll::state::{Trail, TrailEntry, on_trail};

pub type Antecedents = Vec<Option<usize>>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct State {
    pub phi: ClauseSet,
    pub trail: Trail,
    pub ant: Antecedents,
}

impl State {
    pub fn initial(phi: ClauseSet) -> Self {
        State {
            phi: sorted_clauses(phi),
            trail: Vec::new(),
            ant: Vec::new(),
        }
    }
}

pub fn sorted_clauses(phi: ClauseSet) -> ClauseSet {
    phi.into_iter()
        .map(|mut c| {
            c.sort();
            c.dedup();
            c
        })
        .collect()
}
