use super::Cdcl;
use super::invariants::partial_eval;
use crate::cdcl::state::{State, Trail, sorted_clauses};
use crate::cdcl::trace::Step;
use crate::feedback::Algorithm;
use crate::state::ClauseSet;

pub fn read_answer(from: &State, t: Step, phi: ClauseSet, trail: Trail) -> State {
    let phi = sorted_clauses(phi);
    let expected = Cdcl.effect(from, t);
    let ant = trail
        .iter()
        .enumerate()
        .map(|(q, e)| {
            if e.decision {
                return None;
            }
            let is_reason = |a: &usize| {
                phi.get(*a)
                    .is_some_and(|c| partial_eval(c, &trail[..q]).unit() == Some(e.lit))
            };
            let named = [expected.as_ref(), Some(from)]
                .into_iter()
                .flatten()
                .find(|s| s.trail.get(q) == Some(e))
                .and_then(|s| s.ant[q]);
            named
                .filter(is_reason)
                .or_else(|| (0..phi.len()).find(is_reason))
        })
        .collect();
    State { phi, trail, ant }
}
