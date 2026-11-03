use super::invariants::{current_level, has_decision, levels, partial_eval};
use crate::cdcl::state::{State, Trail};
use crate::state::{Clause, ClauseSet, Lit};

pub fn conflict_applicable(phi: &ClauseSet, m: &Trail, c: usize) -> bool {
    phi.get(c)
        .is_some_and(|cl| partial_eval(cl, m).is_conflict())
        && has_decision(m)
}

pub fn conflict_guard(phi: &ClauseSet, m: &Trail, c: usize) -> bool {
    conflict_applicable(phi, m, c)
}

fn slot_of(m: &Trail, l: Lit) -> Option<usize> {
    m.iter().position(|e| e.lit == l)
}

fn involved(s: &State, c: usize, stop: impl Fn(usize) -> bool) -> Vec<bool> {
    let mut involved = vec![false; s.trail.len()];
    let Some(conflicting) = s.phi.get(c) else {
        return involved;
    };
    let mut pending: Vec<usize> = conflicting
        .iter()
        .filter_map(|l| slot_of(&s.trail, l.compl()))
        .collect();
    while let Some(p) = pending.pop() {
        if std::mem::replace(&mut involved[p], true) || stop(p) {
            continue;
        }
        let Some(antecedent) = s.ant[p].and_then(|a| s.phi.get(a)) else {
            continue;
        };
        pending.extend(
            antecedent
                .iter()
                .filter_map(|l| slot_of(&s.trail, l.compl())),
        );
    }
    involved
}

pub fn learned_clause_and_backjump_level(s: &State, c: usize) -> (Clause, usize) {
    let level = levels(&s.trail);
    let current = current_level(&s.trail);
    let stop = |p: usize| s.ant[p].is_none() || level[p] < current;

    let learned_slots: Vec<usize> = involved(s, c, stop)
        .into_iter()
        .enumerate()
        .filter(|&(p, inv)| inv && stop(p))
        .map(|(p, _)| p)
        .collect();

    let beta = learned_slots
        .iter()
        .map(|&p| level[p])
        .filter(|&lv| lv < current)
        .max()
        .unwrap_or(0);

    let mut learned: Clause = learned_slots
        .into_iter()
        .map(|p| s.trail[p].lit.compl())
        .collect();
    learned.sort();
    (learned, beta)
}

pub fn conflict_effect(s: &State, c: usize) -> Option<State> {
    s.phi
        .get(c)
        .filter(|cl| partial_eval(cl, &s.trail).is_conflict())?;
    let (learned, beta) = learned_clause_and_backjump_level(s, c);
    let keep = levels(&s.trail)
        .into_iter()
        .take_while(|&lv| lv <= beta)
        .count();
    let mut phi = s.phi.clone();
    phi.push(learned);
    Some(State {
        phi,
        trail: s.trail[..keep].to_vec(),
        ant: s.ant[..keep].to_vec(),
    })
}
