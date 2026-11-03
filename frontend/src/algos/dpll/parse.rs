//! Reading a designed DPLL trace back out of text.
//!
//! The syntax is the one `bin/dpll.rs` already parses, so a trace file
//! round-trips into the designer unchanged.

use super::state::{DpllRule, DpllStep, Sat};
use crate::algos::trace_text::{
    ParseError, parse_clause, parse_expects, parse_literal, split_comment,
};

#[derive(Clone, PartialEq, Debug)]
pub struct ParsedTrace {
    pub steps: Vec<DpllStep>,
    /// `sat` / `unsat` line, if the text carried one.
    pub target: Option<Sat>,
    /// `# expects: --vars L --clauses K`, the bounds the example files record
    /// alongside their traces. A DPLL trace determines neither — unit
    /// propagation names a clause, never the literal it derives — so when the
    /// text states them they are worth more than anything we can infer.
    pub expects: Option<(usize, usize)>,
}

pub fn parse_trace(text: &str) -> Result<ParsedTrace, ParseError> {
    let mut steps: Vec<DpllStep> = Vec::new();
    let mut target: Option<Sat> = None;
    let mut expects: Option<(usize, usize)> = None;

    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let (line, comment) = split_comment(raw);
        // Both bounds have to be named: neither is inferable from a DPLL
        // trace, so a half-stated hint is no better than none.
        if let Some((Some(v), Some(c))) =
            comment.and_then(parse_expects).map(|e| (e.vars, e.clauses))
        {
            expects = Some((v, c));
        }
        if line.is_empty() {
            continue;
        }

        let mut tokens = line.split_whitespace();
        let head = tokens.next().expect("non-empty line has a first token");

        // The result line closes the trace, exactly as in the .trace files.
        if let "sat" | "unsat" = head {
            if target.is_some() {
                return Err(ParseError::at(line_no, "the result is given twice"));
            }
            target = Some(if head == "sat" { Sat::Sat } else { Sat::Unsat });
            continue;
        }
        if target.is_some() {
            return Err(ParseError::at(
                line_no,
                "steps cannot follow the result line",
            ));
        }

        let step = match head {
            "decide" => {
                let tok = tokens
                    .next()
                    .ok_or_else(|| ParseError::at(line_no, "decide needs a literal"))?;
                let (vi, neg) = parse_literal(tok)
                    .ok_or_else(|| ParseError::at(line_no, format!("`{tok}` is not a literal")))?;
                DpllStep {
                    rule: DpllRule::Decide,
                    vi,
                    neg,
                    ci: 0,
                }
            }
            "unit" | "backtrack" => {
                let tok = tokens
                    .next()
                    .ok_or_else(|| ParseError::at(line_no, format!("{head} needs a clause")))?;
                let ci = parse_clause(tok).ok_or_else(|| {
                    ParseError::at(line_no, format!("`{tok}` is not a clause index"))
                })?;
                let rule = if head == "unit" {
                    DpllRule::Unit
                } else {
                    DpllRule::Backtrack
                };
                DpllStep {
                    rule,
                    vi: 1,
                    neg: false,
                    ci,
                }
            }
            other => {
                return Err(ParseError::at(
                    line_no,
                    format!("unknown rule `{other}` — expected decide, unit or backtrack"),
                ));
            }
        };

        if tokens.next().is_some() {
            return Err(ParseError::at(line_no, "trailing tokens after the step"));
        }
        steps.push(step);
    }

    if steps.is_empty() {
        return Err(ParseError::at(0, "no steps found"));
    }
    Ok(ParsedTrace {
        steps,
        target,
        expects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(p: &ParsedTrace) -> Vec<DpllRule> {
        p.steps.iter().map(|s| s.rule).collect()
    }

    #[test]
    fn reads_a_backend_example_verbatim() {
        let text = "\
# Backtrack a decision that is not the first one.
# expects: --vars 3 --clauses 4
decide 0
decide 2
unit 0
backtrack 1
sat
";
        let p = parse_trace(text).expect("parses");
        use DpllRule::{Backtrack, Decide, Unit};
        assert_eq!(rules(&p), [Decide, Decide, Unit, Backtrack]);
        assert_eq!(p.target, Some(Sat::Sat));
        assert_eq!(p.expects, Some((3, 4)));
        // `decide 0` is x0 and `decide 2` is x1, both positive.
        assert_eq!(p.steps[0].lit(), 1);
        assert_eq!(p.steps[1].lit(), 2);
        assert_eq!(p.steps[2].ci, 0);
        assert_eq!(p.steps[3].ci, 1);
    }

    #[test]
    fn the_literal_index_encodes_polarity() {
        let p = parse_trace("decide 0\ndecide 1\ndecide 5\nsat\n").unwrap();
        // 2*var + 1 is the negative literal.
        assert_eq!(
            p.steps.iter().map(|s| s.lit()).collect::<Vec<_>>(),
            [1, -1, -3]
        );
    }

    #[test]
    fn literals_may_also_be_written_the_way_the_backend_prints_them() {
        let p = parse_trace("decide x2\ndecide !x0\nunit c3\nsat\n").unwrap();
        assert_eq!(p.steps[0].lit(), 3);
        assert_eq!(p.steps[1].lit(), -1);
        assert_eq!(p.steps[2].ci, 3);
    }

    #[test]
    fn a_trace_without_an_expects_comment_states_no_bounds() {
        let p = parse_trace("# just a note\ndecide 0\nsat\n").unwrap();
        assert_eq!(p.expects, None);
    }

    #[test]
    fn the_result_line_is_optional() {
        let p = parse_trace("unit 0\nbacktrack 1\n").unwrap();
        assert_eq!(p.target, None);
        assert_eq!(p.steps.len(), 2);
    }

    #[test]
    fn errors_point_at_the_offending_line() {
        let e = parse_trace("decide 0\nunti 1\nsat\n").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(
            e.describe().starts_with("line 2: unknown rule `unti`"),
            "{}",
            e.describe()
        );

        let e = parse_trace("decide\n").unwrap_err();
        assert_eq!(e.describe(), "line 1: decide needs a literal");

        let e = parse_trace("sat\ndecide 0\n").unwrap_err();
        assert_eq!(e.line, 2);

        let e = parse_trace("# nothing but a comment\n").unwrap_err();
        assert_eq!(e.message, "no steps found");
    }
}
