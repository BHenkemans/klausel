use crate::algos::cdcl::api::{grade_cdcl_step, random_cdcl_exercise, verify_cdcl_trace};
use crate::algos::cdcl::state::{
    CdclPracticeHistoryEntry, CdclPracticeState, CdclRule, CdclStep, Sat, TrailEntry,
};
use crate::algos::cdcl::view::TrailChips;
use crate::algos::controls::{
    MAX_CLAUSES_LIMIT, MAX_STEPS_LIMIT, MAX_VARS_LIMIT, MIN_LIMIT, Stepper, TrailLegend,
};
use crate::algos::feedback::{RuleText, VerdictBanner, step_feedback};
use crate::algos::math::{clause_rsx, lit_name, lits_in};
use dioxus::core::spawn_forever;
use dioxus::prelude::*;

fn conclude(mut state: Signal<CdclPracticeState>, result: Sat) {
    let snap = state.peek();
    let formula = snap.formula.clone();
    let steps = snap
        .history
        .iter()
        .map(|h| (h.step, h.verdict.claimed.clone()))
        .collect();
    drop(snap);
    {
        let mut w = state.write();
        w.concluded = Some(result);
        w.verdict = None;
    }
    spawn_forever(async move {
        if let Ok(v) = verify_cdcl_trace(formula, steps, result).await {
            state.write().verdict = Some(v);
        }
    });
}

fn rule_chip_classes(r: CdclRule, active: bool) -> &'static str {
    match (r, active) {
        (CdclRule::Unit, true) => "bg-unit-bg text-unit-ink ring-2 ring-unit-ink/30",
        (CdclRule::Unit, false) => "bg-unit-bg/40 text-unit-ink/70 hover:bg-unit-bg",
        (CdclRule::Decide, true) => "bg-pure-bg text-pure-ink ring-2 ring-pure-ink/30",
        (CdclRule::Decide, false) => "bg-pure-bg/40 text-pure-ink/70 hover:bg-pure-bg",
        (CdclRule::Conflict, true) => "bg-elim-bg text-elim-ink ring-2 ring-elim-ink/30",
        (CdclRule::Conflict, false) => "bg-elim-bg/40 text-elim-ink/70 hover:bg-elim-bg",
    }
}

fn step_caption(s: &CdclStep) -> String {
    match s.rule {
        CdclRule::Decide => lit_name(s.lit()),
        CdclRule::Unit | CdclRule::Conflict => format!("c{}", s.ci),
    }
}

#[component]
pub fn CdclPractice() -> Element {
    rsx! {
        div { class: "grid grid-cols-[380px_minmax(0,1fr)] flex-1 items-start",
            FormulaPanel {}
            SolverBody {}
        }
    }
}

#[component]
fn PracticePanel() -> Element {
    let mut state = use_context::<Signal<CdclPracticeState>>();
    let snap = state.read();
    let (mv, mc, ms) = (
        snap.practice_max_vars,
        snap.practice_max_clauses,
        snap.practice_max_steps,
    );
    let loading = snap.practice_loading;
    let error = snap.practice_error.clone();
    drop(snap);

    rsx! {
        div { class: "p-5 bg-surface border border-border rounded-2xl shadow-sm flex flex-col gap-3",
            div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase",
                "Practice \u{2014} generate a random exercise"
            }
            Stepper {
                label: "Variables",
                value: mv,
                on_dec: move |_| { let mut w = state.write(); if w.practice_max_vars > MIN_LIMIT { w.practice_max_vars -= 1; } },
                on_inc: move |_| { let mut w = state.write(); if w.practice_max_vars < MAX_VARS_LIMIT { w.practice_max_vars += 1; } },
            }
            Stepper {
                label: "Clauses",
                value: mc,
                on_dec: move |_| { let mut w = state.write(); if w.practice_max_clauses > MIN_LIMIT { w.practice_max_clauses -= 1; } },
                on_inc: move |_| { let mut w = state.write(); if w.practice_max_clauses < MAX_CLAUSES_LIMIT { w.practice_max_clauses += 1; } },
            }
            Stepper {
                label: "Steps",
                value: ms,
                on_dec: move |_| { let mut w = state.write(); if w.practice_max_steps > MIN_LIMIT { w.practice_max_steps -= 1; } },
                on_inc: move |_| { let mut w = state.write(); if w.practice_max_steps < MAX_STEPS_LIMIT { w.practice_max_steps += 1; } },
            }
            button {
                class: "mt-1 w-full py-2.5 rounded-xl bg-accent text-white text-[13px] font-bold cursor-pointer disabled:opacity-60 disabled:cursor-wait",
                disabled: loading,
                onclick: move |_| {
                    {
                        let mut w = state.write();
                        w.practice_loading = true;
                        w.practice_error = None;
                    }
                    spawn(async move {
                        let res = random_cdcl_exercise(mv, mc, ms).await;
                        {
                            let mut w = state.write();
                            w.practice_loading = false;
                            match res {
                                Ok(Some(ex)) => {
                                    w.formula = ex.set;
                                    w.start_over();
                                }
                                Ok(None) => {
                                    w.practice_error = Some("No exercise found in five seconds. CDCL generation is reliable to about 5 variables. Try again, or lower the knobs.".into());
                                }
                                Err(e) => w.practice_error = Some(e.to_string()),
                            }
                        }
                    });
                },
                if loading { "Searching\u{2026}" } else { "\u{1F3B2} New exercise" }
            }
            if let Some(msg) = error {
                div { class: "text-[12.5px] text-err-ink", "{msg}" }
            }
        }
    }
}

#[component]
fn FormulaPanel() -> Element {
    let state = use_context::<Signal<CdclPracticeState>>();
    let snap = state.read();
    let original = snap.formula.len();
    let phi = snap.current_state().phi;
    drop(snap);
    rsx! {
        div { class: "sticky top-topbar max-h-[calc(100vh-var(--spacing-topbar))] overflow-y-auto border-r border-border bg-panel-bg px-6 py-7 flex flex-col gap-5",
            div { class: "text-[11px] font-bold tracking-[0.08em] text-ink-faint uppercase",
                "Exercise"
            }
            PracticePanel {}
            div { class: "p-5 bg-surface border border-border rounded-2xl shadow-sm",
                div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase mb-3",
                    "Formula"
                }
                div { class: "flex flex-col gap-1",
                    for (ci, c) in phi.iter().enumerate() {
                        div { class: "flex items-baseline gap-3 px-2 py-1 rounded-lg odd:bg-surface-2/50",
                            span { class: "font-mono text-[12px] text-ink-faint w-6 shrink-0", "c{ci}" }
                            span { class: "font-serif text-[17px] text-ink", {clause_rsx(c)} }
                            if ci >= original {
                                span { class: "ml-auto px-2 py-0.5 rounded-md bg-elim-bg text-elim-ink text-[11px] font-bold",
                                    "learned"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "p-4 bg-surface-2 rounded-xl text-[12.5px] text-ink-muted leading-[1.5]",
                div { class: "font-bold text-ink-2 mb-1.5", "How to answer" }
                "1. Pick a rule. 2. Name the literal to decide, or the clause that fires or conflicts. 3. For a conflict, build the learned clause. 4. Edit the trail into what you think it becomes. Antecedents follow from the unit steps you name. Mistakes don't block progress — the grader continues from your claimed state."
            }
        }
    }
}

#[component]
fn SolverBody() -> Element {
    let mut state = use_context::<Signal<CdclPracticeState>>();
    let snap = state.read();
    let history = snap.history.clone();
    let concluded = snap.concluded;
    let verdict = snap.verdict;
    let grading = snap.grading;
    drop(snap);

    rsx! {
        div { class: "p-7 flex flex-col gap-5 min-w-0",
            div { class: "text-[11px] font-bold tracking-[0.08em] text-ink-faint uppercase",
                "Solver"
            }
            for (i, entry) in history.iter().cloned().enumerate() {
                HistoryCard { idx: i, entry }
            }
            match concluded {
                None => rsx! {
                    Composer { grading }
                    ConcludePanel {}
                },
                Some(concluded) => rsx! {
                    VerdictBanner {
                        concluded,
                        verdict,
                        on_again: move |_| state.write().start_over(),
                    }
                },
            }
        }
    }
}

#[component]
fn HistoryCard(idx: usize, entry: CdclPracticeHistoryEntry) -> Element {
    let v = &entry.verdict;
    let marker = if v.valid() { "✓" } else { "✗" };
    let marker_classes = if v.valid() {
        "text-ok-ink bg-ok-bg"
    } else {
        "text-err-ink bg-err-bg"
    };
    let is_conflict = entry.step.rule == CdclRule::Conflict;
    let claimed_learned = v.claimed.phi.last().cloned().unwrap_or_default();
    rsx! {
        div { class: "p-5 bg-surface border border-border rounded-2xl shadow-sm",
            div { class: "flex items-center gap-3 mb-3",
                div { class: "w-7 h-7 rounded-full bg-surface-2 flex items-center justify-center text-[12px] font-bold text-ink-2",
                    "{idx + 1}"
                }
                div { class: "px-3 py-1 rounded-lg text-xs font-bold {rule_chip_classes(entry.step.rule, true)}",
                    "{entry.step.rule.name()}"
                }
                div { class: "font-serif text-[16px] italic text-ink", "{step_caption(&entry.step)}" }
                div { class: "ml-auto w-7 h-7 rounded-full flex items-center justify-center text-sm font-bold {marker_classes}",
                    "{marker}"
                }
            }
            if is_conflict {
                div { class: "flex items-center gap-2 flex-wrap mb-2",
                    span { class: "font-serif text-[17px] text-ink-2 italic", "learned" }
                    span { class: "font-serif text-[17px] text-ink-2", "=" }
                    span { class: "font-serif text-[17px] text-ink", {clause_rsx(&claimed_learned)} }
                }
            }
            div { class: "flex items-center gap-2 flex-wrap",
                span { class: "font-serif text-[17px] text-ink-2 italic", "your M" }
                span { class: "font-serif text-[17px] text-ink-2", "=" }
                TrailChips { trail: v.claimed.trail.clone() }
            }
            if let Some(expected) = v.correction() {
                div { class: "mt-2 p-3 bg-ok-bg/50 rounded-lg flex flex-col gap-2",
                    if is_conflict {
                        div { class: "flex items-center gap-2 flex-wrap",
                            span { class: "font-serif text-[15px] text-ok-ink italic", "expected learned" }
                            span { class: "font-serif text-[15px] text-ok-ink", "=" }
                            span { class: "font-serif text-[15px] text-ink",
                                {clause_rsx(&expected.phi.last().cloned().unwrap_or_default())}
                            }
                        }
                    }
                    div { class: "flex items-center gap-2 flex-wrap",
                        span { class: "font-serif text-[15px] text-ok-ink italic", "expected M" }
                        span { class: "font-serif text-[15px] text-ok-ink", "=" }
                        TrailChips { trail: expected.trail.clone() }
                    }
                }
            }
            {step_feedback(entry.step.rule, v)}
        }
    }
}

#[component]
fn Composer(grading: bool) -> Element {
    let mut state = use_context::<Signal<CdclPracticeState>>();
    let snap = state.read();
    let from = snap.current_state();
    let draft_rule = snap.draft_rule;
    let draft_lit = snap.draft_lit;
    let draft_ci = snap.draft_ci;
    let working = snap.working.clone();
    let working_learned = snap.working_learned.clone();
    let lits = lits_in(&snap.formula);
    drop(snap);

    let needs_lit = draft_rule == CdclRule::Decide;
    let is_conflict = draft_rule == CdclRule::Conflict;
    let parameter_chosen = if needs_lit {
        draft_lit.is_some()
    } else {
        draft_ci.is_some()
    };
    let can_apply = !grading && parameter_chosen && (!is_conflict || !working_learned.is_empty());

    rsx! {
        div { class: "p-5 bg-surface border border-border rounded-2xl shadow-sm flex flex-col gap-4",
            div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase",
                "Next step"
            }
            div { class: "flex items-center gap-2 flex-wrap",
                span { class: "font-serif text-[17px] text-ink-2", "From " }
                em { class: "font-serif text-[17px] text-ink-2", "M" }
                span { class: "font-serif text-[17px] text-ink-2", " = " }
                TrailChips { trail: from.trail.clone() }
            }

            div { class: "flex flex-col gap-2",
                div { class: "text-[12.5px] font-bold text-ink-2", "1 · Choose a rule" }
                div { class: "flex gap-2",
                    for r in [CdclRule::Conflict, CdclRule::Unit, CdclRule::Decide] {
                        button {
                            class: "px-3 py-1.5 rounded-lg text-xs font-bold cursor-pointer {rule_chip_classes(r, r == draft_rule)}",
                            onclick: move |_| {
                                let mut w = state.write();
                                w.draft_rule = r;
                                w.draft_lit = None;
                                w.draft_ci = None;
                            },
                            "{r.name()}"
                        }
                    }
                }
            }

            div { class: "flex flex-col gap-2",
                div { class: "text-[12.5px] font-bold text-ink-2",
                    match draft_rule {
                        CdclRule::Decide => "2 · Choose a literal to decide",
                        CdclRule::Unit => "2 · Choose the clause that fires",
                        CdclRule::Conflict => "2 · Choose the clause in conflict",
                    }
                }
                if needs_lit {
                    div { class: "flex flex-wrap gap-1.5",
                        for l in lits.clone() {
                            LitChip { l, active: draft_lit == Some(l) }
                        }
                    }
                } else {
                    div { class: "flex flex-wrap gap-1.5",
                        for ci in 0..from.phi.len() {
                            ClauseChip { ci, active: draft_ci == Some(ci) }
                        }
                    }
                }
            }

            if is_conflict {
                div { class: "flex flex-col gap-2",
                    div { class: "text-[12.5px] font-bold text-ink-2", "3 · Build the learned clause" }
                    LearnedEditor { learned: working_learned, lits: lits.clone() }
                }
            }

            div { class: "flex flex-col gap-2",
                div { class: "text-[12.5px] font-bold text-ink-2",
                    if is_conflict { "4 · Edit the trail after backjumping" } else { "3 · Edit the resulting trail" }
                }
                TrailEditor { working, lits }
            }

            div { class: "flex items-center gap-3 pt-2 border-t border-border-soft",
                button {
                    class: "px-4 py-2 rounded-xl bg-accent text-white text-sm font-bold cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: !can_apply,
                    onclick: move |_| {
                        let snap = state.read();
                        let from = snap.current_state();
                        let claimed = snap.claimed_state();
                        let rule = snap.draft_rule;
                        let lit = snap.draft_lit.unwrap_or(0);
                        let ci = snap.draft_ci.unwrap_or(0);
                        drop(snap);
                        state.write().grading = true;
                        spawn(async move {
                            let res = grade_cdcl_step(from, rule, lit, ci, claimed).await;
                            let mut w = state.write();
                            w.grading = false;
                            if let Ok(verdict) = res {
                                let vi = lit.unsigned_abs() as usize;
                                w.history.push(CdclPracticeHistoryEntry {
                                    step: CdclStep { rule, vi, neg: lit < 0, ci },
                                    verdict,
                                });
                                w.reset_working();
                            }
                        });
                    },
                    if grading { "Grading…" } else { "Apply step ✓" }
                }
                button {
                    class: "px-3 py-2 rounded-xl border border-border text-ink-muted text-sm hover:text-ink hover:bg-surface-2 cursor-pointer",
                    onclick: move |_| state.write().reset_working(),
                    "Reset to current"
                }
            }
        }
    }
}

#[component]
fn LitChip(l: i32, active: bool) -> Element {
    let mut state = use_context::<Signal<CdclPracticeState>>();
    let cls = if active {
        "px-2.5 py-1 rounded-md bg-accent text-white text-sm font-serif italic cursor-pointer"
    } else {
        "px-2.5 py-1 rounded-md bg-surface-2 text-ink text-sm font-serif italic cursor-pointer hover:bg-surface border border-border-soft"
    };
    rsx! {
        button { class: "{cls}", onclick: move |_| state.write().draft_lit = Some(l), "{lit_name(l)}" }
    }
}

#[component]
fn ClauseChip(ci: usize, active: bool) -> Element {
    let mut state = use_context::<Signal<CdclPracticeState>>();
    let cls = if active {
        "px-2.5 py-1 rounded-md bg-accent text-white text-sm font-mono cursor-pointer"
    } else {
        "px-2.5 py-1 rounded-md bg-surface-2 text-ink text-sm font-mono cursor-pointer hover:bg-surface border border-border-soft"
    };
    rsx! {
        button { class: "{cls}", onclick: move |_| state.write().draft_ci = Some(ci), "c{ci}" }
    }
}

#[component]
fn LearnedEditor(learned: Vec<i32>, lits: Vec<i32>) -> Element {
    let mut state = use_context::<Signal<CdclPracticeState>>();
    rsx! {
        div { class: "flex flex-col gap-2",
            div { class: "flex items-center gap-1.5 flex-wrap px-3 py-2 bg-surface-2 rounded-lg border border-border-soft min-h-[42px] font-serif text-[16px]",
                if learned.is_empty() {
                    span { class: "text-[13px] text-ink-muted italic", "pick at least one literal" }
                } else {
                    {clause_rsx(&learned)}
                }
            }
            div { class: "flex items-center flex-wrap gap-1 px-3",
                span { class: "text-ink-faint text-[11px] mr-1", "toggle" }
                for l in lits {
                    button {
                        key: "{l}",
                        class: if learned.contains(&l) {
                            "px-2 py-0.5 rounded bg-elim-bg text-elim-ink font-serif italic text-[13px] cursor-pointer border border-border-soft"
                        } else {
                            "px-2 py-0.5 rounded bg-surface text-ink-muted font-serif italic text-[13px] hover:bg-accent-soft hover:text-ink cursor-pointer border border-border-soft"
                        },
                        onclick: move |_| state.write().toggle_learned(l),
                        "{lit_name(l)}"
                    }
                }
            }
        }
    }
}

#[component]
fn TrailEditor(working: Vec<TrailEntry>, lits: Vec<i32>) -> Element {
    let mut state = use_context::<Signal<CdclPracticeState>>();
    rsx! {
        div { class: "flex flex-col gap-2",
            TrailLegend {}
            div { class: "flex items-center gap-1.5 flex-wrap px-3 py-2 bg-surface-2 rounded-lg border border-border-soft min-h-[42px]",
                if working.is_empty() {
                    span { class: "font-serif text-[13px] text-ink-muted italic", "∅  (empty trail)" }
                }
                for (i, e) in working.iter().copied().enumerate() {
                    div { class: "flex items-center rounded-md overflow-hidden border border-border-soft",
                        button {
                            class: if e.decision {
                                "px-2 py-0.5 bg-accent text-white font-serif text-sm cursor-pointer hover:opacity-80"
                            } else {
                                "px-2 py-0.5 bg-surface text-ink font-serif text-sm cursor-pointer hover:bg-accent-soft"
                            },
                            title: if e.decision { "decision — click to make it propagated" } else { "propagated — click to make it a decision" },
                            onclick: move |_| {
                                let mut w = state.write();
                                w.working[i].decision = !w.working[i].decision;
                            },
                            "{lit_name(e.lit)}"
                        }
                        button {
                            class: "px-1.5 py-0.5 bg-surface text-[12px] text-err-ink hover:bg-err-bg cursor-pointer",
                            title: "remove from trail",
                            onclick: move |_| { state.write().working.remove(i); },
                            "×"
                        }
                    }
                }
            }
            div { class: "flex items-center flex-wrap gap-1 px-3",
                span { class: "text-ink-faint text-[11px] mr-1", "append" }
                for l in lits {
                    {
                        let assigned = working.iter().any(|e| e.lit.abs() == l.abs());
                        rsx! {
                            button {
                                key: "{l}",
                                class: if assigned {
                                    "px-2 py-0.5 rounded text-ink-faint/50 font-serif italic text-[13px] cursor-not-allowed"
                                } else {
                                    "px-2 py-0.5 rounded bg-surface text-ink-muted font-serif italic text-[13px] hover:bg-accent-soft hover:text-ink cursor-pointer border border-border-soft"
                                },
                                disabled: assigned,
                                onclick: move |_| {
                                    state.write().working.push(TrailEntry { lit: l, decision: false, antecedent: None });
                                },
                                "{lit_name(l)}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ConcludePanel() -> Element {
    let mut state = use_context::<Signal<CdclPracticeState>>();
    rsx! {
        div { class: "p-5 bg-surface border border-border rounded-2xl shadow-sm flex items-center gap-3",
            div { class: "text-[12.5px] font-bold text-ink-2 mr-2", "Conclude:" }
            button {
                class: "px-4 py-2 rounded-xl bg-ok-bg text-ok-ink text-sm font-bold cursor-pointer hover:opacity-90",
                onclick: move |_| conclude(state, Sat::Sat),
                "Satisfiable  ∅"
            }
            button {
                class: "px-4 py-2 rounded-xl bg-err-bg text-err-ink text-sm font-bold cursor-pointer hover:opacity-90",
                onclick: move |_| conclude(state, Sat::Unsat),
                "Unsatisfiable  □"
            }
            div { class: "ml-auto flex gap-2",
                button {
                    class: "px-3 py-2 rounded-xl border border-border text-ink-muted text-sm hover:text-ink hover:bg-surface-2 cursor-pointer",
                    onclick: move |_| {
                        let mut w = state.write();
                        if w.history.pop().is_some() {
                            w.reset_working();
                        }
                    },
                    "Undo"
                }
                button {
                    class: "px-3 py-2 rounded-xl border border-border text-ink-muted text-sm hover:text-ink hover:bg-surface-2 cursor-pointer",
                    onclick: move |_| state.write().start_over(),
                    "Reset"
                }
            }
        }
    }
}
