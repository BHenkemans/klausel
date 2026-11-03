use serde::{Deserialize, Serialize};

use crate::algos::dp::parse::parse_trace;
use crate::algos::feedback::{self, RuleText, TraceVerdict};
pub use crate::algos::{Module, Sat};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Rule {
    Unit,
    Pure,
    Elim,
}

impl RuleText for Rule {
    fn name(self) -> &'static str {
        match self {
            Rule::Unit => "unit",
            Rule::Pure => "pure",
            Rule::Elim => "elim",
        }
    }

    fn unapplicable(self) -> &'static str {
        match self {
            Rule::Unit => "Unit propagation doesn't apply — no unit clause with that literal.",
            Rule::Pure => "That literal isn't pure in the current set.",
            Rule::Elim => "That variable doesn't occur in both polarities.",
        }
    }

    fn misapplied(self) -> &'static str {
        match self {
            Rule::Unit => {
                "Right rule, wrong result — keep clauses without the literal, drop its complement from the rest."
            }
            Rule::Pure => {
                "Right rule, wrong result — remove every clause containing the literal, and nothing else."
            }
            Rule::Elim => {
                "Right rule, wrong resolvents — pair each clause with the variable against each one with its negation."
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TraceStep {
    pub rule: Rule,
    pub vi: usize,
    pub neg: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GeneratedFormula {
    pub set: Vec<Vec<i32>>,
}

pub type GradedStep = feedback::GradedStep<Vec<Vec<i32>>, Rule>;

#[derive(Clone, PartialEq, Debug)]
pub struct PracticeHistoryEntry {
    pub rule: Rule,
    pub lit: i32,
    pub verdict: GradedStep,
}

// ---------- Navigation: which submodule is visible ----------
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DpAppState {
    pub module: Module,
}

impl Default for DpAppState {
    fn default() -> Self {
        Self {
            module: Module::Practice,
        }
    }
}

// ---------- Generate module state ----------
#[derive(Clone, PartialEq, Debug)]
pub struct GenerateState {
    pub target: Sat,
    pub steps: Vec<TraceStep>,
    pub max_vars: usize,
    pub max_clauses: usize,
    pub latest_result: Option<Result<GeneratedFormula, String>>,
    pub is_loading: bool,
}

impl Default for GenerateState {
    fn default() -> Self {
        Self {
            target: Sat::Unsat,
            steps: vec![
                TraceStep {
                    rule: Rule::Elim,
                    vi: 1,
                    neg: false,
                },
                TraceStep {
                    rule: Rule::Unit,
                    vi: 2,
                    neg: false,
                },
            ],
            max_vars: 3,
            max_clauses: 5,
            latest_result: None,
            is_loading: false,
        }
    }
}

// ---------- Practice module state ----------
#[derive(Clone, PartialEq, Debug)]
pub struct PracticeState {
    pub exercise: Vec<Vec<i32>>,
    pub history: Vec<PracticeHistoryEntry>,
    pub working: Vec<Vec<i32>>,
    pub draft_rule: Rule,
    pub draft_lit: Option<i32>,
    pub concluded: Option<Sat>,
    pub verdict: Option<TraceVerdict>,
    pub grading: bool,

    // Practice (random exercise) difficulty knobs
    pub practice_max_vars: usize,
    pub practice_max_clauses: usize,
    pub practice_max_steps: usize,
    pub practice_loading: bool,
    pub practice_error: Option<String>,
}

impl Default for PracticeState {
    fn default() -> Self {
        // Example 3.1.12: XOR over α (1), β (2). Always UNSAT under DP.
        let xor: Vec<Vec<i32>> = vec![vec![1, 2], vec![-1, 2], vec![1, -2], vec![-1, -2]];
        Self {
            exercise: xor.clone(),
            history: vec![],
            working: xor,
            draft_rule: Rule::Unit,
            draft_lit: None,
            concluded: None,
            verdict: None,
            grading: false,
            practice_max_vars: 3,
            practice_max_clauses: 5,
            practice_max_steps: 3,
            practice_loading: false,
            practice_error: None,
        }
    }
}

/// Bounds the generator offers. Both are upper limits on the formula it
/// searches for, sized so Z3 answers interactively.
pub const MAX_VARS: usize = 9;
pub const MAX_CLAUSES: usize = 9;

impl GenerateState {
    /// Replace the trace with one parsed from text.
    ///
    /// A Davis–Putnam trace pins its own variable count — every rule names the
    /// variable it acts on — so the variable bound is set to exactly what the
    /// trace needs rather than merely raised. The clause bound is not
    /// recoverable at all: the clause set shrinks as the run proceeds and the
    /// trace never says how wide it started. That one is only touched when an
    /// `# expects: --clauses K` comment states it, and then only upwards.
    pub fn apply_trace(&mut self, text: &str) -> Result<usize, String> {
        let parsed = parse_trace(text).map_err(|e| e.describe())?;

        let needed_vars = parsed.steps.iter().map(|s| s.vi).max().unwrap_or(1);
        if needed_vars > MAX_VARS {
            return Err(format!(
                "this trace ranges over {needed_vars} variables; the generator allows at most {MAX_VARS}"
            ));
        }
        if let Some(k) = parsed.expects_clauses {
            if k > MAX_CLAUSES {
                return Err(format!(
                    "this trace expects {k} clauses; the generator allows at most {MAX_CLAUSES}"
                ));
            }
            self.max_clauses = self.max_clauses.max(k);
        }

        self.max_vars = needed_vars;
        let n = parsed.steps.len();
        self.steps = parsed.steps;
        if let Some(target) = parsed.target {
            self.target = target;
        }
        self.latest_result = None;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trace_pastes_in_and_sizes_its_own_variable_bound() {
        let mut s = GenerateState::default();
        // The highest variable is x3, so four.
        let n = s
            .apply_trace("unit 0\npure 4\nelim 3\nsat\n")
            .expect("parses");

        assert_eq!(n, 3);
        assert_eq!(s.max_vars, 4);
        assert_eq!(s.target, Sat::Sat);
        assert_eq!(
            s.steps
                .iter()
                .map(|x| (x.rule, x.vi, x.neg))
                .collect::<Vec<_>>(),
            [
                (Rule::Unit, 1, false),
                (Rule::Pure, 3, false),
                (Rule::Elim, 4, false)
            ]
        );
    }

    /// The trace fixes the variable count exactly, so a smaller trace shrinks
    /// the bound as well as a larger one growing it.
    #[test]
    fn the_variable_bound_follows_the_trace_in_both_directions() {
        let mut s = GenerateState::default();
        s.apply_trace("elim 6\nsat\n").unwrap();
        assert_eq!(s.max_vars, 7);
        s.apply_trace("elim 0\nsat\n").unwrap();
        assert_eq!(s.max_vars, 1);
    }

    /// The clause bound is not in the trace, so it survives a paste untouched
    /// unless the text states one.
    #[test]
    fn the_clause_bound_is_left_alone_unless_stated() {
        let mut s = GenerateState::default();
        let before = s.max_clauses;
        s.apply_trace("unit 0\nsat\n").unwrap();
        assert_eq!(s.max_clauses, before);

        s.apply_trace("# expects: --clauses 8\nunit 0\nsat\n")
            .unwrap();
        assert_eq!(s.max_clauses, 8);

        // Only upwards: a smaller hint does not shrink a wider design.
        s.apply_trace("# expects: --clauses 2\nunit 0\nsat\n")
            .unwrap();
        assert_eq!(s.max_clauses, 8);
    }

    #[test]
    fn a_trace_beyond_the_limits_is_refused_and_changes_nothing() {
        let mut s = GenerateState::default();
        let before = s.steps.clone();
        let e = s
            .apply_trace(&format!("elim {MAX_VARS}\nsat\n"))
            .unwrap_err();
        assert!(e.contains("variables"), "{e}");
        assert_eq!(s.steps, before);
    }

    #[test]
    fn a_parse_error_is_reported_with_its_line() {
        let mut s = GenerateState::default();
        let e = s.apply_trace("unit 0\nwobble 1\n").unwrap_err();
        assert!(e.starts_with("line 2:"), "{e}");
    }
}
