use crate::algos::cdcl::state::{
    CdclGradedStep, CdclRule, CdclState, CdclStep, GenerateOutcome, GeneratedExercise, Sat,
};
use crate::algos::feedback::TraceVerdict;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::algos::lit::{to_lit, to_phi};

#[cfg(feature = "server")]
const TIMEOUT_SECS: u64 = 10;

#[cfg(feature = "server")]
mod convert {
    use crate::algos::cdcl::state::{CdclState, TrailEntry};
    use backend::cdcl::state::{State, Trail, TrailEntry as BEntry, sorted_clauses};

    use crate::algos::lit::{from_lit, from_phi, to_lit, to_phi};

    pub fn to_trail(t: &[TrailEntry]) -> Trail {
        t.iter()
            .map(|e| BEntry {
                lit: to_lit(e.lit),
                decision: e.decision,
            })
            .collect()
    }

    pub fn to_state(s: &CdclState) -> State {
        State {
            phi: sorted_clauses(to_phi(&s.phi)),
            trail: to_trail(&s.trail),
            ant: s.trail.iter().map(|e| e.antecedent).collect(),
        }
    }

    pub fn from_state(s: &State) -> CdclState {
        CdclState {
            phi: from_phi(&s.phi),
            trail: s
                .trail
                .iter()
                .zip(&s.ant)
                .map(|(e, &antecedent)| TrailEntry {
                    lit: from_lit(e.lit),
                    decision: e.decision,
                    antecedent,
                })
                .collect(),
        }
    }
}

#[cfg(feature = "server")]
fn to_step(
    rule: CdclRule,
    lit: i32,
    ci: usize,
) -> Result<backend::cdcl::trace::Step, ServerFnError> {
    use backend::cdcl::trace::{Rule as BRule, Step};

    Ok(match rule {
        CdclRule::Decide if lit == 0 => {
            return Err(ServerFnError::new(
                "lit must be non-zero (1-based signed encoding)",
            ));
        }
        CdclRule::Decide => Step {
            rule: BRule::Decide,
            param: to_lit(lit).0,
        },
        CdclRule::Unit => Step {
            rule: BRule::UnitProp,
            param: ci,
        },
        CdclRule::Conflict => Step {
            rule: BRule::Conflict,
            param: ci,
        },
    })
}

#[cfg(feature = "server")]
fn from_rule(r: backend::cdcl::trace::Rule) -> CdclRule {
    use backend::cdcl::trace::Rule as BRule;

    match r {
        BRule::Decide => CdclRule::Decide,
        BRule::UnitProp => CdclRule::Unit,
        BRule::Conflict => CdclRule::Conflict,
    }
}

#[server]
pub async fn generate_cdcl_exercise(
    steps: Vec<CdclStep>,
    target: Sat,
    vars: usize,
    clauses: usize,
) -> Result<GenerateOutcome, ServerFnError> {
    use std::time::Duration;

    if steps.is_empty() {
        return Err(ServerFnError::new("trace is empty"));
    }
    if vars < 1 || clauses < 1 {
        return Err(ServerFnError::new("vars and clauses must be >= 1"));
    }

    let trace = to_backend_trace(&steps, vars, clauses).map_err(ServerFnError::new)?;
    let sat = matches!(target, Sat::Sat);

    crate::algos::blocking(move || {
        realise(trace, sat, vars, clauses, Duration::from_secs(TIMEOUT_SECS))
    })
    .await
}

#[cfg(feature = "server")]
fn to_backend_trace(
    steps: &[CdclStep],
    vars: usize,
    clauses: usize,
) -> Result<backend::cdcl::trace::Trace, String> {
    use backend::cdcl::trace::Trace;
    use backend::cdcl::trace::{Rule as BRule, Step};

    let mut trace: Trace = Vec::with_capacity(steps.len());
    let mut available = clauses;
    for (i, s) in steps.iter().enumerate() {
        let step = match s.rule {
            CdclRule::Decide => {
                if s.vi < 1 || s.vi > vars {
                    return Err(format!(
                        "step {}: variable {} is outside 1..={vars}",
                        i + 1,
                        s.vi
                    ));
                }
                Step {
                    rule: BRule::Decide,
                    param: 2 * (s.vi - 1) + usize::from(s.neg),
                }
            }
            CdclRule::Unit | CdclRule::Conflict => {
                if s.ci >= available {
                    return Err(format!(
                        "step {}: clause c{} is outside c0..=c{}",
                        i + 1,
                        s.ci,
                        available - 1
                    ));
                }
                let rule = if s.rule == CdclRule::Unit {
                    BRule::UnitProp
                } else {
                    BRule::Conflict
                };
                Step { rule, param: s.ci }
            }
        };
        if s.rule == CdclRule::Conflict {
            available += 1;
        }
        trace.push(step);
    }
    Ok(trace)
}

#[cfg(feature = "server")]
#[allow(clippy::needless_pass_by_value)]
fn realise(
    trace: backend::cdcl::trace::Trace,
    sat: bool,
    vars: usize,
    clauses: usize,
    per_solve: std::time::Duration,
) -> GenerateOutcome {
    use crate::algos::cdcl::state::TrailEntry;
    use crate::algos::model_text::{parse_model_text, parse_trail_text};
    use backend::cdcl::Model;

    let model = Model::from_trace(&trace, vars, clauses).with_ordered_decisions();
    let outcome = model.solve(&trace, sat, Some(per_solve));

    if outcome.unknown {
        return GenerateOutcome::TimedOut;
    }
    let Some(z3_model) = outcome.model else {
        return GenerateOutcome::Impossible;
    };

    let last = model.step_amount() - 1;
    let set = parse_model_text(&model.format_final_model(&z3_model));
    let learned = parse_model_text(&model.format_clauses(&z3_model, last))
        .into_iter()
        .skip(set.len())
        .collect();
    let trail = (0..model.step_amount())
        .map(|t| {
            parse_trail_text(&model.format_trail(&z3_model, t))
                .into_iter()
                .map(|s| TrailEntry {
                    lit: s.lit,
                    decision: s.decision,
                    antecedent: s.antecedent,
                })
                .collect()
        })
        .collect();

    GenerateOutcome::Found(GeneratedExercise {
        set,
        learned,
        trail,
    })
}

#[server]
pub async fn grade_cdcl_step(
    from: CdclState,
    rule: CdclRule,
    lit: i32,
    ci: usize,
    claimed: CdclState,
) -> Result<CdclGradedStep, ServerFnError> {
    use crate::algos::feedback::convert::graded;
    use backend::cdcl::feedback::Cdcl;
    use backend::cdcl::feedback::answer::read_answer;
    use backend::feedback::grade_step;
    use convert::{from_state, to_state, to_trail};

    let step = to_step(rule, lit, ci)?;
    let from = to_state(&from);
    let claimed = read_answer(&from, step, to_phi(&claimed.phi), to_trail(&claimed.trail));
    let v = grade_step(&Cdcl, &from, step, &claimed);
    Ok(graded(&v, &claimed, from_state, from_rule))
}

/// Check the student's whole run and conclusion, for the closing verdict.
#[server]
pub async fn verify_cdcl_trace(
    formula: Vec<Vec<i32>>,
    steps: Vec<(CdclStep, CdclState)>,
    concluded: Sat,
) -> Result<TraceVerdict, ServerFnError> {
    use crate::algos::feedback::convert::{backend_sat, trace_verdict};
    use backend::cdcl::feedback::Cdcl;
    use backend::cdcl::feedback::answer::read_answer;
    use backend::cdcl::state::State;
    use backend::feedback::{forward, verify};
    use convert::to_trail;

    let s0 = State::initial(to_phi(&formula));
    let mut from = s0.clone();
    let mut read = Vec::with_capacity(steps.len());
    for (st, claimed) in &steps {
        let step = to_step(st.rule, st.lit(), st.ci)?;
        let s = read_answer(&from, step, to_phi(&claimed.phi), to_trail(&claimed.trail));
        read.push((step, s.clone()));
        from = s;
    }
    let v = verify(&Cdcl, &s0, &read, backend_sat(concluded));
    Ok(trace_verdict(&v, &forward(&Cdcl, &s0)))
}

#[server]
pub async fn random_cdcl_exercise(
    max_vars: usize,
    max_clauses: usize,
    max_steps: usize,
) -> Result<Option<GeneratedExercise>, ServerFnError> {
    use crate::algos::step_slack;
    use crate::algos::cdcl::trace_generator::Run;
    use backend::cdcl::solve::forward_cdcl;
    use crate::algos::trace_generator::walk;
    use backend::state::Sat as BSat;
    use rand::{Rng, SeedableRng, rngs::StdRng};
    use std::time::{Duration, Instant};

    if max_vars < 1 || max_clauses < 1 || max_steps < 1 {
        return Err(ServerFnError::new(
            "max_vars / max_clauses / max_steps must be >= 1",
        ));
    }

    let vars = max_vars.max(2);
    let clauses = max_clauses.max(2);
    let n_steps = max_steps.max(3);
    let mut rng = StdRng::from_os_rng();
    const BUDGET: Duration = Duration::from_secs(5);
    let deadline = Instant::now() + BUDGET;
    let mut nearest: Option<((bool, usize), GeneratedExercise)> = None;

    for _ in 0..40 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let target = if rng.random() { BSat::Sat } else { BSat::Unsat };
        let Some(trace) = (0..400).find_map(|_| {
            let run = walk(Run::new(vars, clauses), n_steps, target, &mut rng)?;
            (run.learned() > 0).then_some(run.steps)
        }) else {
            continue;
        };

        let per_solve = left.min(Duration::from_secs(2));
        let sat = target == BSat::Sat;
        let found =
            crate::algos::blocking(move || realise(trace, sat, vars, clauses, per_solve)).await?;
        let GenerateOutcome::Found(ex) = found else {
            continue;
        };
        let run = forward_cdcl(&to_phi(&ex.set));
        let conflicts = run_conflicts(&run);
        let off = run.steps.len().abs_diff(n_steps);
        if conflicts && off <= step_slack(n_steps) {
            return Ok(Some(ex));
        }
        let miss = (!conflicts, off);
        if nearest.as_ref().is_none_or(|(m, _)| miss < *m) {
            nearest = Some((miss, ex));
        }
    }
    Ok(nearest.map(|(_, ex)| ex))
}

#[cfg(feature = "server")]
fn run_conflicts(run: &backend::cdcl::solve::ForwardRun) -> bool {
    use backend::cdcl::trace::Rule as BRule;

    run.steps.iter().any(|(s, _)| s.rule == BRule::Conflict)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::algos::cdcl::state::{CdclPracticeState, TrailEntry};
    use crate::algos::feedback::GradeError;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }
    fn dec(l: i32) -> TrailEntry {
        TrailEntry {
            lit: l,
            decision: true,
            antecedent: None,
        }
    }
    fn prop(l: i32, c: usize) -> TrailEntry {
        TrailEntry {
            lit: l,
            decision: false,
            antecedent: Some(c),
        }
    }
    fn xor() -> Vec<Vec<i32>> {
        vec![vec![1, 2], vec![-1, 2], vec![1, -2], vec![-1, -2]]
    }
    fn state(phi: Vec<Vec<i32>>, trail: Vec<TrailEntry>) -> CdclState {
        CdclState { phi, trail }
    }
    fn grade(
        from: CdclState,
        rule: CdclRule,
        lit: i32,
        ci: usize,
        claimed: CdclState,
    ) -> CdclGradedStep {
        rt().block_on(grade_cdcl_step(from, rule, lit, ci, claimed))
            .unwrap()
    }

    #[test]
    fn the_algorithms_own_run_on_the_default_exercise_is_a_perfect_trace() {
        let p = CdclPracticeState::default();
        let mut learned = xor();
        learned.push(vec![-1]);
        let step = |rule, vi, ci| CdclStep {
            rule,
            vi,
            neg: false,
            ci,
        };
        let steps = vec![
            (step(CdclRule::Decide, 1, 0), state(xor(), vec![dec(1)])),
            (
                step(CdclRule::Unit, 0, 1),
                state(xor(), vec![dec(1), prop(2, 1)]),
            ),
            (
                step(CdclRule::Conflict, 0, 3),
                state(learned.clone(), vec![]),
            ),
            (
                step(CdclRule::Unit, 0, 4),
                state(learned.clone(), vec![prop(-1, 4)]),
            ),
            (
                step(CdclRule::Unit, 0, 0),
                state(learned, vec![prop(-1, 4), prop(2, 0)]),
            ),
        ];
        let v = rt()
            .block_on(verify_cdcl_trace(p.formula, steps, Sat::Unsat))
            .unwrap();
        assert_eq!((v.valid_steps, v.steps), (5, 5));
        assert!(v.terminal && v.consistent && v.correct);
        assert_eq!((v.algorithm, v.algorithm_steps), (Sat::Unsat, 5));
    }

    #[test]
    fn propagation_records_the_clause_it_fired_on() {
        let v = grade(
            state(xor(), vec![dec(1)]),
            CdclRule::Unit,
            0,
            1,
            state(xor(), vec![dec(1), prop(2, 7)]),
        );
        assert!(v.valid());
        assert_eq!(
            v.claimed.trail[1].antecedent,
            Some(1),
            "the grader, not the student, sets it"
        );
    }

    #[test]
    fn a_conflict_is_graded_on_the_learned_clause_and_the_backjump() {
        let from = state(xor(), vec![dec(1), prop(2, 1)]);
        let mut learned = xor();
        learned.push(vec![-1]);

        let v = grade(
            from.clone(),
            CdclRule::Conflict,
            0,
            3,
            state(learned.clone(), vec![]),
        );
        assert!(v.valid());

        let mut wrong = xor();
        wrong.push(vec![-1, -2]);
        let v = grade(from, CdclRule::Conflict, 0, 3, state(wrong, vec![]));
        assert_eq!(v.error, Some(GradeError::Misapplied));
        assert_eq!(v.correction().map(|s| &s.phi), Some(&learned));
    }

    #[test]
    fn propagating_past_a_conflict_is_valid_but_flagged() {
        let phi = vec![vec![-1, -2], vec![-1, 3]];
        let from = state(phi.clone(), vec![dec(1), dec(2)]);
        let v = grade(
            from,
            CdclRule::Unit,
            0,
            1,
            state(phi, vec![dec(1), dec(2), prop(3, 1)]),
        );
        assert!(v.valid());
        assert_eq!(v.preempted_by, Some(CdclRule::Conflict));
    }

    #[test]
    fn a_designed_non_chronological_run_is_realised() {
        let step = |rule, vi, ci| CdclStep {
            rule,
            vi,
            neg: false,
            ci,
        };
        let steps = vec![
            step(CdclRule::Decide, 1, 0),
            step(CdclRule::Decide, 2, 0),
            step(CdclRule::Decide, 3, 0),
            step(CdclRule::Unit, 1, 0),
            step(CdclRule::Conflict, 1, 1),
            step(CdclRule::Unit, 1, 4),
            step(CdclRule::Decide, 2, 0),
        ];
        let out = rt()
            .block_on(generate_cdcl_exercise(steps, Sat::Sat, 4, 4))
            .unwrap();
        let GenerateOutcome::Found(ex) = out else {
            panic!("expected a formula, got {out:?}")
        };
        assert_eq!(ex.set.len(), 4);
        assert_eq!(ex.learned.len(), 1);
        assert_eq!(ex.trail.len(), 8);
        assert_eq!(
            ex.trail[5],
            vec![dec(1)],
            "level 2 is skipped by the backjump"
        );
        assert_eq!(ex.trail[6][1].antecedent, Some(4));
    }

    #[test]
    #[ignore]
    fn bench_random_exercise() {
        use std::time::Instant;
        for (vars, clauses, steps) in [(3, 5, 5), (4, 6, 8), (5, 8, 10), (6, 9, 12)] {
            let (mut hits, mut conflicting, mut used) = (0, 0, 0);
            let t = Instant::now();
            for _ in 0..10 {
                if let Some(ex) = rt()
                    .block_on(random_cdcl_exercise(vars, clauses, steps))
                    .unwrap()
                {
                    hits += 1;
                    conflicting += usize::from(run_conflicts(&backend::cdcl::solve::forward_cdcl(
                        &to_phi(&ex.set),
                    )));
                    used += crate::algos::math::vars_in(&ex.set).len();
                }
            }
            println!(
                "CDCL random v{vars}/c{clauses}/s{steps}: {hits}/10 found, {conflicting} with a conflict, ~{:.1} vars, {:?}",
                used as f64 / hits.max(1) as f64,
                t.elapsed()
            );
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod patterns {
    use super::*;
    use crate::algos::cdcl::trace_generator::Run;
    use crate::algos::trace_generator::TraceGenerator;
    use backend::state::Sat as BSat;
    use rand::{SeedableRng, rngs::StdRng};
    use std::time::Duration;

    fn expand(run: Run, name: String, n: usize, d: &mut StdRng, out: &mut Vec<(String, Run)>) {
        if run.len() == n {
            out.push((name, run));
            return;
        }
        for (i, mut group) in run.successors(d).into_iter().enumerate() {
            let options = std::mem::take(&mut group);
            let picks: Vec<Run> = if i == 2 {
                options
            } else {
                options.into_iter().take(1).collect()
            };
            for next in picks.into_iter().filter(|r| r.len() <= n) {
                let tag = if i == 2 {
                    format!("C{}", next.decisions())
                } else {
                    ["U", "D"][i].to_string()
                };
                expand(next, format!("{name}{tag}"), n, d, out);
            }
        }
    }

    #[test]
    #[ignore]
    fn realisable_rule_patterns() {
        let (vars, clauses, n) = (4, 5, 7);
        let mut d = StdRng::seed_from_u64(7);
        let mut runs = Vec::new();
        expand(Run::new(vars, clauses), String::new(), n, &mut d, &mut runs);
        let mut seen = std::collections::BTreeSet::new();
        let mut tally = std::collections::BTreeMap::new();
        for (name, run) in runs {
            if !seen.insert(name.clone()) {
                continue;
            }
            for target in [BSat::Sat, BSat::Unsat] {
                if run.closing(target) != Some(0) {
                    continue;
                }
                let trace = run.steps.clone();
                let out = match realise(
                    trace,
                    target == BSat::Sat,
                    vars,
                    clauses,
                    Duration::from_secs(3),
                ) {
                    GenerateOutcome::Found(_) => "ok",
                    GenerateOutcome::Impossible => "--",
                    GenerateOutcome::TimedOut => "??",
                };
                println!("{name} {target:?} {out}");
                *tally.entry(out).or_insert(0) += 1;
            }
        }
        println!("{tally:?}");
    }
}

#[cfg(all(test, feature = "server"))]
mod walks {
    use super::*;
    use crate::algos::cdcl::trail::preview;
    use crate::algos::feedback::convert::sat;
    use crate::algos::cdcl::trace_generator::Run;
    use backend::cdcl::trace::{Rule as BRule, Step};
    use crate::algos::trace_generator::walk;
    use backend::state::Sat as BSat;
    use rand::{SeedableRng, rngs::StdRng};

    fn designer_steps(trace: &[Step]) -> Vec<CdclStep> {
        trace
            .iter()
            .map(|s| match s.rule {
                BRule::Decide => CdclStep {
                    rule: CdclRule::Decide,
                    vi: s.param / 2 + 1,
                    neg: s.param % 2 == 1,
                    ci: 0,
                },
                BRule::UnitProp => CdclStep {
                    rule: CdclRule::Unit,
                    vi: 1,
                    neg: false,
                    ci: s.param,
                },
                BRule::Conflict => CdclStep {
                    rule: CdclRule::Conflict,
                    vi: 1,
                    neg: false,
                    ci: s.param,
                },
            })
            .collect()
    }

    #[test]
    fn every_walk_passes_the_preview_and_fits_its_target() {
        let mut rng = StdRng::seed_from_u64(0xCDC1);
        for (vars, clauses, len) in [(3, 5, 5), (3, 5, 6), (4, 6, 8), (5, 8, 10), (6, 9, 12)] {
            for target in [BSat::Sat, BSat::Unsat] {
                let mut walked = 0;
                for _ in 0..200 {
                    let Some(run) = walk(Run::new(vars, clauses), len, target, &mut rng) else {
                        continue;
                    };
                    walked += 1;
                    let steps = designer_steps(&run.steps);
                    assert_eq!(steps.len(), len);
                    let p = preview(&steps, vars, clauses, sat(target));
                    assert!(p.steps.iter().all(|s| s.issue.is_none()), "{steps:?}");
                    assert!(!p.ends_on_conflict, "{steps:?}");
                    if target == BSat::Unsat {
                        assert!(!p.ends_with_decision, "{steps:?}");
                    }
                }
                assert!(
                    walked >= 20,
                    "{vars} vars, {clauses} clauses, {len} steps, {target:?}: {walked}/200"
                );
            }
        }
    }
}
