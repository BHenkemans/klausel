use crate::algos::dpll::state::{
    DpllGradedStep, DpllRule, DpllStep, GenerateOutcome, GeneratedExercise, Sat, TrailEntry,
};
use crate::algos::feedback::TraceVerdict;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::algos::lit::{to_lit, to_phi};

#[cfg(feature = "server")]
mod convert {
    use crate::algos::dpll::state::TrailEntry;
    use crate::algos::lit::{from_lit, to_lit};
    use backend::dpll::state::{Trail, TrailEntry as BEntry};

    pub fn to_trail(t: &[TrailEntry]) -> Trail {
        t.iter()
            .map(|e| BEntry {
                lit: to_lit(e.lit),
                decision: e.decision,
            })
            .collect()
    }

    pub fn from_trail(t: &Trail) -> Vec<TrailEntry> {
        t.iter()
            .map(|e| TrailEntry {
                lit: from_lit(e.lit),
                decision: e.decision,
            })
            .collect()
    }
}

#[cfg(feature = "server")]
fn to_step(
    rule: DpllRule,
    lit: i32,
    ci: usize,
) -> Result<backend::dpll::trace::Step, ServerFnError> {
    use backend::dpll::trace::{Rule as BRule, Step};

    Ok(match rule {
        DpllRule::Decide if lit == 0 => {
            return Err(ServerFnError::new(
                "lit must be non-zero (1-based signed encoding)",
            ));
        }
        DpllRule::Decide => Step {
            rule: BRule::Decide,
            param: to_lit(lit).0,
        },
        DpllRule::Unit => Step {
            rule: BRule::UnitProp,
            param: ci,
        },
        DpllRule::Backtrack => Step {
            rule: BRule::Backtrack,
            param: ci,
        },
    })
}

#[cfg(feature = "server")]
fn from_rule(r: backend::dpll::trace::Rule) -> DpllRule {
    use backend::dpll::trace::Rule as BRule;

    match r {
        BRule::Decide => DpllRule::Decide,
        BRule::UnitProp => DpllRule::Unit,
        BRule::Backtrack => DpllRule::Backtrack,
    }
}

/// Z3 budget for one *designed* trace: the user asked for this exact run, so it
/// is worth waiting on. The practice generator draws its own and passes a much
/// shorter cap, since it will simply try another trace.
#[cfg(feature = "server")]
const TIMEOUT_SECS: u64 = 10;

#[server]
pub async fn generate_dpll_exercise(
    steps: Vec<DpllStep>,
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

    // Blocking work. The desktop build shares its executor with the window, so
    // this cannot run inline without freezing the UI.
    crate::algos::blocking(move || {
        realise(trace, sat, vars, clauses, Duration::from_secs(TIMEOUT_SECS))
    })
    .await
}

/// Designer steps to the backend's trace form. The backend indexes literals as
/// `2*var + polarity` over 0-based variables and asserts every parameter is in
/// range, so range-check here rather than hand it a trace it will reject.
#[cfg(feature = "server")]
fn to_backend_trace(
    steps: &[DpllStep],
    vars: usize,
    clauses: usize,
) -> Result<backend::dpll::trace::Trace, String> {
    use backend::dpll::trace::Trace;
    use backend::dpll::trace::{Rule as BRule, Step};

    let mut trace: Trace = Vec::with_capacity(steps.len());
    for (i, s) in steps.iter().enumerate() {
        let step = match s.rule {
            DpllRule::Decide => {
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
            DpllRule::Unit | DpllRule::Backtrack => {
                if s.ci >= clauses {
                    return Err(format!(
                        "step {}: clause c{} is outside c0..=c{}",
                        i + 1,
                        s.ci,
                        clauses - 1
                    ));
                }
                let rule = if s.rule == DpllRule::Unit {
                    BRule::UnitProp
                } else {
                    BRule::Backtrack
                };
                Step { rule, param: s.ci }
            }
        };
        trace.push(step);
    }
    Ok(trace)
}

/// Solve one DPLL trace and read the exercise back out. Shared by the designer
/// and the practice generator, which pass very different `per_solve` caps.
#[cfg(feature = "server")]
#[allow(clippy::needless_pass_by_value)]
fn realise(
    trace: backend::dpll::trace::Trace,
    sat: bool,
    vars: usize,
    clauses: usize,
    per_solve: std::time::Duration,
) -> GenerateOutcome {
    use crate::algos::dpll::state::GeneratedExercise;
    use crate::algos::model_text::{parse_model_text, parse_trail_text};
    use backend::dpll::Model;

    let model = Model::from_trace(&trace, vars, clauses).with_ordered_decisions();
    let outcome = model.solve(&trace, sat, Some(per_solve));

    if outcome.unknown {
        return GenerateOutcome::TimedOut;
    }
    let Some(z3_model) = outcome.model else {
        return GenerateOutcome::Impossible;
    };

    let set = parse_model_text(&model.format_final_model(&z3_model));
    let trail = (0..model.step_amount())
        .map(|t| {
            parse_trail_text(&model.format_trail(&z3_model, t))
                .into_iter()
                .map(|s| TrailEntry {
                    lit: s.lit,
                    decision: s.decision,
                })
                .collect()
        })
        .collect();

    GenerateOutcome::Found(GeneratedExercise { set, trail })
}

/// Grade one student step against the DPLL feedback module.
///
/// `lit` is read by `Decide` (1-based signed); `ci` by `Unit` and `Backtrack`,
/// which name the clause that fires rather than a literal.
#[server]
pub async fn grade_dpll_step(
    formula: Vec<Vec<i32>>,
    from: Vec<TrailEntry>,
    rule: DpllRule,
    lit: i32,
    ci: usize,
    claimed: Vec<TrailEntry>,
) -> Result<DpllGradedStep, ServerFnError> {
    use crate::algos::feedback::convert::graded;
    use backend::dpll::feedback::Dpll;
    use backend::feedback::grade_step;
    use convert::{from_trail, to_trail};

    let step = to_step(rule, lit, ci)?;
    let claimed = to_trail(&claimed);
    let v = grade_step(
        &Dpll {
            phi: to_phi(&formula),
        },
        &to_trail(&from),
        step,
        &claimed,
    );
    Ok(graded(&v, &claimed, from_trail, from_rule))
}

/// Check the student's whole run and conclusion, for the closing verdict.
#[server]
pub async fn verify_dpll_trace(
    formula: Vec<Vec<i32>>,
    steps: Vec<(DpllStep, Vec<TrailEntry>)>,
    concluded: Sat,
) -> Result<TraceVerdict, ServerFnError> {
    use crate::algos::feedback::convert::{backend_sat, trace_verdict};
    use backend::dpll::feedback::Dpll;
    use backend::feedback::{forward, verify};
    use convert::to_trail;

    let a = Dpll {
        phi: to_phi(&formula),
    };
    let steps = steps
        .iter()
        .map(|(st, m)| Ok((to_step(st.rule, st.lit(), st.ci)?, to_trail(m))))
        .collect::<Result<Vec<_>, ServerFnError>>()?;
    let v = verify(&a, &Vec::new(), &steps, backend_sat(concluded));
    Ok(trace_verdict(&v, &forward(&a, &Vec::new())))
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
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
        }
    }
    fn prop(l: i32) -> TrailEntry {
        TrailEntry {
            lit: l,
            decision: false,
        }
    }

    // { α β, ¬α β, α ¬β, ¬α ¬β }
    fn xor() -> Vec<Vec<i32>> {
        vec![vec![1, 2], vec![-1, 2], vec![1, -2], vec![-1, -2]]
    }

    fn grade(
        phi: Vec<Vec<i32>>,
        from: Vec<TrailEntry>,
        rule: DpllRule,
        lit: i32,
        ci: usize,
        claimed: Vec<TrailEntry>,
    ) -> DpllGradedStep {
        rt().block_on(grade_dpll_step(phi, from, rule, lit, ci, claimed))
            .unwrap()
    }

    /// Not part of the suite: `random_dpll_exercise` seeds from the clock, so
    /// asserting it always finds one would be flaky. Run with `--ignored` to
    /// measure the hit rate.

    /// Sizes to measure, as "v,c,s;v,c,s" in BENCH_SIZES.
    fn sweep() -> Vec<(usize, usize, usize)> {
        std::env::var("BENCH_SIZES")
            .unwrap_or_else(|_| "3,5,3".into())
            .split(';')
            .filter_map(|t| {
                let n: Vec<usize> = t.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                (n.len() == 3).then(|| (n[0], n[1], n[2]))
            })
            .collect()
    }

    #[test]
    #[ignore]
    fn bench_random_exercise() {
        use std::time::Instant;
        for (mv, mc, ms) in sweep() {
            let (mut hits, mut total) = (0, std::time::Duration::ZERO);
            let (mut sv, mut sc) = (0usize, 0usize);
            const N: usize = 3;
            for _ in 0..N {
                let t = Instant::now();
                let got = rt().block_on(random_dpll_exercise(mv, mc, ms)).unwrap();
                total += t.elapsed();
                if let Some(ex) = &got {
                    hits += 1;
                    sc += ex.set.len();
                    sv += crate::algos::math::vars_in(&ex.set).len();
                }
            }
            let h = hits.max(1);
            println!(
                "DPLL ask v{mv}/c{mc}/s{ms}: {hits}/{N} | avg {:?} | got ~{:.1} vars, ~{:.1} clauses",
                total / N as u32,
                sv as f64 / h as f64,
                sc as f64 / h as f64
            );
        }
    }

    #[test]
    fn the_algorithms_own_run_is_a_perfect_trace() {
        let step = |rule, vi, ci| DpllStep {
            rule,
            vi,
            neg: false,
            ci,
        };
        let steps = vec![
            (step(DpllRule::Decide, 1, 0), vec![dec(1)]),
            (step(DpllRule::Unit, 0, 1), vec![dec(1), prop(2)]),
            (step(DpllRule::Backtrack, 0, 3), vec![prop(-1)]),
            (step(DpllRule::Unit, 0, 0), vec![prop(-1), prop(2)]),
        ];
        let v = rt()
            .block_on(verify_dpll_trace(xor(), steps, Sat::Unsat))
            .unwrap();
        assert_eq!((v.valid_steps, v.steps), (4, 4));
        assert!(v.terminal && v.consistent && v.correct);
        assert_eq!((v.algorithm, v.algorithm_steps), (Sat::Unsat, 4));
    }

    #[test]
    fn concluding_on_a_trail_that_is_not_terminal_is_early() {
        let steps = vec![(
            DpllStep {
                rule: DpllRule::Decide,
                vi: 1,
                neg: false,
                ci: 0,
            },
            vec![dec(1)],
        )];
        let v = rt()
            .block_on(verify_dpll_trace(xor(), steps, Sat::Unsat))
            .unwrap();
        assert!(!v.terminal && !v.consistent && v.correct);
    }

    #[test]
    fn deciding_on_an_empty_trail_is_valid_and_conforming() {
        let v = grade(xor(), vec![], DpllRule::Decide, 1, 0, vec![dec(1)]);
        assert!(v.valid());
        assert_eq!(v.preempted_by, None);
    }

    // c1 = { ¬α, β }; under [α] only β survives, so it forces β.
    #[test]
    fn unit_propagation_appends_the_survivor() {
        let v = grade(
            xor(),
            vec![dec(1)],
            DpllRule::Unit,
            0,
            1,
            vec![dec(1), prop(2)],
        );
        assert!(v.valid());
    }

    #[test]
    fn propagating_the_wrong_literal_is_misapplied() {
        let v = grade(
            xor(),
            vec![dec(1)],
            DpllRule::Unit,
            0,
            1,
            vec![dec(1), prop(-2)],
        );
        assert_eq!(v.error, Some(GradeError::Misapplied));
        assert_eq!(v.correction(), Some(&vec![dec(1), prop(2)]));
    }

    // [α*, β] conflicts with c3 = { ¬α, ¬β }: cut back past the decision and
    // append its complement as a propagation.
    #[test]
    fn backtrack_cuts_to_the_last_decision_and_flips_it() {
        let v = grade(
            xor(),
            vec![dec(1), prop(2)],
            DpllRule::Backtrack,
            0,
            3,
            vec![prop(-1)],
        );
        assert!(v.valid());
    }

    #[test]
    fn backtracking_without_a_conflict_is_unapplicable() {
        let v = grade(
            xor(),
            vec![dec(1)],
            DpllRule::Backtrack,
            0,
            0,
            vec![prop(-1)],
        );
        assert_eq!(v.error, Some(GradeError::Unapplicable));
        assert_eq!(
            v.preempted_by, None,
            "no priority remark on an illegal step"
        );
    }

    // { ¬α ¬β, ¬α γ } under [α*, β]: c0 conflicts while c1 is unit, so
    // propagating is legal but backtrack outranks it.
    #[test]
    fn propagating_past_a_conflict_is_valid_but_flagged() {
        let phi = vec![vec![-1, -2], vec![-1, 3]];
        let v = grade(
            phi,
            vec![dec(1), prop(2)],
            DpllRule::Unit,
            0,
            1,
            vec![dec(1), prop(2), prop(3)],
        );
        assert!(v.valid(), "a non-priority rule is still a valid step");
        assert_eq!(v.preempted_by, Some(DpllRule::Backtrack));
    }

    // { α, β γ }: c0 is unit, so deciding γ is legal but out of order.
    #[test]
    fn deciding_past_a_unit_clause_is_valid_but_flagged() {
        let phi = vec![vec![1], vec![2, 3]];
        let v = grade(phi, vec![], DpllRule::Decide, 3, 0, vec![dec(3)]);
        assert!(v.valid());
        assert_eq!(v.preempted_by, Some(DpllRule::Unit));
    }
}

#[server]
pub async fn random_dpll_exercise(
    max_vars: usize,
    max_clauses: usize,
    max_steps: usize,
) -> Result<Option<GeneratedExercise>, ServerFnError> {
    use crate::algos::step_slack;
    use crate::algos::dpll::trace_generator::Run;
    use backend::dpll::solve::forward_dpll;
    use backend::dpll::trace::Rule as BRule;
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
    let n_steps = max_steps.max(2);
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
            let backtracks = run.steps.iter().any(|s| s.rule == BRule::Backtrack);
            (backtracks && run.fills_every_variable()).then_some(run.steps)
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
        let run = forward_dpll(&to_phi(&ex.set));
        let complete = run.steps.iter().any(|(s, _)| s.rule == BRule::Backtrack)
            && crate::algos::math::vars_in(&ex.set).len() == vars;
        let off = run.steps.len().abs_diff(n_steps);
        if complete && off <= step_slack(n_steps) {
            return Ok(Some(ex));
        }
        let miss = (!complete, off);
        if nearest.as_ref().is_none_or(|(m, _)| miss < *m) {
            nearest = Some((miss, ex));
        }
    }

    Ok(nearest.map(|(_, ex)| ex))
}

#[cfg(all(test, feature = "server"))]
mod soundness {
    use super::*;
    use backend::dpll::feedback::backtrack::backtrack_effect;
    use backend::dpll::feedback::decide::decide_effect;
    use backend::dpll::feedback::invariants::{find_conflict, has_decision, partial_eval};
    use backend::dpll::feedback::unit::{unit_applicable, unit_effect};
    use convert::to_trail;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }

    /// Every step of a generated exercise must be a step the independent
    /// forward checker also accepts — in particular every backtrack must sit on
    /// a real conflict with a real decision to undo.
    fn audit(set: &[Vec<i32>], trail: &[Vec<TrailEntry>], steps: &[DpllStep]) {
        let phi = to_phi(set);
        for (i, st) in steps.iter().enumerate() {
            let from = to_trail(&trail[i]);
            let to = to_trail(&trail[i + 1]);
            match st.rule {
                DpllRule::Decide => {
                    let l = to_lit(st.lit());
                    assert_eq!(decide_effect(&from, l), to, "step {i}: decide effect");
                }
                DpllRule::Unit => {
                    assert!(
                        unit_applicable(&phi, &from, st.ci),
                        "step {i}: clause c{} is not unit under the trail",
                        st.ci
                    );
                    assert_eq!(
                        unit_effect(&phi, &from, st.ci),
                        Some(to),
                        "step {i}: unit effect"
                    );
                }
                DpllRule::Backtrack => {
                    let conflict = find_conflict(&phi, &from);
                    assert!(
                        conflict.is_some(),
                        "step {i}: backtrack with no clause in conflict"
                    );
                    assert!(
                        has_decision(&from),
                        "step {i}: backtrack with no decision to undo"
                    );
                    assert_eq!(
                        backtrack_effect(&from),
                        Some(to),
                        "step {i}: backtrack effect"
                    );
                }
            }
        }
        // And the formula itself is not degenerate.
        assert!(!set.is_empty(), "empty formula");
        for c in set {
            assert!(!c.is_empty(), "formula contains the empty clause");
            assert!(
                partial_eval(&to_phi(std::slice::from_ref(c))[0], &Vec::new())
                    != backend::dpll::feedback::invariants::PartialEval::True,
                "clause is satisfied by the empty trail"
            );
        }
    }

    /// What does solving a randomly generated exercise actually involve? An
    /// exercise that unit-propagates straight to an answer teaches nothing
    /// about DPLL.
    #[test]
    #[ignore]
    fn random_exercises_are_worth_solving() {
        use backend::dpll::solve::forward_dpll;
        use backend::dpll::trace::Rule as BRule;
        let (mut trivial, mut with_backtrack, mut total_steps, mut n) = (0, 0, 0usize, 0);
        for _ in 0..12 {
            let Some(ex) = rt().block_on(random_dpll_exercise(4, 6, 5)).unwrap() else {
                continue;
            };
            let run = forward_dpll(&to_phi(&ex.set));
            let bt = run
                .steps
                .iter()
                .filter(|(s, _)| s.rule == BRule::Backtrack)
                .count();
            let de = run
                .steps
                .iter()
                .filter(|(s, _)| s.rule == BRule::Decide)
                .count();
            if bt > 0 {
                with_backtrack += 1;
            }
            if de == 0 {
                trivial += 1;
            }
            total_steps += run.steps.len();
            n += 1;
            println!(
                "  {} steps, {de} decides, {bt} backtracks, {:?}",
                run.steps.len(),
                run.result
            );
        }
        println!(
            "SUMMARY {n} exercises: {with_backtrack} need backtracking, {trivial} need no decision at all, avg {} steps",
            total_steps / n.max(1)
        );
    }

    #[test]
    fn a_designed_backtracking_run_is_genuine() {
        // decide, propagate, backtrack, propagate: the shape the default
        // exercise uses, and the one where a bogus backtrack would show up.
        let steps = vec![
            DpllStep {
                rule: DpllRule::Decide,
                vi: 1,
                neg: false,
                ci: 0,
            },
            DpllStep {
                rule: DpllRule::Unit,
                vi: 1,
                neg: false,
                ci: 0,
            },
            DpllStep {
                rule: DpllRule::Backtrack,
                vi: 1,
                neg: false,
                ci: 2,
            },
            DpllStep {
                rule: DpllRule::Unit,
                vi: 1,
                neg: false,
                ci: 1,
            },
        ];
        let out = rt()
            .block_on(generate_dpll_exercise(steps.clone(), Sat::Unsat, 2, 4))
            .unwrap();
        match out {
            GenerateOutcome::Found(ex) => audit(&ex.set, &ex.trail, &steps),
            other => panic!("expected a formula, got {other:?}"),
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod patterns {
    use super::*;
    use crate::algos::dpll::trace_generator::Run;
    use crate::algos::trace_generator::TraceGenerator;
    use backend::state::Sat as BSat;
    use rand::{SeedableRng, rngs::StdRng};
    use std::time::Duration;

    #[test]
    #[ignore]
    fn realisable_rule_patterns() {
        let (vars, clauses, n) = (4, 6, 6);
        let mut d = StdRng::seed_from_u64(7);
        let mut tally = std::collections::BTreeMap::new();
        for code in 0..3usize.pow(n as u32) {
            let mut run = Run::new(vars, clauses);
            let mut name = String::new();
            let mut ok = true;
            for k in 0..n {
                let pick = code / 3usize.pow(k as u32) % 3;
                let mut groups = run.successors(&mut d);
                if groups[pick].is_empty() {
                    ok = false;
                    break;
                }
                run = groups[pick].remove(0);
                name.push(['U', 'D', 'B'][pick]);
            }
            if !ok {
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
    use crate::algos::dpll::trail::preview;
    use crate::algos::feedback::convert::sat;
    use crate::algos::dpll::trace_generator::Run;
    use backend::dpll::trace::{Rule as BRule, Step};
    use crate::algos::trace_generator::walk;
    use backend::state::Sat as BSat;
    use rand::{SeedableRng, rngs::StdRng};

    fn designer_steps(trace: &[Step]) -> Vec<DpllStep> {
        trace
            .iter()
            .map(|s| match s.rule {
                BRule::Decide => DpllStep {
                    rule: DpllRule::Decide,
                    vi: s.param / 2 + 1,
                    neg: s.param % 2 == 1,
                    ci: 0,
                },
                BRule::UnitProp => DpllStep {
                    rule: DpllRule::Unit,
                    vi: 1,
                    neg: false,
                    ci: s.param,
                },
                BRule::Backtrack => DpllStep {
                    rule: DpllRule::Backtrack,
                    vi: 1,
                    neg: false,
                    ci: s.param,
                },
            })
            .collect()
    }

    #[test]
    fn every_walk_passes_the_preview_and_fits_its_target() {
        let mut rng = StdRng::seed_from_u64(0xD911);
        for (vars, clauses, len) in [(2, 4, 4), (3, 4, 4), (4, 6, 6), (5, 7, 8), (9, 9, 12)] {
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
                    assert!(!p.decision_survives, "{steps:?}");
                }
                assert!(
                    walked >= 20,
                    "{vars} vars, {clauses} clauses, {len} steps, {target:?}: {walked}/200"
                );
            }
        }
    }
}
