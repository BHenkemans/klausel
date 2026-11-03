use crate::algos::controls::{
    MAX_CLAUSES_LIMIT, MAX_STEPS_LIMIT, MAX_VARS_LIMIT, MIN_LIMIT, Stepper,
};
use crate::algos::dp::api::{grade_step, random_exercise, verify_trace};
use crate::algos::dp::state::{PracticeHistoryEntry, PracticeState, Rule, Sat};
use crate::algos::feedback::{RuleText, VerdictBanner, step_feedback};
use crate::algos::math::{clause_set_rsx, lit_name, lits_in, vars_in};
use dioxus::core::spawn_forever;
use dioxus::prelude::*;

// ---------- helpers ----------

fn current_set(s: &PracticeState) -> Vec<Vec<i32>> {
    s.history
        .last()
        .map(|h| h.verdict.claimed.clone())
        .unwrap_or_else(|| s.exercise.clone())
}

fn conclude(mut state: Signal<PracticeState>, result: Sat) {
    let snap = state.peek();
    let exercise = snap.exercise.clone();
    let steps = snap
        .history
        .iter()
        .map(|h| (h.rule, h.lit, h.verdict.claimed.clone()))
        .collect();
    drop(snap);
    {
        let mut w = state.write();
        w.concluded = Some(result);
        w.verdict = None;
    }
    spawn_forever(async move {
        if let Ok(v) = verify_trace(exercise, steps, result).await {
            state.write().verdict = Some(v);
        }
    });
}

fn start_over(state: &mut PracticeState) {
    state.history.clear();
    state.working = state.exercise.clone();
    state.draft_lit = None;
    state.concluded = None;
}

fn rule_chip_classes(r: Rule, active: bool) -> &'static str {
    match (r, active) {
        (Rule::Unit, true) => "bg-unit-bg text-unit-ink ring-2 ring-unit-ink/30",
        (Rule::Unit, false) => "bg-unit-bg/40 text-unit-ink/70 hover:bg-unit-bg",
        (Rule::Pure, true) => "bg-pure-bg text-pure-ink ring-2 ring-pure-ink/30",
        (Rule::Pure, false) => "bg-pure-bg/40 text-pure-ink/70 hover:bg-pure-bg",
        (Rule::Elim, true) => "bg-elim-bg text-elim-ink ring-2 ring-elim-ink/30",
        (Rule::Elim, false) => "bg-elim-bg/40 text-elim-ink/70 hover:bg-elim-bg",
    }
}

// ---------- root ----------

#[component]
pub fn Practice() -> Element {
    rsx! {
        div { class: "grid grid-cols-[380px_minmax(0,1fr)] flex-1 items-start",
            ExercisePanel {}
            SolverBody {}
        }
    }
}

// ---------- left panel ----------

#[component]
fn ExercisePanel() -> Element {
    let state = use_context::<Signal<PracticeState>>();
    let snap = state.read();
    let exercise = snap.exercise.clone();
    let loading = snap.practice_loading;
    let error = snap.practice_error.clone();
    drop(snap);
    let set_el = clause_set_rsx(&exercise);
    rsx! {
        div { class: "sticky top-topbar max-h-[calc(100vh-var(--spacing-topbar))] overflow-y-auto border-r border-border bg-panel-bg px-6 py-7 flex flex-col gap-5",
            div { class: "text-[11px] font-bold tracking-[0.08em] text-ink-faint uppercase",
                "Exercise"
            }
            PracticePanel { loading, error }
            div { class: "p-5 bg-surface border border-border rounded-2xl shadow-sm",
                div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase mb-3",
                    "Initial clause set"
                }
                div { class: "font-serif text-[22px] text-ink leading-[1.4] break-words [overflow-wrap:anywhere]",
                    em { "S" }
                    " = "
                    {set_el}
                }
            }
            div { class: "p-4 bg-surface-2 rounded-xl text-[12.5px] text-ink-muted leading-[1.5]",
                div { class: "font-bold text-ink-2 mb-1.5", "How to answer" }
                "1. Pick a rule and a literal. 2. Edit the working set into what you think the resulting clause set should be. 3. Apply step. Mistakes don't block progress — the grader continues from your claimed state."
            }
        }
    }
}

#[component]
fn PracticePanel(loading: bool, error: Option<String>) -> Element {
    let mut state = use_context::<Signal<PracticeState>>();
    let snap = state.read();
    let max_vars = snap.practice_max_vars;
    let max_clauses = snap.practice_max_clauses;
    let max_steps = snap.practice_max_steps;
    drop(snap);

    rsx! {
        div { class: "p-5 bg-surface border border-border rounded-2xl shadow-sm flex flex-col gap-3",
            div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase",
                "Practice — generate a random exercise"
            }
            Stepper {
                label: "Variables",
                value: max_vars,
                on_dec: move |_| {
                    let mut w = state.write();
                    if w.practice_max_vars > MIN_LIMIT {
                        w.practice_max_vars -= 1;
                        // Every rule consumes its variable, so a run can never
                        // be longer than the universe it runs over.
                        w.practice_max_steps = w.practice_max_steps.min(w.practice_max_vars);
                    }
                },
                on_inc: move |_| {
                    let mut w = state.write();
                    if w.practice_max_vars < MAX_VARS_LIMIT { w.practice_max_vars += 1; }
                },
            }
            Stepper {
                label: "Clauses",
                value: max_clauses,
                on_dec: move |_| {
                    let mut w = state.write();
                    if w.practice_max_clauses > MIN_LIMIT { w.practice_max_clauses -= 1; }
                },
                on_inc: move |_| {
                    let mut w = state.write();
                    if w.practice_max_clauses < MAX_CLAUSES_LIMIT { w.practice_max_clauses += 1; }
                },
            }
            Stepper {
                label: "Steps",
                value: max_steps,
                on_dec: move |_| {
                    let mut w = state.write();
                    if w.practice_max_steps > MIN_LIMIT { w.practice_max_steps -= 1; }
                },
                on_inc: move |_| {
                    let mut w = state.write();
                    let ceiling = MAX_STEPS_LIMIT.min(w.practice_max_vars);
                    if w.practice_max_steps < ceiling { w.practice_max_steps += 1; }
                },
            }
            button {
                class: "mt-1 w-full py-2.5 rounded-xl bg-accent text-white text-[13px] font-bold cursor-pointer disabled:opacity-60 disabled:cursor-wait",
                disabled: loading,
                onclick: move |_| {
                    let snap = state.read();
                    let (mv, mc, ms) = (snap.practice_max_vars, snap.practice_max_clauses, snap.practice_max_steps);
                    drop(snap);
                    {
                        let mut w = state.write();
                        w.practice_loading = true;
                        w.practice_error = None;
                    }
                    spawn(async move {
                        let res = random_exercise(mv, mc, ms).await;
                        {
                            let mut w = state.write();
                            w.practice_loading = false;
                            match res {
                                Ok(Some(set)) => {
                                    w.exercise = set.clone();
                                    w.working = set;
                                    w.history.clear();
                                    w.draft_lit = None;
                                    w.draft_rule = Rule::Unit;
                                    w.concluded = None;
                                    w.grading = false;
                                }
                                Ok(None) => {
                                    w.practice_error = Some("No exercise found in five seconds. Davis-Putnam generation is reliable to about 6 variables. Try again, or lower the knobs.".into());
                                }
                                Err(e) => {
                                    w.practice_error = Some(e.to_string());
                                }
                            }
                        }
                    });
                },
                if loading { "Searching…" } else { "🎲 New exercise" }
            }
            if let Some(msg) = error {
                div { class: "text-[12.5px] text-err-ink", "{msg}" }
            }
        }
    }
}

// ---------- right side: history + composer + conclude ----------

#[component]
fn SolverBody() -> Element {
    let mut state = use_context::<Signal<PracticeState>>();
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
                        on_again: move |_| start_over(&mut state.write()),
                    }
                },
            }
        }
    }
}

#[component]
fn HistoryCard(idx: usize, entry: PracticeHistoryEntry) -> Element {
    let v = &entry.verdict;
    let marker = if v.valid() { "✓" } else { "✗" };
    let marker_classes = if v.valid() {
        "text-ok-ink bg-ok-bg"
    } else {
        "text-err-ink bg-err-bg"
    };
    let lit_text = lit_name(entry.lit);
    let claimed_el = clause_set_rsx(&v.claimed);
    let expected_el = v.correction().map(|s| clause_set_rsx(s));
    rsx! {
        div { class: "p-5 bg-surface border border-border rounded-2xl shadow-sm",
            div { class: "flex items-center gap-3 mb-3",
                div { class: "w-7 h-7 rounded-full bg-surface-2 flex items-center justify-center text-[12px] font-bold text-ink-2",
                    "{idx + 1}"
                }
                div { class: "px-3 py-1 rounded-lg text-xs font-bold {rule_chip_classes(entry.rule, true)}",
                    "{entry.rule.name()}"
                }
                div { class: "font-serif text-[16px] italic text-ink", "{lit_text}" }
                div { class: "ml-auto w-7 h-7 rounded-full flex items-center justify-center text-sm font-bold {marker_classes}",
                    "{marker}"
                }
            }
            div { class: "font-serif text-[18px] text-ink-2 leading-[1.4] break-words [overflow-wrap:anywhere]",
                em { "your S′" }
                " = "
                {claimed_el}
            }
            if let Some(expected_el) = expected_el {
                div { class: "mt-2 p-3 bg-ok-bg/50 rounded-lg font-serif text-[16px] text-ok-ink leading-[1.4] break-words [overflow-wrap:anywhere]",
                    em { "expected" }
                    " = "
                    {expected_el}
                }
            }
            {step_feedback(entry.rule, v)}
        }
    }
}

#[component]
fn Composer(grading: bool) -> Element {
    let mut state = use_context::<Signal<PracticeState>>();
    let snap = state.read();
    let from = current_set(&snap);
    let draft_rule = snap.draft_rule;
    let draft_lit = snap.draft_lit;
    let working = snap.working.clone();
    drop(snap);

    let lits_in_palette = lits_in(&from);
    let from_el = clause_set_rsx(&from);
    let can_apply = draft_lit.is_some() && !grading;

    rsx! {
        div { class: "p-5 bg-surface border border-border rounded-2xl shadow-sm flex flex-col gap-4",
            div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase",
                "Next step"
            }
            div { class: "font-serif text-[17px] text-ink-2 break-words [overflow-wrap:anywhere]",
                "From "
                em { "S" }
                " = "
                {from_el}
            }

            div { class: "flex flex-col gap-2",
                div { class: "text-[12.5px] font-bold text-ink-2", "1 · Choose a rule" }
                div { class: "flex gap-2",
                    for r in [Rule::Unit, Rule::Pure, Rule::Elim] {
                        button {
                            class: "px-3 py-1.5 rounded-lg text-xs font-bold cursor-pointer {rule_chip_classes(r, r == draft_rule)}",
                            onclick: move |_| {
                                let mut w = state.write();
                                w.draft_rule = r;
                                w.draft_lit = None;
                            },
                            "{r.name()}"
                        }
                    }
                }
            }

            div { class: "flex flex-col gap-2",
                div { class: "text-[12.5px] font-bold text-ink-2",
                    if matches!(draft_rule, Rule::Elim) { "2 · Choose a variable" } else { "2 · Choose a literal" }
                }
                if lits_in_palette.is_empty() {
                    div { class: "text-[12.5px] text-ink-muted italic", "No literals in the current set." }
                } else {
                    div { class: "flex flex-wrap gap-1.5",
                        for l in lits_in_palette {
                            LitChip { l, active: draft_lit == Some(l) }
                        }
                    }
                }
            }

            div { class: "flex flex-col gap-2",
                div { class: "text-[12.5px] font-bold text-ink-2", "3 · Edit the resulting clause set" }
                ClauseEditor { working, exercise_atoms: vars_in(&from) }
            }

            div { class: "flex items-center gap-3 pt-2 border-t border-border-soft",
                button {
                    class: "px-4 py-2 rounded-xl bg-accent text-white text-sm font-bold cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: !can_apply,
                    onclick: move |_| {
                        let snap = state.read();
                        let from = current_set(&snap);
                        let rule = snap.draft_rule;
                        let Some(lit) = snap.draft_lit else { return; };
                        let claimed = snap.working.clone();
                        drop(snap);
                        state.write().grading = true;
                        spawn(async move {
                            let res = grade_step(from, rule, lit, claimed).await;
                            let mut w = state.write();
                            w.grading = false;
                            match res {
                                Ok(verdict) => {
                                    let next_working = verdict.claimed.clone();
                                    w.history.push(PracticeHistoryEntry { rule, lit, verdict });
                                    w.working = next_working;
                                    w.draft_lit = None;
                                }
                                Err(_) => { /* surface later; ignore for first cut */ }
                            }
                        });
                    },
                    if grading { "Grading…" } else { "Apply step ✓" }
                }
                button {
                    class: "px-3 py-2 rounded-xl border border-border text-ink-muted text-sm hover:text-ink hover:bg-surface-2 cursor-pointer",
                    onclick: move |_| {
                        let mut w = state.write();
                        w.working = current_set(&w);
                        w.draft_lit = None;
                    },
                    "Reset to current"
                }
            }
        }
    }
}

#[component]
fn LitChip(l: i32, active: bool) -> Element {
    let mut state = use_context::<Signal<PracticeState>>();
    let label = lit_name(l);
    let cls = if active {
        "px-2.5 py-1 rounded-md bg-accent text-white text-sm font-serif italic cursor-pointer"
    } else {
        "px-2.5 py-1 rounded-md bg-surface-2 text-ink text-sm font-serif italic cursor-pointer hover:bg-surface border border-border-soft"
    };
    rsx! {
        button {
            class: "{cls}",
            onclick: move |_| state.write().draft_lit = Some(l),
            "{label}"
        }
    }
}

// ---------- clause editor ----------

#[component]
fn ClauseEditor(working: Vec<Vec<i32>>, exercise_atoms: Vec<usize>) -> Element {
    let mut state = use_context::<Signal<PracticeState>>();
    rsx! {
        div { class: "flex flex-col gap-2",
            if working.is_empty() {
                div { class: "text-[13px] text-ink-muted italic font-serif", "∅  (empty set — claim SAT)" }
            }
            for (ci, clause) in working.iter().cloned().enumerate() {
                ClauseRow { ci, clause, atoms: exercise_atoms.clone() }
            }
            div { class: "flex gap-2 pt-1",
                button {
                    class: "px-2.5 py-1 rounded-md border border-dashed border-border text-[12.5px] text-ink-muted hover:text-ink hover:bg-surface-2 cursor-pointer",
                    onclick: move |_| state.write().working.push(vec![]),
                    "+ clause"
                }
            }
        }
    }
}

#[component]
fn ClauseRow(ci: usize, clause: Vec<i32>, atoms: Vec<usize>) -> Element {
    let mut state = use_context::<Signal<PracticeState>>();
    rsx! {
        div { class: "flex items-center gap-1.5 flex-wrap px-3 py-2 bg-surface-2 rounded-lg border border-border-soft",
            span { class: "font-serif text-ink-faint", "{{" }
            if clause.is_empty() {
                span { class: "font-serif text-err-ink italic", "□" }
            }
            for (li, l) in clause.iter().copied().enumerate() {
                button {
                    class: "px-2 py-0.5 rounded bg-surface text-ink font-serif italic text-sm hover:bg-err-bg hover:text-err-ink cursor-pointer",
                    title: "click to remove",
                    onclick: move |_| {
                        let mut w = state.write();
                        w.working[ci].remove(li);
                    },
                    "{lit_name(l)}"
                }
                if li + 1 < clause.len() {
                    span { class: "text-ink-faint", "," }
                }
            }
            span { class: "font-serif text-ink-faint", "}}" }
            // The addable literals sit in the row rather than behind a
            // toggle: building a clause is the common action here, and one
            // click should be enough for it.
            div { class: "ml-auto flex items-center gap-1.5",
                span { class: "text-ink-faint text-[11px] mr-0.5", "add" }
                for a in atoms.iter().copied() {
                    for sign in [1i32, -1] {
                        {
                            let l = sign * (a as i32);
                            let already = clause.contains(&l);
                            rsx! {
                                button {
                                    key: "{l}",
                                    class: if already {
                                        "px-1.5 py-0.5 rounded text-ink-faint/50 font-serif italic text-[13px] cursor-not-allowed"
                                    } else {
                                        "px-1.5 py-0.5 rounded bg-surface text-ink-muted font-serif italic text-[13px] hover:bg-accent-soft hover:text-ink cursor-pointer border border-border-soft"
                                    },
                                    disabled: already,
                                    onclick: move |_| { state.write().working[ci].push(l); },
                                    "{lit_name(l)}"
                                }
                            }
                        }
                    }
                }
                button {
                    class: "ml-1 px-1.5 py-0.5 rounded text-[12px] text-err-ink hover:bg-err-bg cursor-pointer",
                    title: "delete clause",
                    onclick: move |_| { state.write().working.remove(ci); },
                    "×"
                }
            }
        }
    }
}

// ---------- conclude ----------

#[component]
fn ConcludePanel() -> Element {
    let mut state = use_context::<Signal<PracticeState>>();
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
                            w.working = current_set(&w);
                            w.draft_lit = None;
                        }
                    },
                    "Undo"
                }
                button {
                    class: "px-3 py-2 rounded-xl border border-border text-ink-muted text-sm hover:text-ink hover:bg-surface-2 cursor-pointer",
                    onclick: move |_| start_over(&mut state.write()),
                    "Reset"
                }
            }
        }
    }
}
