use super::feedback::Dp;
use crate::feedback::{Run, forward};
use crate::state::{ClauseSet, canonical_set};

pub type ForwardRun = Run<Dp>;

pub fn forward_dp(s0: &ClauseSet) -> ForwardRun {
    forward(&Dp, &canonical_set(s0.clone()))
}
