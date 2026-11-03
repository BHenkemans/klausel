use super::feedback::Dpll;
use crate::feedback::{Run, forward};
use crate::state::ClauseSet;

pub type ForwardRun = Run<Dpll>;

pub fn forward_dpll(phi: &ClauseSet) -> ForwardRun {
    forward(&Dpll { phi: phi.clone() }, &Vec::new())
}
