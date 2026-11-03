//! Reading a designed Davis–Putnam trace back out of text.
//!
//! The syntax is the one `bin/dp.rs` parses, so a trace file round-trips into
//! the designer unchanged:
//!
//! ```text
//! unit 0     # 2·var + polarity — the literal the unit clause holds
//! pure 4
//! elim 3     # a bare variable: elimination has no polarity
//! sat
//! ```

use super::state::{Rule, Sat, TraceStep};
use crate::algos::trace_text::{
    ParseError, parse_expects, parse_literal, parse_var, split_comment,
};

#[derive(Clone, PartialEq, Debug)]
pub struct ParsedTrace {
    pub steps: Vec<TraceStep>,
    /// `sat` / `unsat` line, if the text carried one.
    pub target: Option<Sat>,
    /// `# expects: --clauses K`. Unlike DPLL, a Davis–Putnam trace fixes its
    /// own variable count — every rule names a variable outright — so only the
    /// clause bound is ever worth stating.
    pub expects_clauses: Option<usize>,
}

pub fn parse_trace(text: &str) -> Result<ParsedTrace, ParseError> {
    let mut steps: Vec<TraceStep> = Vec::new();
    let mut target: Option<Sat> = None;
    let mut expects_clauses: Option<usize> = None;

    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let (line, comment) = split_comment(raw);
        if let Some(found) = comment.and_then(parse_expects) {
            expects_clauses = found.clauses.or(expects_clauses);
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

        let rule = match head {
            "unit" => Rule::Unit,
            "pure" => Rule::Pure,
            "elim" => Rule::Elim,
            other => {
                return Err(ParseError::at(
                    line_no,
                    format!("unknown rule `{other}` — expected unit, pure or elim"),
                ));
            }
        };

        let tok = tokens
            .next()
            .ok_or_else(|| ParseError::at(line_no, format!("{head} needs a variable")))?;

        // Unit and pure fire on a literal; elimination resolves a variable
        // away in both polarities at once, so a sign there is meaningless.
        let (vi, neg) = match rule {
            Rule::Elim => {
                let vi = parse_var(tok).ok_or_else(|| {
                    ParseError::at(
                        line_no,
                        format!("`{tok}` is not a variable — elim takes no polarity"),
                    )
                })?;
                (vi, false)
            }
            Rule::Unit | Rule::Pure => parse_literal(tok)
                .ok_or_else(|| ParseError::at(line_no, format!("`{tok}` is not a literal")))?,
        };

        if tokens.next().is_some() {
            return Err(ParseError::at(line_no, "trailing tokens after the step"));
        }
        steps.push(TraceStep { rule, vi, neg });
    }

    if steps.is_empty() {
        return Err(ParseError::at(0, "no steps found"));
    }
    Ok(ParsedTrace {
        steps,
        target,
        expects_clauses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_backend_example_verbatim() {
        let p = parse_trace("unit 0\npure 4\nelim 3\nsat\n").expect("parses");
        assert_eq!(
            p.steps.iter().map(|s| s.rule).collect::<Vec<_>>(),
            [Rule::Unit, Rule::Pure, Rule::Elim]
        );
        // `unit 0` is x0, `pure 4` is x2, `elim 3` is the *variable* x3.
        assert_eq!(
            p.steps.iter().map(|s| (s.vi, s.neg)).collect::<Vec<_>>(),
            [(1, false), (3, false), (4, false)]
        );
        assert_eq!(p.target, Some(Sat::Sat));
    }

    #[test]
    fn odd_literal_indices_are_negative() {
        let p = parse_trace("unit 1\npure 5\nunsat\n").unwrap();
        assert_eq!(
            p.steps.iter().map(|s| (s.vi, s.neg)).collect::<Vec<_>>(),
            [(1, true), (3, true)]
        );
        assert_eq!(p.target, Some(Sat::Unsat));
    }

    #[test]
    fn literals_may_also_be_written_the_way_the_backend_prints_them() {
        let p = parse_trace("unit !x1\npure x0\nelim x2\nsat\n").unwrap();
        assert_eq!(
            p.steps.iter().map(|s| (s.vi, s.neg)).collect::<Vec<_>>(),
            [(2, true), (1, false), (3, false)]
        );
    }

    /// Elimination removes a variable in both polarities, so a signed argument
    /// is a mistake worth naming rather than silently dropping.
    #[test]
    fn elim_refuses_a_signed_argument() {
        let e = parse_trace("elim !x1\nsat\n").unwrap_err();
        assert!(e.message.contains("takes no polarity"), "{}", e.message);
    }

    #[test]
    fn the_clause_bound_can_be_stated_in_a_comment() {
        let p = parse_trace("# expects: --clauses 6\nunit 0\nsat\n").unwrap();
        assert_eq!(p.expects_clauses, Some(6));

        let p = parse_trace("unit 0\nsat\n").unwrap();
        assert_eq!(p.expects_clauses, None);
    }

    #[test]
    fn errors_point_at_the_offending_line() {
        let e = parse_trace("unit 0\nresolve 1\nsat\n").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(
            e.describe().starts_with("line 2: unknown rule `resolve`"),
            "{}",
            e.describe()
        );

        assert_eq!(
            parse_trace("pure\n").unwrap_err().describe(),
            "line 1: pure needs a variable"
        );
        assert_eq!(parse_trace("sat\nunit 0\n").unwrap_err().line, 2);
        assert_eq!(
            parse_trace("# only a comment\n").unwrap_err().message,
            "no steps found"
        );
    }
}
