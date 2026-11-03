#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step<R> {
    pub rule: R,
    pub param: usize,
}

pub type Trace<R> = Vec<Step<R>>;

pub struct ParsedTrace<R> {
    pub trace: Trace<R>,
    pub sat: bool,
}

#[derive(Debug)]
pub enum ParseError {
    UnknownRule { line: usize, token: String },
    MissingParam { line: usize },
    BadParam { line: usize, token: String },
    ExtraTokens { line: usize },
    MissingResult,
    ResultNotLast { line: usize },
}

pub fn parse_trace<R>(
    text: &str,
    rule: impl Fn(&str) -> Option<R>,
) -> Result<ParsedTrace<R>, ParseError> {
    let mut out: Trace<R> = Vec::new();
    let mut sat: Option<bool> = None;

    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if sat.is_some() {
            return Err(ParseError::ResultNotLast { line: line_no });
        }

        let mut tokens = line.split_whitespace();
        let head = tokens
            .next()
            .ok_or(ParseError::MissingParam { line: line_no })?;

        if head == "sat" || head == "unsat" {
            if tokens.next().is_some() {
                return Err(ParseError::ExtraTokens { line: line_no });
            }
            sat = Some(head == "sat");
            continue;
        }

        let rule = rule(head).ok_or_else(|| ParseError::UnknownRule {
            line: line_no,
            token: head.to_string(),
        })?;

        let param_token = tokens
            .next()
            .ok_or(ParseError::MissingParam { line: line_no })?;
        let param: usize = param_token.parse().map_err(|_| ParseError::BadParam {
            line: line_no,
            token: param_token.to_string(),
        })?;

        if tokens.next().is_some() {
            return Err(ParseError::ExtraTokens { line: line_no });
        }

        out.push(Step { rule, param });
    }

    let sat = sat.ok_or(ParseError::MissingResult)?;
    Ok(ParsedTrace { trace: out, sat })
}
