use super::feedback::Cdcl;
use super::state::State;
use crate::feedback::{Run, forward};
use crate::state::ClauseSet;

pub type ForwardRun = Run<Cdcl>;

pub fn forward_cdcl(phi: &ClauseSet) -> ForwardRun {
    forward(&Cdcl, &State::initial(phi.clone()))
}
