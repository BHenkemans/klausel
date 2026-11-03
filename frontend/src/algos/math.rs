use dioxus::prelude::*;
pub const NAMES: [&str; 24] = [
    "α", "β", "γ", "δ", "ε", "ζ", "η", "θ", "ι", "κ", "λ", "μ", "ν", "ξ", "ο", "π", "ρ", "σ", "τ",
    "υ", "φ", "χ", "ψ", "ω",
];

pub fn var_name(vi: usize) -> String {
    NAMES
        .get(vi.wrapping_sub(1))
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("v{vi}"))
}

pub fn clause_set_rsx(s: &[Vec<i32>]) -> Element {
    if s.is_empty() {
        return rsx! { span { class: "font-serif", "∅" } };
    }
    rsx! {
        span { class: "font-serif",
            "{{ "
            for (i, c) in s.iter().enumerate() {
                if i > 0 { ", " }
                if c.is_empty() {
                    "□"
                } else {
                    "{{"
                    for (j, l) in c.iter().enumerate() {
                        if j > 0 { ", " }
                        if *l < 0 { "¬" }
                        em { "{var_name(l.unsigned_abs() as usize)}" }
                    }
                    "}}"
                }
            }
            " }}"
        }
    }
}

pub fn cnf_rsx(s: &[Vec<i32>]) -> Element {
    if s.is_empty() {
        return rsx! { span { class: "font-serif", "⊤" } };
    }
    rsx! {
        span { class: "font-serif",
            for (i, c) in s.iter().enumerate() {
                if i > 0 { " ∧ " }
                "("
                {clause_rsx(c)}
                ")"
            }
        }
    }
}

/// Every literal the formula mentions, ordered by variable then polarity, so a
/// literal palette lists ¬α beside α rather than grouping all negations.
pub fn lits_in(formula: &[Vec<i32>]) -> Vec<i32> {
    let mut all: Vec<i32> = formula.iter().flatten().copied().collect();
    all.sort_by_key(|l| (l.unsigned_abs(), *l));
    all.dedup();
    all
}

/// Every variable the formula mentions, 1-based and ascending.
pub fn vars_in(formula: &[Vec<i32>]) -> Vec<usize> {
    let mut a: Vec<usize> = formula
        .iter()
        .flatten()
        .map(|l| l.unsigned_abs() as usize)
        .collect();
    a.sort_unstable();
    a.dedup();
    a
}

/// The highest variable index the formula mentions, which is how many variables
/// a generated exercise needs.
pub fn var_count(formula: &[Vec<i32>]) -> usize {
    formula
        .iter()
        .flatten()
        .map(|l| l.unsigned_abs() as usize)
        .max()
        .unwrap_or(0)
}

/// A 1-based signed literal as a name: `-1` → `¬α`.
pub fn lit_name(l: i32) -> String {
    let name = var_name(l.unsigned_abs() as usize);
    if l < 0 { format!("¬{name}") } else { name }
}

/// A single clause as a disjunction, for views where clause identity matters
/// and the enclosing set braces would only get in the way.
pub fn clause_rsx(c: &[i32]) -> Element {
    if c.is_empty() {
        return rsx! { span { class: "font-serif", "□" } };
    }
    rsx! {
        span { class: "font-serif",
            for (i, l) in c.iter().enumerate() {
                if i > 0 { " ∨ " }
                if *l < 0 { "¬" }
                em { "{var_name(l.unsigned_abs() as usize)}" }
            }
        }
    }
}
