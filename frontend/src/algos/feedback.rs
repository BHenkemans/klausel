use crate::algos::Sat;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum GradeError {
    Unapplicable,
    Misapplied,
    UnapplicableAndWrong,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GradedStep<S, R> {
    pub claimed: S,
    pub expected: Option<S>,
    pub error: Option<GradeError>,
    pub preempted_by: Option<R>,
    pub finished: Option<Sat>,
}

impl<S, R> GradedStep<S, R> {
    pub fn valid(&self) -> bool {
        self.error.is_none()
    }

    pub fn correction(&self) -> Option<&S> {
        self.expected
            .as_ref()
            .filter(|_| self.error == Some(GradeError::Misapplied))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TraceVerdict {
    pub valid_steps: usize,
    pub steps: usize,
    pub terminal: bool,
    pub consistent: bool,
    pub correct: bool,
    pub algorithm: Sat,
    pub algorithm_steps: usize,
}

pub trait RuleText: Copy {
    fn name(self) -> &'static str;
    fn unapplicable(self) -> &'static str;
    fn misapplied(self) -> &'static str;
}

fn error_text<R: RuleText>(rule: R, e: GradeError, finished: Option<Sat>) -> String {
    match (finished, e) {
        (Some(result), _) => format!(
            "Already finished: no rule applies any more, and the result is {}.",
            result.label()
        ),
        (None, GradeError::Unapplicable) => rule.unapplicable().into(),
        (None, GradeError::Misapplied) => rule.misapplied().into(),
        (None, GradeError::UnapplicableAndWrong) => format!(
            "{} doesn't apply here, and the result doesn't match its effect either.",
            capitalised(rule.name())
        ),
    }
}

fn capitalised(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

pub fn step_feedback<S, R: RuleText>(rule: R, v: &GradedStep<S, R>) -> Element {
    let error = v.error.map(|e| error_text(rule, e, v.finished));
    let order = v.preempted_by.map(|by| {
        format!(
            "Order note: {} was possible here, and it takes precedence over {}.",
            by.name(),
            rule.name()
        )
    });
    rsx! {
        if let Some(msg) = error {
            div { class: "mt-3 p-3 bg-err-bg border border-err-border rounded-lg text-[12.5px] text-err-ink",
                "{msg}"
            }
        }
        if let Some(msg) = order {
            div { class: "mt-3 p-3 bg-warn-bg rounded-lg text-[12.5px] text-warn-ink",
                "{msg}"
            }
        }
    }
}

fn headline(v: &TraceVerdict) -> (&'static str, &'static str) {
    match (v.correct, v.consistent) {
        (true, true) => ("Correct conclusion.", "text-ok-ink"),
        (true, false) => (
            "Right answer, but it doesn't follow from your final state.",
            "text-warn-ink",
        ),
        (false, true) => (
            "Your conclusion follows from your work, but your work contains a mistake.",
            "text-err-ink",
        ),
        (false, false) => ("Concluded incorrectly.", "text-err-ink"),
    }
}

#[component]
pub fn VerdictBanner(
    concluded: Sat,
    verdict: Option<TraceVerdict>,
    on_again: EventHandler<()>,
) -> Element {
    let border = match verdict {
        Some(v) if v.correct && v.consistent => "border-ok-ink/40",
        Some(_) => "border-err-border",
        None => "border-border",
    };
    rsx! {
        div { class: "p-6 bg-surface border-[1.5px] rounded-2xl shadow-sm flex flex-col gap-3 {border}",
            div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase", "Verdict" }
            match verdict {
                None => rsx! {
                    div { class: "text-[17px] font-bold text-ink-2", "Checking…" }
                },
                Some(v) => {
                    let (text, colour) = headline(&v);
                    rsx! {
                        div { class: "text-[17px] font-bold {colour}", "{text}" }
                        if !v.terminal {
                            div { class: "text-[13px] text-ink-2",
                                "You concluded early: a rule still applies to your final state."
                            }
                        }
                        if !v.correct {
                            div { class: "text-[13px] text-ink-2",
                                "Your answer: "
                                span { class: "font-bold", "{concluded.label()}" }
                                ". Algorithm: "
                                span { class: "font-bold", "{v.algorithm.label()}" }
                                "."
                            }
                        }
                        div { class: "text-[13px] text-ink-muted",
                            "{v.valid_steps} / {v.steps} steps valid · algorithm solves in {v.algorithm_steps} steps"
                        }
                    }
                }
            }
            button {
                class: "self-start mt-2 px-3 py-2 rounded-xl border border-border text-ink-muted text-sm hover:text-ink hover:bg-surface-2 cursor-pointer",
                onclick: move |_| on_again.call(()),
                "↻ Solve again"
            }
        }
    }
}

#[cfg(feature = "server")]
pub mod convert {
    use super::{GradeError, GradedStep, TraceVerdict};
    use crate::algos::Sat;
    use backend::feedback::{ErrorKind, ForwardRun, StepVerdict};
    use backend::state::Sat as BSat;

    pub fn sat(s: BSat) -> Sat {
        match s {
            BSat::Sat => Sat::Sat,
            BSat::Unsat => Sat::Unsat,
        }
    }

    pub fn backend_sat(s: Sat) -> BSat {
        match s {
            Sat::Sat => BSat::Sat,
            Sat::Unsat => BSat::Unsat,
        }
    }

    fn grade_error(e: ErrorKind) -> GradeError {
        match e {
            ErrorKind::Unapplicable => GradeError::Unapplicable,
            ErrorKind::Misapplied => GradeError::Misapplied,
            ErrorKind::UnapplicableAndWrong => GradeError::UnapplicableAndWrong,
        }
    }

    pub fn graded<BS, BR, S, R>(
        v: &StepVerdict<BS, BR>,
        claimed: &BS,
        state: impl Fn(&BS) -> S,
        rule: impl Fn(BR) -> R,
    ) -> GradedStep<S, R>
    where
        BR: Copy,
    {
        GradedStep {
            claimed: state(claimed),
            expected: v.expected.as_ref().map(&state),
            error: v.error().map(grade_error),
            preempted_by: v.preempted_by.map(rule),
            finished: v.finished.map(sat),
        }
    }

    pub fn trace_verdict<S, R, T>(
        v: &backend::feedback::TraceVerdict<S, R>,
        run: &ForwardRun<S, T>,
    ) -> TraceVerdict {
        let (valid_steps, steps) = v.score();
        TraceVerdict {
            valid_steps,
            steps,
            terminal: v.terminal,
            consistent: v.consistent,
            correct: v.correct,
            algorithm: sat(run.result),
            algorithm_steps: run.steps.len(),
        }
    }
}
