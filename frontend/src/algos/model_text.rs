//! Parsers for the textual dumps the backend's Z3 models produce.
//!
//! Both `davis_putnam::Model` and `dpll::Model` render a clause set as
//! `  c{i}: !x0 v x2` lines (or `[]` for the empty clause), so the clause
//! parser is shared. Clause indices are positional: line `c{i}` becomes entry
//! `i` of the returned vector, which the DPLL views rely on — a `unit` step
//! names a clause by that index.

#![cfg(feature = "server")]

/// `  c0: !x0 v x2` → `[[-1, 3]]`, 1-based signed literals.
pub fn parse_model_text(text: &str) -> Vec<Vec<i32>> {
    let mut clauses: Vec<Vec<i32>> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        let Some(colon) = line.find(':') else {
            continue;
        };
        let body = line[colon + 1..].trim();
        if body == "[]" {
            clauses.push(Vec::new());
            continue;
        }
        clauses.push(body.split(" v ").filter_map(parse_lit).collect());
    }
    clauses
}

/// `!x0` → `-1`, `x2` → `3`. `None` when the token is not a literal.
fn parse_lit(tok: &str) -> Option<i32> {
    let tok = tok.trim();
    let (neg, rest) = match tok.strip_prefix('!') {
        Some(s) => (true, s),
        None => (false, tok),
    };
    let var: i32 = rest.strip_prefix('x')?.parse().ok()?;
    Some(if neg { -(var + 1) } else { var + 1 })
}

/// One entry of a trail, as rendered by `dpll::Model::format_trail` or
/// `cdcl::Model::format_trail`.
pub struct TrailSlot {
    pub lit: i32,
    pub decision: bool,
    pub antecedent: Option<usize>,
}

/// `!x0* x1(c2)` → two slots, the first a decision, the second propagated by
/// clause 2. `[]` → empty trail.
pub fn parse_trail_text(text: &str) -> Vec<TrailSlot> {
    let text = text.trim();
    if text == "[]" {
        return Vec::new();
    }
    text.split_whitespace()
        .filter_map(|tok| {
            let (tok, antecedent) = match tok.strip_suffix(')').and_then(|t| t.split_once("(c")) {
                Some((lit, clause)) => (lit, Some(clause.parse().ok()?)),
                None => (tok, None),
            };
            let decision = tok.ends_with('*');
            let lit = parse_lit(tok.trim_end_matches('*'))?;
            Some(TrailSlot {
                lit,
                decision,
                antecedent,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim `dpll::Model::format_final_model` output.
    const CLAUSES: &str =
        "  c0: !x0 v !x1 v x2\n  c1: !x1 v !x2\n  c2: x0 v !x2\n  c3: x0 v !x1 v !x2\n";

    #[test]
    fn clause_indices_are_positional() {
        let set = parse_model_text(CLAUSES);
        assert_eq!(
            set,
            vec![vec![-1, -2, 3], vec![-2, -3], vec![1, -3], vec![1, -2, -3],]
        );
    }

    #[test]
    fn the_empty_clause_is_an_empty_vec() {
        assert_eq!(
            parse_model_text("  c0: []\n  c1: x0\n"),
            vec![vec![], vec![1]]
        );
    }

    #[test]
    fn a_trail_marks_its_decisions() {
        // `t4: x0* !x1` — slot 0 decided, slot 1 flipped by the backtrack.
        let slots = parse_trail_text("x0* !x1");
        assert_eq!(
            slots
                .iter()
                .map(|s| (s.lit, s.decision))
                .collect::<Vec<_>>(),
            [(1, true), (-2, false)]
        );
    }

    #[test]
    fn a_propagated_slot_carries_its_antecedent() {
        let slots = parse_trail_text("x0* x1* !x2(c4) x3(c0)");
        assert_eq!(
            slots
                .iter()
                .map(|s| (s.lit, s.decision, s.antecedent))
                .collect::<Vec<_>>(),
            [
                (1, true, None),
                (2, true, None),
                (-3, false, Some(4)),
                (4, false, Some(0))
            ]
        );
    }

    #[test]
    fn an_empty_trail_has_no_slots() {
        assert!(parse_trail_text("[]").is_empty());
    }
}
