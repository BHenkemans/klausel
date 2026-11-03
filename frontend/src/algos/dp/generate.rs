use crate::algos::controls::{MIN_LIMIT, Stepper};
use crate::algos::dp::api::generate_formula;
use crate::algos::dp::state::{
    DpAppState, GenerateState, GeneratedFormula, Module, PracticeState, Rule, Sat, TraceStep,
};
use crate::algos::math::{clause_set_rsx, cnf_rsx, var_count, var_name};
use crate::algos::paste_box::TracePasteBox;
use dioxus::prelude::*;

use crate::algos::dp::state::{MAX_CLAUSES, MAX_VARS};

#[component]
pub fn Generate() -> Element {
    rsx! {
        div { class: "grid grid-cols-[400px_minmax(0,1fr)] flex-1 items-start",
            TraceDesigner {}
            ResultPanel {}
        }
    }
}

#[component]
fn TraceDesigner() -> Element {
    let mut state = use_context::<Signal<GenerateState>>();
    let s = state.read();
    let target = s.target;
    let steps = s.steps.clone();
    let max_vars = s.max_vars;
    let max_clauses = s.max_clauses;
    drop(s);

    rsx! {
        div { class: "border-r border-border bg-panel-bg px-6 py-7",
            // Solution trace heading
            div { class: "text-[11px] font-bold tracking-[0.08em] text-ink-faint uppercase",
                "Solution trace"
            }
            div { class: "text-[12.5px] text-ink-muted mt-1.5 leading-[1.5]",
                "The steps a student should perform, in order. The studio builds a formula that solves in exactly these steps."
            }

            TracePasteBox {
                apply: move |text: String| state.write().apply_trace(&text),
                placeholder: "unit 0\npure 4\nelim 3\nsat",
                hint: "unit / pure / elim, one per line",
            }

            // Target result
            div { class: "mt-5 text-[12.5px] font-semibold text-ink-2 mb-2", "Target result" }
            div { class: "flex gap-1.5 bg-surface-2 p-1 rounded-[11px]",
                TargetButton { value: Sat::Sat, label: "Satisfiable", symbol: "∅", current: target }
                TargetButton { value: Sat::Unsat, label: "Unsatisfiable", symbol: "□", current: target }
            }

            // Step list
            div { class: "mt-5 flex flex-col gap-2.5",
                for (i, step) in steps.iter().enumerate() {
                    StepRow { idx: i, step: *step, max_vars }
                }
                button {
                    class: "flex items-center justify-center gap-2 py-3 border-[1.5px] border-dashed border-border rounded-[13px] text-[13px] font-semibold text-ink-muted cursor-pointer hover:bg-surface w-full",
                    onclick: move |_| {
                        let mut w = state.write();
                        w.steps.push(TraceStep { rule: Rule::Unit, vi: 1, neg: false });
                        w.latest_result = None;
                    },
                    "＋ Add step"
                }
            }

            // Constraints
            div { class: "mt-6 text-[11px] font-bold tracking-[0.08em] text-ink-faint uppercase",
                "Constraints"
            }
            div { class: "mt-3 flex flex-col gap-2.5",
                Stepper {
                    label: "Max variables",
                    value: max_vars,
                    on_dec: move |_| {
                        let mut w = state.write();
                        if w.max_vars > MIN_LIMIT {
                            w.max_vars -= 1;
                            let cap = w.max_vars;
                            for st in &mut w.steps {
                                if st.vi > cap { st.vi = cap; }
                            }
                            w.latest_result = None;
                        }
                    },
                    on_inc: move |_| {
                        let mut w = state.write();
                        if w.max_vars < MAX_VARS {
                            w.max_vars += 1;
                            w.latest_result = None;
                        }
                    },
                }
                Stepper {
                    label: "Max clauses",
                    value: max_clauses,
                    on_dec: move |_| {
                        let mut w = state.write();
                        if w.max_clauses > MIN_LIMIT {
                            w.max_clauses -= 1;
                            w.latest_result = None;
                        }
                    },
                    on_inc: move |_| {
                        let mut w = state.write();
                        if w.max_clauses < MAX_CLAUSES {
                            w.max_clauses += 1;
                            w.latest_result = None;
                        }
                    },
                }

            }

            // Generate button
            button {
                class: "mt-7 w-full py-3.5 rounded-xl bg-accent text-white text-sm font-bold flex items-center justify-center gap-2 shadow-md hover:shadow-lg cursor-pointer disabled:opacity-60 disabled:cursor-wait",
                disabled: state.read().is_loading,
                onclick: move |_| {
                    let cur = state.read();
                    if cur.is_loading || cur.steps.is_empty() {
                        return;
                    }
                    let steps = cur.steps.clone();
                    let target = cur.target;
                    let max_vars = cur.max_vars;
                    let max_clauses = cur.max_clauses;
                    drop(cur);
                    state.write().is_loading = true;
                    spawn(async move {
                        let result: Result<GeneratedFormula, String> =
                            match generate_formula(steps, target, max_vars, max_clauses).await {
                                Err(e) => Err(e.to_string()),
                                Ok(None) => Err("No formula found within these constraints".to_string()),
                                Ok(Some(f)) => Ok(f),
                            };
                        let mut w = state.write();
                        w.latest_result = Some(result);
                        w.is_loading = false;
                    });
                },
                if state.read().is_loading {
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
    let mut state = use_context::<Signal<GenerateState>>();
    let active = value == current;
    rsx! {
        button {
            class: if active {
                "flex-1 px-3 py-2 rounded-lg text-[12.5px] font-medium bg-surface text-ink shadow-sm flex items-center justify-center gap-1.5"
            } else {
                "flex-1 px-3 py-2 rounded-lg text-[12.5px] text-ink-muted hover:text-ink flex items-center justify-center gap-1.5"
            },
            onclick: move |_| {
                let mut w = state.write();
                w.target = value;
                w.latest_result = None;
            },
            "{label}"
            span { class: "font-serif text-[15px]", "{symbol}" }
        }
    }
}

#[component]
fn StepRow(idx: usize, step: TraceStep, max_vars: usize) -> Element {
    let mut state = use_context::<Signal<GenerateState>>();
    let num = idx + 1;
    let rule = step.rule;
    let neg = step.neg;
    let vi = step.vi;

    rsx! {
        div { class: "flex items-center gap-2.5 px-3 py-3 bg-surface border border-border rounded-[13px] shadow-sm",
            // Step number
            div { class: "w-6 h-6 rounded-full bg-surface-2 flex items-center justify-center text-xs font-bold text-ink-2 shrink-0",
                "{num}"
            }
            // Rule chip
            div {
                class: "px-3 py-1 rounded-lg text-xs font-bold cursor-pointer {rule_chip_classes(rule)}",
                title: "click to change rule",
                onclick: move |_| {
                    let mut w = state.write();
                    w.steps[idx].rule = next_rule(rule);
                    w.latest_result = None;
                },
                "{rule_label(rule)}"
            }
            // Rule verb
            div { class: "text-[12.5px] text-ink-muted whitespace-nowrap overflow-hidden text-ellipsis",
                "{rule_verb(rule)}"
            }
            // Right cluster
            div { class: "ml-auto flex items-center gap-1.5 shrink-0",
                // ¬ toggle (hidden for elim)
                if !matches!(rule, Rule::Elim) {
                    div {
                        class: if neg {
                            "px-2 py-1 rounded-md bg-accent text-white text-sm cursor-pointer font-serif"
                        } else {
                            "px-2 py-1 rounded-md border border-border text-ink-faint text-sm cursor-pointer hover:text-ink font-serif"
                        },
                        title: "toggle negation",
                        onclick: move |_| {
                            let mut w = state.write();
                            w.steps[idx].neg = !w.steps[idx].neg;
                            w.latest_result = None;
                        },
                        "¬"
                    }
                }
                // Variable token
                div {
                    class: "px-3 py-1 rounded-lg border border-border bg-surface-2 font-serif text-[17px] italic text-ink cursor-pointer min-w-[14px] text-center",
                    title: "click to change variable",
                    onclick: move |_| {
                        let mut w = state.write();
                        let new_vi = (vi % max_vars) + 1;
                        w.steps[idx].vi = new_vi;
                        w.latest_result = None;
                    },
                    "{var_name(vi)}"
                }
                // Remove
                div {
                    class: "w-6 h-6 rounded-md flex items-center justify-center text-ink-faint cursor-pointer hover:bg-surface-2 hover:text-ink text-[15px]",
                    title: "remove step",
                    onclick: move |_| {
                        let mut w = state.write();
                        w.steps.remove(idx);
                        w.latest_result = None;
                    },
                    "×"
                }
            }
        }
    }
}

#[component]
fn ResultPanel() -> Element {
    let state = use_context::<Signal<GenerateState>>();
    let snap = state.read();
    let is_loading = snap.is_loading;
    let result = snap.latest_result.clone();
    drop(snap);

    rsx! {
        div { class: "p-7 flex flex-col gap-5 min-w-0",
            div { class: "text-[11px] font-bold tracking-[0.08em] text-ink-faint uppercase",
                "Generated exercise"
            }
            match (is_loading, result) {
                (true, _) => rsx! { LoadingCard {} },
                (false, Some(Ok(f))) => rsx! { FormulaCard { formula: f } },
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
            div { class: "font-serif text-[30px] text-accent mb-3", "{{ {{ … }} }}" }
            div { class: "text-sm font-bold text-ink-2", "Design a trace, then press Generate" }
            div { class: "text-[13px] text-ink-muted mt-1.5 leading-[1.5] max-w-[400px] mx-auto",
                "The studio searches for a Davis–Putnam formula whose solution is exactly your sequence of steps — verified by running the algorithm forward."
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
                "Z3 is exploring CNF clause sets that realise your trace."
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
            div { class: "text-[13px] text-ink-muted mt-1.5 leading-[1.5] max-w-[380px] mx-auto",
                "Try raising max clauses or variables, or adjust the trace — some traces aren't realisable in algorithm order."
            }
        }
    }
}

#[component]
fn FormulaCard(formula: GeneratedFormula) -> Element {
    let set_el = clause_set_rsx(&formula.set);
    let cnf_el = cnf_rsx(&formula.set);
    let n_clauses = formula.set.len();
    let n_vars = var_count(&formula.set);

    rsx! {
        div { class: "p-6 bg-surface border border-border rounded-2xl shadow-sm",
            div { class: "text-[11px] font-bold tracking-[0.06em] text-ink-faint uppercase mb-3",
                "Clausal form"
            }
            div { class: "font-serif text-[27px] text-ink leading-[1.4] tracking-[0.01em] break-words [overflow-wrap:anywhere]",
                em { "S" }
                " = "
                {set_el}
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
            div { class: "flex items-center gap-3 mt-5 flex-wrap",
                span { class: "inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-accent-soft text-accent text-[12.5px] font-bold",
                    "✓ Realises the trace"
                }
                span { class: "text-[12.5px] text-ink-muted",
                    "{n_clauses} clauses · {n_vars} vars"
                }
                PracticeThisButton { set: formula.set.clone() }
            }
        }
    }
}

#[component]
fn PracticeThisButton(set: Vec<Vec<i32>>) -> Element {
    let mut app = use_context::<Signal<DpAppState>>();
    let solve = use_context::<Signal<PracticeState>>();
    rsx! {
        button {
            class: "ml-auto px-3.5 py-1.5 rounded-lg bg-accent text-white text-[12.5px] font-bold cursor-pointer hover:opacity-90",
            onclick: move |_| {
                {
                    let mut s = solve.clone();
                    let mut w = s.write();
                    w.exercise = set.clone();
                    w.working = set.clone();
                    w.history.clear();
                    w.draft_lit = None;
                    w.draft_rule = Rule::Unit;
                    w.concluded = None;
                    w.grading = false;
                }
                app.write().module = Module::Practice;
            },
            "Practice this →"
        }
    }
}

// ----- helpers -----

fn next_rule(r: Rule) -> Rule {
    match r {
        Rule::Unit => Rule::Pure,
        Rule::Pure => Rule::Elim,
        Rule::Elim => Rule::Unit,
    }
}

fn rule_label(r: Rule) -> &'static str {
    match r {
        Rule::Unit => "unit",
        Rule::Pure => "pure",
        Rule::Elim => "elim",
    }
}

fn rule_verb(r: Rule) -> &'static str {
    match r {
        Rule::Unit => "unit-literal",
        Rule::Pure => "pure-literal",
        Rule::Elim => "eliminate",
    }
}

fn rule_chip_classes(r: Rule) -> &'static str {
    match r {
        Rule::Unit => "bg-unit-bg text-unit-ink",
        Rule::Pure => "bg-pure-bg text-pure-ink",
        Rule::Elim => "bg-elim-bg text-elim-ink",
    }
}
