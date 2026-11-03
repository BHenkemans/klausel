use crate::algos::cdcl::parse::parse_trace;
use crate::algos::cdcl::trail::preview;
use crate::algos::controls::MIN_LIMIT;
use crate::algos::feedback::{GradedStep, RuleText, TraceVerdict};
pub use crate::algos::{Module, Sat};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum CdclRule {
    Decide,
    Unit,
    Conflict,
}

impl RuleText for CdclRule {
    fn name(self) -> &'static str {
        match self {
            CdclRule::Conflict => "conflict",
            CdclRule::Unit => "unit",
            CdclRule::Decide => "decide",
        }
    }

    fn unapplicable(self) -> &'static str {
        match self {
            CdclRule::Decide => {
                "That literal is already assigned on the trail, or the formula is already satisfied."
            }
            CdclRule::Unit => {
                "That clause is not unit under the current trail — it has none or several literals left."
            }
            CdclRule::Conflict => {
                "That clause is not in conflict, or there is no decision left to backjump over."
            }
        }
    }

    fn misapplied(self) -> &'static str {
        match self {
            CdclRule::Decide => {
                "Right rule, wrong state — append the literal as a decision (click it in the trail until it turns solid) and change nothing else."
            }
            CdclRule::Unit => {
                "Right rule, wrong state — append the clause's one surviving literal as a propagation."
            }
            CdclRule::Conflict => {
                "Right rule, wrong state — learn the negated decisions and earlier-level literals the conflict traces back to, then keep only the trail up to the highest earlier level among them."
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CdclStep {
    pub rule: CdclRule,
    pub vi: usize,
    pub neg: bool,
    pub ci: usize,
}

impl CdclStep {
    pub fn lit(&self) -> i32 {
        if self.neg {
            -(self.vi as i32)
        } else {
            self.vi as i32
        }
    }
}

pub fn learned_before(steps: &[CdclStep], idx: usize) -> usize {
    steps
        .iter()
        .take(idx)
        .filter(|s| s.rule == CdclRule::Conflict)
        .count()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TrailEntry {
    pub lit: i32,
    pub decision: bool,
    pub antecedent: Option<usize>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GeneratedExercise {
    pub set: Vec<Vec<i32>>,
    pub learned: Vec<Vec<i32>>,
    pub trail: Vec<Vec<TrailEntry>>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum GenerateOutcome {
    Found(GeneratedExercise),
    Impossible,
    TimedOut,
}

#[derive(Clone, PartialEq, Debug)]
pub struct CdclGenerateState {
    pub target: Sat,
    pub steps: Vec<CdclStep>,
    pub vars: usize,
    pub clauses: usize,
    pub latest_result: Option<Result<GeneratedExercise, String>>,
    pub is_loading: bool,
}

impl Default for CdclGenerateState {
    fn default() -> Self {
        Self {
            target: Sat::Sat,
            steps: vec![
                CdclStep {
                    rule: CdclRule::Decide,
                    vi: 1,
                    neg: false,
                    ci: 0,
                },
                CdclStep {
                    rule: CdclRule::Unit,
                    vi: 1,
                    neg: false,
                    ci: 0,
                },
                CdclStep {
                    rule: CdclRule::Conflict,
                    vi: 1,
                    neg: false,
                    ci: 1,
                },
                CdclStep {
                    rule: CdclRule::Unit,
                    vi: 1,
                    neg: false,
                    ci: 2,
                },
            ],
            vars: 2,
            clauses: 2,
            latest_result: None,
            is_loading: false,
        }
    }
}

pub const MAX_VARS: usize = 8;
pub const MAX_CLAUSES: usize = 9;

impl CdclGenerateState {
    fn edited(&mut self) {
        self.latest_result = None;
    }

    pub fn available_clauses(&self, idx: usize) -> usize {
        self.clauses + learned_before(&self.steps, idx)
    }

    fn clamp_clauses(&mut self) {
        let mut learned = 0;
        for s in &mut self.steps {
            s.ci = s.ci.min(self.clauses + learned - 1);
            if s.rule == CdclRule::Conflict {
                learned += 1;
            }
        }
    }

    pub fn set_target(&mut self, target: Sat) {
        self.target = target;
        self.edited();
    }

    pub fn set_rule(&mut self, idx: usize, rule: CdclRule) {
        if let Some(s) = self.steps.get_mut(idx) {
            s.rule = rule;
            self.edited();
        }
    }

    pub fn toggle_neg(&mut self, idx: usize) {
        if let Some(s) = self.steps.get_mut(idx) {
            s.neg = !s.neg;
            self.edited();
        }
    }

    pub fn set_var(&mut self, idx: usize, vi: usize) {
        let vi = vi.clamp(1, self.vars);
        if let Some(s) = self.steps.get_mut(idx) {
            s.vi = vi;
            self.edited();
        }
    }

    pub fn set_clause(&mut self, idx: usize, ci: usize) {
        let ci = ci.min(self.available_clauses(idx) - 1);
        if let Some(s) = self.steps.get_mut(idx) {
            s.ci = ci;
            self.edited();
        }
    }

    pub fn push_rule(&mut self, rule: CdclRule) {
        let after_conflict = self
            .steps
            .last()
            .is_some_and(|s| s.rule == CdclRule::Conflict);
        let newest = self.available_clauses(self.steps.len()) - 1;
        let ci = if after_conflict {
            newest
        } else {
            self.steps.last().map_or(0, |s| s.ci.min(newest))
        };
        self.steps.push(CdclStep {
            rule,
            vi: 1,
            neg: false,
            ci,
        });
        self.edited();
    }

    pub fn insert_after(&mut self, idx: usize) {
        if let Some(&step) = self.steps.get(idx) {
            self.steps.insert(idx + 1, step);
            self.edited();
        }
    }

    pub fn remove_step(&mut self, idx: usize) {
        if idx < self.steps.len() {
            self.steps.remove(idx);
            self.edited();
        }
    }

    pub fn clear_steps(&mut self) {
        self.steps.clear();
        self.edited();
    }

    pub fn inc_vars(&mut self) {
        if self.vars < MAX_VARS {
            self.vars += 1;
            self.edited();
        }
    }

    pub fn dec_vars(&mut self) {
        if self.vars > MIN_LIMIT {
            self.vars -= 1;
            for s in &mut self.steps {
                s.vi = s.vi.min(self.vars);
            }
            self.edited();
        }
    }

    pub fn inc_clauses(&mut self) {
        if self.clauses < MAX_CLAUSES {
            self.clauses += 1;
            self.edited();
        }
    }

    pub fn dec_clauses(&mut self) {
        if self.clauses > MIN_LIMIT {
            self.clauses -= 1;
            self.clamp_clauses();
            self.edited();
        }
    }

    pub fn apply_trace(&mut self, text: &str) -> Result<usize, String> {
        let parsed = parse_trace(text).map_err(|e| e.describe())?;

        let named_vars = parsed.steps.iter().map(|s| s.vi).max().unwrap_or(1);
        let named_clauses = parsed
            .steps
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s.rule, CdclRule::Unit | CdclRule::Conflict))
            .map(|(i, s)| (s.ci + 1).saturating_sub(learned_before(&parsed.steps, i)))
            .max()
            .unwrap_or(1)
            .max(1);
        let depth = preview(&parsed.steps, usize::MAX, usize::MAX, Sat::Sat).peak_depth;

        let (hinted_vars, hinted_clauses) = parsed.expects.unwrap_or((0, 0));
        let needed_vars = named_vars.max(depth).max(hinted_vars);
        let needed_clauses = named_clauses.max(hinted_clauses);
        if needed_vars > MAX_VARS {
            return Err(format!(
                "this trace needs {needed_vars} variables; the designer allows at most {MAX_VARS}"
            ));
        }
        if needed_clauses > MAX_CLAUSES {
            return Err(format!(
                "this trace needs {needed_clauses} original clauses; the designer allows at most {MAX_CLAUSES}"
            ));
        }

        self.vars = self.vars.max(needed_vars);
        self.clauses = self.clauses.max(needed_clauses);
        let n = parsed.steps.len();
        self.steps = parsed.steps;
        if let Some(target) = parsed.target {
            self.target = target;
        }
        self.edited();
        Ok(n)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CdclAppState {
    pub module: Module,
}

impl Default for CdclAppState {
    fn default() -> Self {
        Self {
            module: Module::Practice,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct CdclState {
    pub phi: Vec<Vec<i32>>,
    pub trail: Vec<TrailEntry>,
}

pub type CdclGradedStep = GradedStep<CdclState, CdclRule>;

#[derive(Clone, PartialEq, Debug)]
pub struct CdclPracticeHistoryEntry {
    pub step: CdclStep,
    pub verdict: CdclGradedStep,
}

#[derive(Clone, PartialEq, Debug)]
pub struct CdclPracticeState {
    pub formula: Vec<Vec<i32>>,
    pub history: Vec<CdclPracticeHistoryEntry>,
    pub working: Vec<TrailEntry>,
    pub working_learned: Vec<i32>,
    pub draft_rule: CdclRule,
    pub draft_lit: Option<i32>,
    pub draft_ci: Option<usize>,
    pub concluded: Option<Sat>,
    pub verdict: Option<TraceVerdict>,
    pub grading: bool,

    pub practice_max_vars: usize,
    pub practice_max_clauses: usize,
    pub practice_max_steps: usize,
    pub practice_loading: bool,
    pub practice_error: Option<String>,
}

impl Default for CdclPracticeState {
    fn default() -> Self {
        let xor: Vec<Vec<i32>> = vec![vec![1, 2], vec![-1, 2], vec![1, -2], vec![-1, -2]];
        Self {
            formula: xor,
            history: vec![],
            working: vec![],
            working_learned: vec![],
            draft_rule: CdclRule::Decide,
            draft_lit: None,
            draft_ci: None,
            concluded: None,
            verdict: None,
            grading: false,
            practice_max_vars: 3,
            practice_max_clauses: 5,
            practice_max_steps: 5,
            practice_loading: false,
            practice_error: None,
        }
    }
}

impl CdclPracticeState {
    pub fn current_state(&self) -> CdclState {
        self.history.last().map_or_else(
            || CdclState {
                phi: self.formula.clone(),
                trail: vec![],
            },
            |h| h.verdict.claimed.clone(),
        )
    }

    pub fn claimed_state(&self) -> CdclState {
        let mut phi = self.current_state().phi;
        if self.draft_rule == CdclRule::Conflict {
            phi.push(self.working_learned.clone());
        }
        CdclState {
            phi,
            trail: self.working.clone(),
        }
    }

    pub fn toggle_learned(&mut self, l: i32) {
        match self.working_learned.iter().position(|&x| x == l) {
            Some(i) => {
                self.working_learned.remove(i);
            }
            None => {
                self.working_learned.push(l);
                self.working_learned.sort_by_key(|x| (x.unsigned_abs(), *x));
            }
        }
    }

    pub fn reset_working(&mut self) {
        self.working = self.current_state().trail;
        self.working_learned.clear();
        self.draft_lit = None;
        self.draft_ci = None;
    }

    pub fn start_over(&mut self) {
        self.history.clear();
        self.concluded = None;
        self.draft_rule = CdclRule::Decide;
        self.reset_working();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CdclRule::{Conflict, Decide, Unit};

    fn step(rule: CdclRule, ci: usize) -> CdclStep {
        CdclStep {
            rule,
            vi: 1,
            neg: false,
            ci,
        }
    }

    #[test]
    fn a_learned_clause_becomes_addressable_after_its_conflict() {
        let mut s = CdclGenerateState::default();
        s.clear_steps();
        s.push_rule(Decide);
        s.push_rule(Unit);
        s.set_clause(1, 5);
        assert_eq!(s.steps[1].ci, 1, "before any conflict only c0, c1 exist");

        s.push_rule(Conflict);
        s.set_clause(2, 1);
        s.push_rule(Unit);
        assert_eq!(
            s.steps[3].ci, 2,
            "a unit right after a conflict defaults to the new clause"
        );
        s.set_clause(3, 5);
        assert_eq!(s.steps[3].ci, 2);
    }

    #[test]
    fn lowering_the_clause_bound_keeps_learned_references_inside_it() {
        let mut s = CdclGenerateState::default();
        s.inc_clauses();
        s.steps = vec![
            step(Decide, 0),
            step(Unit, 2),
            step(Conflict, 2),
            step(Unit, 3),
        ];
        s.dec_clauses();
        assert_eq!(
            s.steps.iter().map(|x| x.ci).collect::<Vec<_>>(),
            [0, 1, 1, 2]
        );
    }

    #[test]
    fn a_pasted_trace_counts_learned_slots_out_of_the_clause_bound() {
        let mut s = CdclGenerateState::default();
        s.apply_trace("decide 0\nunit 0\nconflict 1\nunit 2\nsat\n")
            .unwrap();
        assert_eq!(
            s.clauses, 2,
            "c2 is the learned clause, not an original one"
        );
        assert_eq!(s.target, Sat::Sat);
    }

    #[test]
    fn a_pasted_trace_fits_the_variable_bound_to_its_shallowest_trail() {
        let mut s = CdclGenerateState::default();
        s.apply_trace("decide 0\nunit 0\nunit 1\nunit 2\nsat\n")
            .unwrap();
        assert_eq!(s.vars, 4);
    }

    #[test]
    fn a_conflict_step_adds_the_learned_clause_to_the_claim() {
        let mut p = CdclPracticeState {
            draft_rule: Conflict,
            ..CdclPracticeState::default()
        };
        p.toggle_learned(-1);
        p.toggle_learned(2);
        p.toggle_learned(-1);
        p.toggle_learned(-3);
        let claimed = p.claimed_state();
        assert_eq!(claimed.phi.len(), p.formula.len() + 1);
        assert_eq!(claimed.phi.last().unwrap(), &vec![2, -3]);

        p.draft_rule = Unit;
        assert_eq!(p.claimed_state().phi, p.formula);
    }
}
