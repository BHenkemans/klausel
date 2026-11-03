use crate::algos::controls::Stepper;
use crate::algos::dpll::api::generate_dpll_exercise;
use crate::algos::dpll::state::{
    DpllGenerateState, DpllRule, DpllStep, GenerateOutcome, GeneratedExercise, Sat, TrailEntry,
};
use crate::algos::dpll::trail::{StepIssue, preview};
use crate::algos::dpll::view::TrailChips;
use crate::algos::math::{clause_rsx, cnf_rsx, lit_name, var_count, var_name};
use crate::algos::paste_box::TracePasteBox;
use dioxus::prelude::*;

#[component]
pub fn DpllGenerate() -> Element {
    rsx! {
        div { class: "grid grid-cols-[400px_minmax(0,1fr)] flex-1 items-start",
            TraceDesigner {}
            ResultPanel {}
        }
    }
}

// ---------------------------------------------------------------- designer

#[component]
fn TraceDesigner() -> Element {
    let mut state = use_context::<Signal<DpllGenerateState>>();
    let s = state.read();
    let target = s.target;
    let steps = s.steps.clone();
    let vars = s.vars;
    let clauses = s.clauses;
    let is_loading = s.is_loading;
    drop(s);

    let pre = preview(&steps, vars, clauses, target);
    let blocker = if steps.is_empty() {
        Some(
            "Add at least one step — an empty run neither satisfies nor refutes a formula."
                .to_string(),
        )
    } else {
        pre.blocker()
    };
    let can_generate = blocker.is_none() && !is_loading;

    rsx! {
        div { class: "border-r border-border bg-panel-bg px-6 py-7",
            div { class: "text-[11px] font-bold tracking-[0.08em] text-ink-faint uppercase",
                "Solution trace"
            }
            div { class: "text-[12.5px] text-ink-muted mt-1.5 leading-[1.5]",
                "The DPLL transitions a student should perform, in order. The studio builds a formula whose run is exactly this sequence."
            }

            TracePasteBox {
                apply: move |text: String| state.write().apply_trace(&text),
                placeholder: "decide 0\nunit 1\ndecide 2\nbacktrack 3\nsat",
                hint: "decide <lit> / unit <clause> / backtrack <clause>, one per line",
            }

            div { class: "mt-5 text-[12.5px] font-semibold text-ink-2 mb-2", "Target result" }
            div { class: "flex gap-1.5 bg-surface-2 p-1 rounded-[11px]",
                TargetButton { value: Sat::Sat, label: "Satisfiable", symbol: "∅", current: target }
                TargetButton { value: Sat::Unsat, label: "Unsatisfiable", symbol: "□", current: target }
            }

            div { class: "mt-5 flex flex-col gap-2.5",
                for (i, step) in steps.iter().enumerate() {
                    StepRow {
                        idx: i,
                        step: *step,
                        vars,
                        clauses,
                        issue: pre.steps.get(i).and_then(|p| p.issue),
                        depth: pre.steps.get(i).map(|p| p.depth).unwrap_or(0),
                    }
                }
                // One button per rule: adding a backtrack is a single click,
                // not an add followed by two rule changes.
                div { class: "flex items-center gap-1.5",
                    for (rule , label) in [
                        (DpllRule::Decide, "decide"),
                        (DpllRule::Unit, "unit"),
                        (DpllRule::Backtrack, "backtrack"),
                    ] {
                        button {
                            class: "flex-1 py-2.5 border-[1.5px] border-dashed border-border rounded-[11px] text-[12px] font-bold cursor-pointer hover:bg-surface {rule_chip_classes(rule)}",
                            onclick: move |_| state.write().push_rule(rule),
                            "＋ {label}"
                        }
                    }
                    if !steps.is_empty() {
                        button {
                            class: "px-2.5 py-2.5 rounded-[11px] text-[12px] font-semibold text-ink-faint cursor-pointer hover:text-err-ink hover:bg-err-bg",
                            title: "remove every step",
                            onclick: move |_| state.write().clear_steps(),
                            "Clear"
                        }
                    }
                }
            }

            div { class: "mt-6 text-[11px] font-bold tracking-[0.08em] text-ink-faint uppercase",
                "Constraints"
            }
            // Both bounds are exact, and neither can be read off the trace: a
            // unit step names a clause, never the literal it derives, so the
            // teacher has to say how wide the formula is.
            div { class: "text-[12.5px] text-ink-muted mt-1.5 leading-[1.5]",
                "The formula gets exactly this many distinct, non-empty clauses. The trail holds one literal per variable."
            }
            div { class: "mt-3 flex flex-col gap-2.5",
                Stepper {
                    label: "Variables",
                    value: vars,
                    on_dec: move |_| state.write().dec_vars(),
                    on_inc: move |_| state.write().inc_vars(),
                }
                Stepper {
                    label: "Clauses",
                    value: clauses,
                    on_dec: move |_| state.write().dec_clauses(),
                    on_inc: move |_| state.write().inc_clauses(),
                }
                TrailGauge { peak: pre.peak_depth, vars }
            }

            if let Some(reason) = blocker.clone() {
                div { class: "mt-5 px-3.5 py-3 rounded-xl bg-err-bg border border-err-border text-err-ink text-[12.5px] leading-[1.5]",
                    "{reason}"
                }
            }

            button {
                class: "mt-5 w-full py-3.5 rounded-xl bg-accent text-white text-sm font-bold flex items-center justify-center gap-2 shadow-md hover:shadow-lg cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none",
                disabled: !can_generate,
                onclick: move |_| {
                    let cur = state.read();
                    if cur.is_loading {
                        return;
                    }
                    let steps = cur.steps.clone();
                    let target = cur.target;
                    let vars = cur.vars;
                    let clauses = cur.clauses;
                    drop(cur);
                    state.write().is_loading = true;
                    spawn(async move {
                        let result = match generate_dpll_exercise(steps, target, vars, clauses).await {
                            Err(e) => Err(e.to_string()),
                            Ok(GenerateOutcome::Found(f)) => Ok(f),
                            Ok(GenerateOutcome::Impossible) => {
                                Err(format!(
                                    "No formula over {vars} variables and {clauses} clauses realises this trace",
                                ))
                            }
                            Ok(GenerateOutcome::TimedOut) => {
                                Err("The search ran out of time before deciding".to_string())
                            }
                        };
                        let mut w = state.write();
                        w.latest_result = Some(result);
                        w.is_loading = false;
                    });
                },
                if is_loading {
                    "Generating…"
                } else {
                    "Generate "
                    span { class: "text-[13px]", "→" }
                }
            }
        }
    }
}

#[component]
fn TargetButton(value: Sat, label: &'static str, symbol: &'static str, current: Sat) -> Element {
    let mut state = use_context::<Signal<DpllGenerateState>>();
    let active = value == current;
    rsx! {
        button {
            class: if active {
                "flex-1 px-3 py-2 rounded-lg text-[12.5px] font-medium bg-surface text-ink shadow-sm flex items-center justify-center gap-1.5"
            } else {
                "flex-1 px-3 py-2 rounded-lg text-[12.5px] text-ink-muted hover:text-ink flex items-center justify-center gap-1.5"
            },
            onclick: move |_| state.write().set_target(value),
            "{label}"
            span { class: "font-serif text-[15px]", "{symbol}" }
        }
    }
}

#[component]
fn StepRow(
    idx: usize,
    step: DpllStep,
    vars: usize,
    clauses: usize,
    issue: Option<StepIssue>,
    depth: usize,
) -> Element {
    let mut state = use_context::<Signal<DpllGenerateState>>();
    let num = idx + 1;

    rsx! {
        div {
            class: if issue.is_some() {
                "px-3 py-2.5 bg-surface border border-err-border rounded-[13px] shadow-sm"
            } else {
                "px-3 py-2.5 bg-surface border border-border rounded-[13px] shadow-sm"
            },
            // Rule: a three-way pick, so any rule is one click from any other.
            div { class: "flex items-center gap-2",
                div { class: "w-6 h-6 rounded-full bg-surface-2 flex items-center justify-center text-xs font-bold text-ink-2 shrink-0",
                    "{num}"
                }
                div { class: "flex gap-1",
                    for rule in [DpllRule::Decide, DpllRule::Unit, DpllRule::Backtrack] {
                        button {
                            class: if rule == step.rule {
                                format!("px-2.5 py-1 rounded-lg text-[11.5px] font-bold cursor-pointer {}", rule_chip_classes(rule))
                            } else {
                                "px-2.5 py-1 rounded-lg text-[11.5px] font-semibold text-ink-faint cursor-pointer hover:text-ink hover:bg-surface-2".to_string()
                            },
                            title: rule_verb(rule),
                            onclick: move |_| state.write().set_rule(idx, rule),
                            "{rule_label(rule)}"
                        }
                    }
                }
                div { class: "ml-auto flex items-center gap-0.5 shrink-0",
                    button {
                        class: "w-6 h-6 rounded-md flex items-center justify-center text-ink-faint cursor-pointer hover:bg-surface-2 hover:text-ink text-[14px]",
                        title: "insert a copy below",
                        onclick: move |_| state.write().insert_after(idx),
                        "＋"
                    }
                    button {
                        class: "w-6 h-6 rounded-md flex items-center justify-center text-ink-faint cursor-pointer hover:bg-err-bg hover:text-err-ink text-[15px]",
                        title: "remove step",
                        onclick: move |_| state.write().remove_step(idx),
                        "×"
                    }
                }
            }
            // Parameter: every choice visible, one click each.
            match step.rule {
                DpllRule::Decide => rsx! {
                    div { class: "flex items-center gap-1 mt-2 flex-wrap",
                        button {
                            class: if step.neg {
                                "px-2 py-0.5 rounded-md bg-accent text-white text-[14px] cursor-pointer font-serif"
                            } else {
                                "px-2 py-0.5 rounded-md border border-border text-ink-faint text-[14px] cursor-pointer hover:text-ink font-serif"
                            },
                            title: "toggle negation",
                            onclick: move |_| state.write().toggle_neg(idx),
                            "¬"
                        }
                        for vi in 1..=vars {
                            button {
                                class: if vi == step.vi {
                                    "w-7 py-0.5 rounded-md bg-surface-2 border border-border font-serif text-[16px] italic text-ink cursor-pointer text-center"
                                } else {
                                    "w-7 py-0.5 rounded-md font-serif text-[16px] italic text-ink-faint cursor-pointer hover:bg-surface-2 hover:text-ink text-center"
                                },
                                onclick: move |_| state.write().set_var(idx, vi),
                                "{var_name(vi)}"
                            }
                        }
                    }
                },
                DpllRule::Unit | DpllRule::Backtrack => rsx! {
                    div { class: "flex items-center gap-1 mt-2 flex-wrap",
                        for ci in 0..clauses {
                            button {
                                class: if ci == step.ci {
                                    "px-1.5 py-0.5 rounded-md bg-unit-bg border border-border font-mono text-[11.5px] text-unit-ink cursor-pointer"
                                } else {
                                    "px-1.5 py-0.5 rounded-md font-mono text-[11.5px] text-ink-faint cursor-pointer hover:bg-surface-2 hover:text-ink"
                                },
                                onclick: move |_| state.write().set_clause(idx, ci),
                                "c{ci}"
                            }
                        }
                    }
                },
            }
            match issue {
                Some(i) => rsx! {
                    div { class: "mt-2 pt-2 border-t border-border-soft text-[11.5px] text-err-ink leading-[1.45]",
                        "{i.message()}"
                    }
                },
                None => rsx! {
                    div { class: "mt-2 pt-2 border-t border-border-soft text-[11.5px] text-ink-faint",
                        "{rule_verb(step.rule)} · trail depth {depth}"
                    }
                },
            }
        }
    }
}

/// How many of the variable-indexed trail slots the trace actually needs.
#[component]
fn TrailGauge(peak: usize, vars: usize) -> Element {
    let tight = peak == vars && vars > 0;
    rsx! {
        div { class: "flex items-center justify-between pt-1",
            span { class: "text-[13px] text-ink-2", "Deepest trail" }
            span {
                class: if tight {
                    "text-[12.5px] font-bold text-warn-ink px-2 py-0.5 rounded-md bg-warn-bg"
                } else {
                    "text-[12.5px] font-bold text-ink-muted px-2 py-0.5"
                },
                "{peak} / {vars}"
            }
        }
    }
}

// ------------------------------------------------------------------ result

#[component]
fn ResultPanel() -> Element {
    let state = use_context::<Signal<DpllGenerateState>>();
    let snap = state.read();
    let is_loading = snap.is_loading;
    let result = snap.latest_result.clone();
    let steps = snap.steps.clone();
    let target = snap.target;
    drop(snap);

    rsx! {
        div { class: "p-7 flex flex-col gap-5 min-w-0",
            div { class: "text-[11px] font-bold tracking-[0.08em] text-ink-faint uppercase",
                "Generated exercise"
            }
            match (is_loading, result) {
                (true, _) => rsx! { LoadingCard {} },
                (false, Some(Ok(f))) => rsx! { ExerciseCard { exercise: f, steps, target } },
                (false, Some(Err(msg))) => rsx! { EmptyResultCard { msg } },
                (false, None) => rsx! { IdleCard {} },
            }
        }
    }
}

#[component]
fn IdleCard() -> Element {
    rsx! {
        div { class: "p-12 bg-surface border border-border rounded-2xl text-center",
            div { class: "font-serif text-[30px] text-accent mb-3", "α* ¬β" }
            div { class: "text-sm font-bold text-ink-2", "Design a trace, then press Generate" }
            div { class: "text-[13px] text-ink-muted mt-1.5 leading-[1.5] max-w-[420px] mx-auto",
                "The studio searches for a CNF formula on which DPLL decides, propagates and backtracks in exactly your order — and reports the trail it builds along the way."
            }
        }
    }
}

#[component]
fn LoadingCard() -> Element {
    rsx! {
        div { class: "p-12 bg-surface border border-border rounded-2xl text-center",
            div { class: "font-serif text-[28px] text-ink-faint mb-3", "…" }
            div { class: "text-sm font-bold text-ink-2", "Searching for a formula" }
            div { class: "text-[13px] text-ink-muted mt-1.5 leading-[1.5]",
                "Z3 is fitting clauses to your trail."
            }
        }
    }
}

#[component]
fn EmptyResultCard(msg: String) -> Element {
    rsx! {
        div { class: "p-10 bg-surface border-[1.5px] border-dashed border-border rounded-2xl text-center",
            div { class: "font-serif text-[34px] text-ink-faint mb-2.5", "□  ∅" }
            div { class: "text-sm font-bold text-ink-2", "{msg}" }
            div { class: "text-[13px] text-ink-muted mt-1.5 leading-[1.5] max-w-[430px] mx-auto",
                "Raising the variable bound is the usual fix — a trace often needs more variables than it names, because the literals unit propagation derives take up trail slots of their own. Failing that, the clause a unit step names has to be the one left with a single unassigned literal at that point."
            }
        }
    }
}

#[component]
fn ExerciseCard(exercise: GeneratedExercise, steps: Vec<DpllStep>, target: Sat) -> Element {
    let n_clauses = exercise.set.len();
    let n_vars = var_count(&exercise.set);
    let cnf_el = cnf_rsx(&exercise.set);

    rsx! {
        div { class: "p-6 bg-surface border border-border rounded-2xl shadow-sm min-w-0",
            // Clauses are numbered, not just listed: a unit step addresses one
            // by index, so c0..cK-1 are part of the exercise's meaning.
            div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase mb-3",
                "Clause set"
            }
            div { class: "flex flex-col gap-1.5",
                for (i, clause) in exercise.set.iter().enumerate() {
                    ClauseRow { idx: i, clause: clause.clone(), steps: steps.clone() }
                }
            }
            div { class: "h-px bg-border-soft my-4" }
            div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase mb-2.5",
                "Equivalent CNF"
            }
            div { class: "font-serif text-[20px] text-ink-2 leading-[1.45] break-words [overflow-wrap:anywhere]",
                em { "φ" }
                " = "
                {cnf_el}
            }
            div { class: "h-px bg-border-soft my-4" }
            TrailTable { trail: exercise.trail.clone(), steps }
            div { class: "flex items-center gap-3 mt-5 flex-wrap",
                span { class: "inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-accent-soft text-accent text-[12.5px] font-bold",
                    "✓ Realises the trace"
                }
                // The verdict is set in the designer, but the card is what gets
                // read once you have scrolled away from it.
                span { class: "inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-surface-2 text-ink-2 text-[12.5px] font-bold",
                    match target {
                        Sat::Sat => rsx! { "ends satisfiable " span { class: "font-serif", "∅" } },
                        Sat::Unsat => rsx! { "ends unsatisfiable " span { class: "font-serif", "□" } },
                    }
                }
                span { class: "text-[12.5px] text-ink-muted",
                    "{n_clauses} clauses · {n_vars} vars"
                }
            }
        }
    }
}

#[component]
fn ClauseRow(idx: usize, clause: Vec<i32>, steps: Vec<DpllStep>) -> Element {
    // Which steps propagate on this clause — the link between the trace on the
    // left and the formula on the right.
    let firing: Vec<usize> = steps
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s.rule, DpllRule::Unit) && s.ci == idx)
        .map(|(i, _)| i + 1)
        .collect();
    let body = clause_rsx(&clause);

    rsx! {
        div {
            class: if firing.is_empty() {
                "flex items-baseline gap-3 px-3 py-2 rounded-lg"
            } else {
                "flex items-baseline gap-3 px-3 py-2 rounded-lg bg-unit-bg/40"
            },
            span { class: "font-mono text-[12px] text-ink-faint w-7 shrink-0", "c{idx}" }
            span { class: "font-serif text-[20px] text-ink leading-[1.4] break-words [overflow-wrap:anywhere]",
                {body}
            }
            if !firing.is_empty() {
                span { class: "ml-auto shrink-0 px-2 py-0.5 rounded-md bg-unit-bg text-unit-ink text-[11px] font-bold",
                    if firing.len() == 1 { "unit at step {firing[0]}" } else { "unit at steps {join(&firing)}" }
                }
            }
        }
    }
}

/// The trail after every state of the run. This is the part of a DPLL exercise
/// a student actually writes down, so the generator shows its own answer.
#[component]
fn TrailTable(trail: Vec<Vec<TrailEntry>>, steps: Vec<DpllStep>) -> Element {
    rsx! {
        div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase mb-2.5",
            "Trail"
        }
        div { class: "flex flex-col gap-1",
            for (t, slots) in trail.into_iter().enumerate() {
                div { class: "flex items-baseline gap-3 px-3 py-1.5 rounded-lg odd:bg-surface-2/50",
                    span { class: "font-mono text-[12px] text-ink-faint w-7 shrink-0", "t{t}" }
                    span { class: "text-[11.5px] text-ink-muted w-[110px] shrink-0",
                        // State t is what the t-th step produced; t0 is the start.
                        match t.checked_sub(1).and_then(|i| steps.get(i)) {
                            Some(s) => rsx! { "{step_caption(s)}" },
                            None => rsx! { span { class: "italic", "start" } },
                        }
                    }
                    TrailChips { trail: slots }
                }
            }
        }
        div { class: "flex items-center gap-3 mt-2.5 px-3 text-[11.5px] text-ink-faint",
            span { class: "px-1.5 py-0.5 rounded bg-accent text-white font-serif", "α" }
            "decision"
            span { class: "px-1.5 py-0.5 rounded border border-border font-serif", "α" }
            "propagated"
        }
    }
}

// ----- helpers -----

fn join(nums: &[usize]) -> String {
    nums.iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn step_caption(s: &DpllStep) -> String {
    match s.rule {
        DpllRule::Decide => format!("decide {}", lit_name(s.lit())),
        DpllRule::Unit => format!("unit on c{}", s.ci),
        DpllRule::Backtrack => format!("backtrack on c{}", s.ci),
    }
}

fn rule_label(r: DpllRule) -> &'static str {
    match r {
        DpllRule::Decide => "decide",
        DpllRule::Unit => "unit",
        DpllRule::Backtrack => "backtrack",
    }
}

fn rule_verb(r: DpllRule) -> &'static str {
    match r {
        DpllRule::Decide => "guess a literal",
        DpllRule::Unit => "propagate clause",
        DpllRule::Backtrack => "flip the last decision",
    }
}

fn rule_chip_classes(r: DpllRule) -> &'static str {
    match r {
        // Reuses the Davis–Putnam palette tokens so the two designers read as
        // one tool; the names are colour slots, not rule names.
        DpllRule::Decide => "bg-pure-bg text-pure-ink",
        DpllRule::Unit => "bg-unit-bg text-unit-ink",
        DpllRule::Backtrack => "bg-elim-bg text-elim-ink",
    }
}
