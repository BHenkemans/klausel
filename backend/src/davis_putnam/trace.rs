use crate::trace::{ParseError, ParsedTrace};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Unit,
    Pure,
    Elim,
}

pub type Step = crate::trace::Step<Rule>;
pub type Trace = crate::trace::Trace<Rule>;

pub fn parse_trace(text: &str) -> Result<ParsedTrace<Rule>, ParseError> {
    crate::trace::parse_trace(text, |keyword| match keyword {
        "unit" => Some(Rule::Unit),
        "pure" => Some(Rule::Pure),
        "elim" => Some(Rule::Elim),
        _ => None,
    })
}
