use super::state::{CdclRule, CdclStep, Sat};
use crate::algos::trace_text::{
    ParseError, parse_clause, parse_expects, parse_literal, split_comment,
};

#[derive(Clone, PartialEq, Debug)]
pub struct ParsedTrace {
    pub steps: Vec<CdclStep>,
    pub target: Option<Sat>,
    pub expects: Option<(usize, usize)>,
}

pub fn parse_trace(text: &str) -> Result<ParsedTrace, ParseError> {
    let mut steps: Vec<CdclStep> = Vec::new();
    let mut target: Option<Sat> = None;
    let mut expects: Option<(usize, usize)> = None;

    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let (line, comment) = split_comment(raw);
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
                CdclStep {
                    rule: CdclRule::Decide,
                    vi,
                    neg,
                    ci: 0,
                }
            }
            "unit" | "conflict" => {
                let tok = tokens
                    .next()
                    .ok_or_else(|| ParseError::at(line_no, format!("{head} needs a clause")))?;
                let ci = parse_clause(tok).ok_or_else(|| {
                    ParseError::at(line_no, format!("`{tok}` is not a clause index"))
                })?;
                let rule = if head == "unit" {
                    CdclRule::Unit
                } else {
                    CdclRule::Conflict
                };
                CdclStep {
                    rule,
                    vi: 1,
                    neg: false,
                    ci,
                }
            }
            other => {
                return Err(ParseError::at(
                    line_no,
                    format!("unknown rule `{other}` — expected decide, unit or conflict"),
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

    #[test]
    fn reads_a_backend_example_verbatim() {
        let text = "\
# expects: --vars 4 --clauses 4
decide 0
decide 2
decide 4
unit 0
conflict 1
unit 4
decide 2
sat
";
        let p = parse_trace(text).expect("parses");
        use CdclRule::{Conflict, Decide, Unit};
        assert_eq!(
            p.steps.iter().map(|s| s.rule).collect::<Vec<_>>(),
            [Decide, Decide, Decide, Unit, Conflict, Unit, Decide]
        );
        assert_eq!(p.steps[4].ci, 1);
        assert_eq!(p.steps[5].ci, 4);
        assert_eq!(p.target, Some(Sat::Sat));
        assert_eq!(p.expects, Some((4, 4)));
    }

    #[test]
    fn conflict_needs_the_clause_it_found_falsified() {
        let e = parse_trace("decide 0\nconflict\n").unwrap_err();
        assert_eq!(e.describe(), "line 2: conflict needs a clause");
    }

    #[test]
    fn backtrack_is_not_a_cdcl_rule() {
        let e = parse_trace("decide 0\nbacktrack\n").unwrap_err();
        assert!(
            e.describe().starts_with("line 2: unknown rule `backtrack`"),
            "{}",
            e.describe()
        );
    }
}
