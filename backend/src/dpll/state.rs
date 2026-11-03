use crate::state::Lit;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TrailEntry {
    pub lit: Lit,
    pub decision: bool,
}
pub type Trail = Vec<TrailEntry>;

pub fn on_trail(m: &[TrailEntry], l: Lit) -> bool {
    m.iter().any(|e| e.lit == l)
}
