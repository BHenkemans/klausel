//! Pieces shared by the two trace parsers.
//!
//! Davis–Putnam and DPLL traces are written the same way — one rule per line,
//! a `sat`/`unsat` result, `#` comments — and differ only in which rules exist
//! and what their parameters mean. What is common is the literal notation and
//! the `expects:` comment the example files carry.

#[derive(Clone, PartialEq, Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl ParseError {
    pub fn at(line: usize, message: impl Into<String>) -> Self {
        ParseError {
            line,
            message: message.into(),
        }
    }

    pub fn describe(&self) -> String {
        format!("line {}: {}", self.line, self.message)
    }
}

/// Split a line into its content and its comment, so a parser can read both.
pub fn split_comment(raw: &str) -> (&str, Option<&str>) {
    match raw.split_once('#') {
        Some((body, comment)) => (body.trim(), Some(comment)),
        None => (raw.trim(), None),
    }
}

/// A literal: `4` (the backends' 2·var+polarity index) or `x2` / `!x2` (how
/// they print one). Returns a 1-based variable and its polarity.
pub fn parse_literal(tok: &str) -> Option<(usize, bool)> {
    if let Ok(index) = tok.parse::<usize>() {
        return Some((index / 2 + 1, !index.is_multiple_of(2)));
    }
    let (neg, rest) = match tok.strip_prefix('!') {
        Some(r) => (true, r),
        None => (false, tok),
    };
    let var0: usize = rest.strip_prefix('x')?.parse().ok()?;
    Some((var0 + 1, neg))
}

/// A bare variable: `3` or `x3`, both 0-based as the backends index them.
/// Returns a 1-based variable.
pub fn parse_var(tok: &str) -> Option<usize> {
    let rest = tok.strip_prefix('x').unwrap_or(tok);
    rest.parse::<usize>().ok().map(|v| v + 1)
}

/// A clause index: `2` or `c2`. Positional, so it needs no rebasing.
pub fn parse_clause(tok: &str) -> Option<usize> {
    tok.strip_prefix('c').unwrap_or(tok).parse().ok()
}

/// ` expects: --vars 3 --clauses 4` → the flags that were given. Either may be
/// absent: a Davis–Putnam trace determines its own variable count, so those
/// files only ever need to state the clause bound.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Expects {
    pub vars: Option<usize>,
    pub clauses: Option<usize>,
}

pub fn parse_expects(comment: &str) -> Option<Expects> {
    let rest = comment.trim().strip_prefix("expects:")?;
    let mut tokens = rest.split_whitespace();
    let mut out = Expects::default();
    while let Some(flag) = tokens.next() {
        match flag {
            "--vars" => out.vars = tokens.next()?.parse().ok(),
            "--clauses" => out.clauses = tokens.next()?.parse().ok(),
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_literal_index_encodes_polarity() {
        assert_eq!(parse_literal("0"), Some((1, false)));
        assert_eq!(parse_literal("1"), Some((1, true)));
        assert_eq!(parse_literal("5"), Some((3, true)));
    }

    #[test]
    fn a_literal_may_be_written_the_way_the_backends_print_it() {
        assert_eq!(parse_literal("x2"), Some((3, false)));
        assert_eq!(parse_literal("!x0"), Some((1, true)));
        assert_eq!(parse_literal("nonsense"), None);
    }

    #[test]
    fn a_variable_carries_no_polarity() {
        assert_eq!(parse_var("3"), Some(4));
        assert_eq!(parse_var("x3"), Some(4));
        assert_eq!(parse_var("!x3"), None);
    }

    #[test]
    fn expects_flags_are_each_optional() {
        assert_eq!(
            parse_expects(" expects: --vars 3 --clauses 4"),
            Some(Expects {
                vars: Some(3),
                clauses: Some(4)
            })
        );
        assert_eq!(
            parse_expects(" expects: --clauses 6"),
            Some(Expects {
                vars: None,
                clauses: Some(6)
            })
        );
        assert_eq!(parse_expects(" just a note"), None);
        assert_eq!(parse_expects(" expects: --wat 2"), None);
    }

    #[test]
    fn a_comment_is_split_from_its_line() {
        assert_eq!(split_comment("unit 0  # note"), ("unit 0", Some(" note")));
        assert_eq!(split_comment("  unit 0 "), ("unit 0", None));
        assert_eq!(split_comment("# whole line"), ("", Some(" whole line")));
    }
}
