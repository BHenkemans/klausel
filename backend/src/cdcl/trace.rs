use crate::trace::{ParseError, ParsedTrace};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Conflict,
    UnitProp,
    Decide,
}

pub type Step = crate::trace::Step<Rule>;
pub type Trace = crate::trace::Trace<Rule>;

pub fn parse_trace(text: &str) -> Result<ParsedTrace<Rule>, ParseError> {
    crate::trace::parse_trace(text, |keyword| match keyword {
        "conflict" => Some(Rule::Conflict),
        "unit" => Some(Rule::UnitProp),
        "decide" => Some(Rule::Decide),
        _ => None,
    })
}
