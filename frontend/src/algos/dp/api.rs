use crate::algos::dp::state::{GeneratedFormula, GradedStep, Rule, Sat, TraceStep};
use crate::algos::feedback::TraceVerdict;
#[cfg(feature = "server")]
use crate::algos::model_text::parse_model_text;
use dioxus::prelude::*;

#[cfg(feature = "server")]
fn to_backend_trace(steps: &[TraceStep]) -> backend::davis_putnam::trace::Trace {
    use backend::davis_putnam::trace::{Rule as DpRule, Step};

    steps
        .iter()
        .map(|s| {
            let var = s.vi.saturating_sub(1);
            match s.rule {
                Rule::Unit => Step {
                    rule: DpRule::Unit,
                    param: 2 * var + usize::from(s.neg),
                },
                Rule::Pure => Step {
                    rule: DpRule::Pure,
                    param: 2 * var + usize::from(s.neg),
                },
                Rule::Elim => Step {
                    rule: DpRule::Elim,
                    param: var,
                },
            }
        })
        .collect()
}

#[cfg(feature = "server")]
fn to_step(rule: Rule, lit: i32) -> Result<backend::davis_putnam::trace::Step, ServerFnError> {
    use crate::algos::lit::to_lit;
    use backend::davis_putnam::trace::{Rule as DpRule, Step};

    if lit == 0 {
        return Err(ServerFnError::new(
            "lit must be non-zero (1-based signed encoding)",
        ));
    }
    let l = to_lit(lit);
    Ok(match rule {
        Rule::Unit => Step {
            rule: DpRule::Unit,
            param: l.0,
        },
        Rule::Pure => Step {
            rule: DpRule::Pure,
            param: l.0,
        },
        Rule::Elim => Step {
            rule: DpRule::Elim,
            param: l.atom(),
        },
    })
}

#[cfg(feature = "server")]
fn from_rule(r: backend::davis_putnam::trace::Rule) -> Rule {
    use backend::davis_putnam::trace::Rule as DpRule;

    match r {
        DpRule::Unit => Rule::Unit,
        DpRule::Pure => Rule::Pure,
        DpRule::Elim => Rule::Elim,
    }
}

#[server]
pub async fn generate_formula(
    steps: Vec<TraceStep>,
    target: Sat,
    max_vars: usize,
    max_clauses: usize,
) -> Result<Option<GeneratedFormula>, ServerFnError> {
    use backend::davis_putnam::Model;
    use std::time::Duration;

    if steps.is_empty() {
        return Err(ServerFnError::new("trace is empty"));
    }

    let trace = to_backend_trace(&steps);

    let sat = matches!(target, Sat::Sat);

    // Z3 blocks for up to the timeout below. The desktop build runs this
    // function on the executor that also drives the window, so the solve has to
    // happen on a blocking thread or the UI freezes for the whole search.
    let set = crate::algos::blocking(move || {
        let model = Model::from_trace_with_vars(&trace, max_clauses, max_vars);
        let outcome = model.solve(&trace, sat, Some(Duration::from_secs(5)));

        if !outcome.sat {
            return None;
        }
        let z3_model = outcome.model?;
        Some(parse_model_text(&model.format_final_model(&z3_model)))
    })
    .await?;

    Ok(set.map(|set| GeneratedFormula { set }))
}

#[server]
pub async fn grade_step(
    from: Vec<Vec<i32>>,
    rule: Rule,
    lit: i32,
    claimed: Vec<Vec<i32>>,
) -> Result<GradedStep, ServerFnError> {
    use crate::algos::feedback::convert::graded;
    use crate::algos::lit::{from_phi, to_phi};
    use backend::davis_putnam::feedback::Dp;
    use backend::feedback::grade_step as backend_grade;
    use backend::state::{canonical_set, norm_set};

    let step = to_step(rule, lit)?;
    let written = norm_set(to_phi(&claimed));
    let v = backend_grade(
        &Dp,
        &canonical_set(to_phi(&from)),
        step,
        &canonical_set(written.clone()),
    );
    Ok(graded(&v, &written, |s| from_phi(s), from_rule))
}

#[server]
pub async fn verify_trace(
    exercise: Vec<Vec<i32>>,
    steps: Vec<(Rule, i32, Vec<Vec<i32>>)>,
    concluded: Sat,
) -> Result<TraceVerdict, ServerFnError> {
    use crate::algos::feedback::convert::{backend_sat, trace_verdict};
    use crate::algos::lit::to_phi;
    use backend::davis_putnam::feedback::Dp;
    use backend::feedback::{forward, verify};
    use backend::state::canonical_set;

    let s0 = canonical_set(to_phi(&exercise));
    let steps = steps
        .iter()
        .map(|(rule, lit, s)| Ok((to_step(*rule, *lit)?, canonical_set(to_phi(s)))))
        .collect::<Result<Vec<_>, ServerFnError>>()?;
    let v = verify(&Dp, &s0, &steps, backend_sat(concluded));
    Ok(trace_verdict(&v, &forward(&Dp, &s0)))
}

#[server]
pub async fn random_exercise(
    max_vars: usize,
    max_clauses: usize,
    max_steps: usize,
) -> Result<Option<Vec<Vec<i32>>>, ServerFnError> {
    use crate::algos::lit::to_phi;
    use crate::algos::step_slack;
    use backend::davis_putnam::Model;
    use crate::algos::dp::trace_generator::Run;
    use backend::davis_putnam::solve::forward_dp;
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
    let n_steps = max_steps.max(2).min(vars);

    crate::algos::blocking(move || {
        let mut rng = StdRng::from_os_rng();
        const BUDGET: Duration = Duration::from_secs(5);
        const PER_SOLVE: Duration = Duration::from_millis(500);
        let deadline = Instant::now() + BUDGET;
        let mut nearest: Option<(usize, Vec<Vec<i32>>)> = None;

        for _ in 0..64 {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            let target = if rng.random() { BSat::Sat } else { BSat::Unsat };
            let Some(run) = (0..400).find_map(|_| walk(Run::new(vars), n_steps, target, &mut rng))
            else {
                continue;
            };
            let trace = run.steps;
            let model = Model::from_trace_with_vars(&trace, clauses, vars);
            let outcome = model.solve(&trace, target == BSat::Sat, Some(left.min(PER_SOLVE)));
            let Some(z3_model) = outcome.model.filter(|_| outcome.sat) else {
                continue;
            };
            let set = parse_model_text(&model.format_final_model(&z3_model));
            if set.is_empty() {
                continue;
            }
            let off = forward_dp(&to_phi(&set)).steps.len().abs_diff(n_steps);
            if off <= step_slack(n_steps) {
                return Some(set);
            }
            if nearest.as_ref().is_none_or(|(o, _)| off < *o) {
                nearest = Some((off, set));
            }
        }

        nearest.map(|(_, set)| set)
    })
    .await
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

    // { a b, !a b, a !b, !a !b } in the UI's 1-based signed encoding.
    fn xor() -> Vec<Vec<i32>> {
        vec![vec![1, 2], vec![-1, 2], vec![1, -2], vec![-1, -2]]
    }

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
                let got = rt().block_on(random_exercise(mv, mc, ms)).unwrap();
                total += t.elapsed();
                if let Some(set) = &got {
                    hits += 1;
                    sc += set.len();
                    sv += crate::algos::math::vars_in(set).len();
                }
            }
            let h = hits.max(1);
            println!(
                "DP  ask v{mv}/c{mc}/s{ms}: {hits}/{N} | avg {:?} | got ~{:.1} vars, ~{:.1} clauses",
                total / N as u32,
                sv as f64 / h as f64,
                sc as f64 / h as f64
            );
        }
    }

    #[test]
    fn the_algorithms_own_run_is_a_perfect_trace() {
        let steps = vec![
            (Rule::Elim, 1, vec![vec![2], vec![-2]]),
            (Rule::Unit, 2, vec![vec![]]),
        ];
        let v = rt()
            .block_on(verify_trace(xor(), steps, Sat::Unsat))
            .unwrap();
        assert_eq!((v.valid_steps, v.steps), (2, 2));
        assert!(v.terminal && v.consistent && v.correct);
        assert_eq!((v.algorithm, v.algorithm_steps), (Sat::Unsat, 2));
    }

    #[test]
    fn concluding_from_a_wrong_state_is_consistent_but_not_correct() {
        let steps = vec![(Rule::Elim, 1, vec![])];
        let v = rt().block_on(verify_trace(xor(), steps, Sat::Sat)).unwrap();
        assert_eq!(v.valid_steps, 0);
        assert!(v.terminal && v.consistent && !v.correct);
    }

    #[test]
    fn correct_elim_is_valid() {
        let v = rt()
            .block_on(grade_step(xor(), Rule::Elim, 1, vec![vec![2], vec![-2]]))
            .unwrap();
        assert!(v.valid());
        assert_eq!(v.preempted_by, None);
    }

    // Clause sets are sets: the order the student writes them in is irrelevant.
    #[test]
    fn clause_order_does_not_matter() {
        let v = rt()
            .block_on(grade_step(xor(), Rule::Elim, 1, vec![vec![-2], vec![2]]))
            .unwrap();
        assert!(v.valid());
    }

    // Student writes { b !a } where the mechanical result is { !a b }.
    #[test]
    fn literal_order_within_a_clause_does_not_matter() {
        let from = vec![vec![1], vec![-1, 2]];
        let v = rt()
            .block_on(grade_step(from, Rule::Unit, 1, vec![vec![2]]))
            .unwrap();
        assert!(v.valid());
    }

    #[test]
    fn wrong_state_is_misapplied() {
        let v = rt()
            .block_on(grade_step(xor(), Rule::Elim, 1, vec![vec![2]]))
            .unwrap();
        assert_eq!(v.error, Some(GradeError::Misapplied));
        assert_eq!(v.correction(), Some(&vec![vec![2], vec![-2]]));
    }

    // { a, b }: b is pure, but a is a unit clause and Unit has priority.
    #[test]
    fn out_of_order_pure_is_valid_but_flagged() {
        let from = vec![vec![1], vec![2]];
        let v = rt()
            .block_on(grade_step(from, Rule::Pure, 2, vec![vec![1]]))
            .unwrap();
        assert!(v.valid());
        assert_eq!(v.preempted_by, Some(Rule::Unit));
    }
}

#[cfg(all(test, feature = "server"))]
mod patterns {
    use super::*;
    use backend::davis_putnam::Model;
    use std::time::Duration;

    #[test]
    #[ignore]
    fn realisable_rule_patterns() {
        for (vars, clauses, n) in [(3, 5, 3), (4, 6, 4), (5, 7, 4)] {
            println!("== v{vars} c{clauses} s{n}");
            for code in 0..3usize.pow(n as u32) {
                let rules: Vec<Rule> = (0..n)
                    .map(|k| [Rule::Unit, Rule::Pure, Rule::Elim][code / 3usize.pow(k as u32) % 3])
                    .collect();
                let steps: Vec<TraceStep> = rules
                    .iter()
                    .enumerate()
                    .map(|(k, &rule)| TraceStep {
                        rule,
                        vi: k + 1,
                        neg: false,
                    })
                    .collect();
                let trace = to_backend_trace(&steps);
                let name: String = rules
                    .iter()
                    .map(|r| match r {
                        Rule::Unit => 'U',
                        Rule::Pure => 'P',
                        Rule::Elim => 'E',
                    })
                    .collect();
                let mut row = String::new();
                for sat in [true, false] {
                    let model = Model::from_trace_with_vars(&trace, clauses, vars);
                    let o = model.solve(&trace, sat, Some(Duration::from_secs(5)));
                    row.push_str(if o.sat {
                        " ok  "
                    } else if o.unknown {
                        " ??  "
                    } else {
                        " --  "
                    });
                }
                println!("{name} {row}");
            }
        }
    }
}
