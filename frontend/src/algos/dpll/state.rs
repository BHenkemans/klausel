use crate::algos::controls::MIN_LIMIT;
use crate::algos::dpll::parse::parse_trace;
use crate::algos::dpll::trail::preview;
use crate::algos::feedback::{GradedStep, RuleText, TraceVerdict};
pub use crate::algos::{Module, Sat};
use serde::{Deserialize, Serialize};

/// The three DPLL transitions.
///
/// Their parameters are not all literals, which is the main way this designer
/// departs from the Davis–Putnam one: `Unit` names the *clause* that fires
/// (which literal it forces is up to the generated formula), and `Backtrack`
/// names the clause in conflict; it always flips the most recent decision.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DpllRule {
    Decide,
    Unit,
    Backtrack,
}

impl RuleText for DpllRule {
    fn name(self) -> &'static str {
        match self {
            DpllRule::Backtrack => "backtrack",
            DpllRule::Unit => "unit",
            DpllRule::Decide => "decide",
        }
    }

    fn unapplicable(self) -> &'static str {
        match self {
            DpllRule::Decide => {
                "That literal is already assigned on the trail, or the formula is already satisfied."
            }
            DpllRule::Unit => {
                "That clause is not unit under the current trail — it has none or several literals left."
            }
            DpllRule::Backtrack => {
                "That clause is not in conflict, or there is no decision left to undo."
            }
        }
    }

    fn misapplied(self) -> &'static str {
        match self {
            DpllRule::Decide => {
                "Right rule, wrong trail — append the literal as a decision (click it in the trail until it turns solid) and change nothing else."
            }
            DpllRule::Unit => {
                "Right rule, wrong trail — append the clause's one surviving literal as a propagation."
            }
            DpllRule::Backtrack => {
                "Right rule, wrong trail — cut back to the last decision, then append its complement as a propagation."
            }
        }
    }
}

/// A step of the designed trace. `vi`/`neg` and `ci` are kept across rule
/// changes so cycling a row's rule and cycling back does not lose the choice.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DpllStep {
    pub rule: DpllRule,
    /// 1-based variable index; read by `Decide`.
    pub vi: usize,
    /// Polarity of the decided literal; read by `Decide`.
    pub neg: bool,
    /// 0-based clause index; read by `Unit`.
    pub ci: usize,
}

impl DpllStep {
    /// The decided literal in 1-based signed form.
    pub fn lit(&self) -> i32 {
        if self.neg {
            -(self.vi as i32)
        } else {
            self.vi as i32
        }
    }
}

/// One slot of the trail in a solved model.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TrailEntry {
    pub lit: i32,
    pub decision: bool,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GeneratedExercise {
    /// Clause `i` is the one a `Unit` step addresses as `ci`.
    pub set: Vec<Vec<i32>>,
    /// Trail after each state, `t = 0 ..= steps.len()`.
    pub trail: Vec<Vec<TrailEntry>>,
}

/// Why a generation attempt produced nothing. Distinguishing the two matters:
/// a timeout says "try again or shrink the search", `Impossible` says "this
/// trace has no realisation at all under these bounds".
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum GenerateOutcome {
    Found(GeneratedExercise),
    Impossible,
    TimedOut,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DpllGenerateState {
    pub target: Sat,
    pub steps: Vec<DpllStep>,
    /// Exact size of the variable universe, and hence the number of trail slots.
    pub vars: usize,
    /// Exact number of clauses: the encoding forbids empty and duplicate
    /// clauses, so all `clauses` of them show up in the answer.
    pub clauses: usize,
    pub latest_result: Option<Result<GeneratedExercise, String>>,
    pub is_loading: bool,
}

impl Default for DpllGenerateState {
    fn default() -> Self {
        // decide -> propagate -> conflict -> flip the decision -> satisfied:
        // the smallest trace that exercises all three rules.
        Self {
            target: Sat::Sat,
            steps: vec![
                DpllStep {
                    rule: DpllRule::Decide,
                    vi: 1,
                    neg: false,
                    ci: 0,
                },
                DpllStep {
                    rule: DpllRule::Unit,
                    vi: 1,
                    neg: false,
                    ci: 0,
                },
                DpllStep {
                    rule: DpllRule::Backtrack,
                    vi: 1,
                    neg: false,
                    ci: 0,
                },
            ],
            vars: 2,
            clauses: 2,
            latest_result: None,
            is_loading: false,
        }
    }
}

/// Bounds the designer offers. The upper limits keep the encoding's search
/// space at a size Z3 answers interactively; the trail is one slot per
/// variable, so `MAX_VARS` also caps how deep a designed run can go.
pub const MAX_VARS: usize = 8;
pub const MAX_CLAUSES: usize = 9;

/// The edits the designer can make, as named operations rather than closures
/// inlined into the view. Every control sets its value directly — nothing
/// cycles, so choosing `c3` costs one click whatever the row said before.
impl DpllGenerateState {
    /// Every edit invalidates the previous answer — it was generated for a
    /// different trace.
    fn edited(&mut self) {
        self.latest_result = None;
    }

    pub fn set_target(&mut self, target: Sat) {
        self.target = target;
        self.edited();
    }

    pub fn set_rule(&mut self, idx: usize, rule: DpllRule) {
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
        let ci = ci.min(self.clauses.saturating_sub(1));
        if let Some(s) = self.steps.get_mut(idx) {
            s.ci = ci;
            self.edited();
        }
    }

    /// Append a step of a chosen rule. New steps inherit the previous step's
    /// clause so consecutive propagations on one clause need no re-selecting.
    pub fn push_rule(&mut self, rule: DpllRule) {
        let ci = self.steps.last().map(|s| s.ci).unwrap_or(0);
        self.steps.push(DpllStep {
            rule,
            vi: 1,
            neg: false,
            ci,
        });
        self.edited();
    }

    /// Copy a step in below itself, so a forgotten step in the middle does not
    /// mean rebuilding the whole tail.
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

    /// Lowering a bound has to pull the steps back inside it, or the trace
    /// would name a variable the formula does not have.
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
            for s in &mut self.steps {
                s.ci = s.ci.min(self.clauses - 1);
            }
            self.edited();
        }
    }

    /// Replace the trace with one parsed from text, raising the bounds to fit
    /// it. Bounds are only ever raised: a pasted trace should not silently
    /// shrink a universe the rest of the design depends on.
    ///
    /// Fitting the variable bound matters more than it looks — a trace whose
    /// trail gets `n` deep needs `n` variables even when it names fewer, and
    /// too low a bound makes the trace unrealisable rather than merely clamped.
    pub fn apply_trace(&mut self, text: &str) -> Result<usize, String> {
        let parsed = parse_trace(text).map_err(|e| e.describe())?;

        let named_vars = parsed.steps.iter().map(|s| s.vi).max().unwrap_or(1);
        let named_clauses = parsed
            .steps
            .iter()
            .filter(|s| matches!(s.rule, DpllRule::Unit))
            .map(|s| s.ci + 1)
            .max()
            .unwrap_or(1);
        let depth = preview(&parsed.steps, usize::MAX, usize::MAX, Sat::Sat).peak_depth;

        // An `expects:` comment states bounds the trace itself cannot: a unit
        // step names a clause, not the literal it derives, so the formula's
        // width is genuinely unrecoverable from the steps alone. Where the
        // text says, believe it; otherwise fall back to what is provably
        // necessary and let the generator report if that is not enough.
        let (hinted_vars, hinted_clauses) = parsed.expects.unwrap_or((0, 0));
        let needed_vars = named_vars.max(depth).max(hinted_vars);
        let named_clauses = named_clauses.max(hinted_clauses);
        if needed_vars > MAX_VARS {
            return Err(format!(
                "this trace needs {needed_vars} variables; the designer allows at most {MAX_VARS}"
            ));
        }
        if named_clauses > MAX_CLAUSES {
            return Err(format!(
                "this trace names c{}; the designer allows at most c{}",
                named_clauses - 1,
                MAX_CLAUSES - 1
            ));
        }

        self.vars = self.vars.max(needed_vars);
        self.clauses = self.clauses.max(named_clauses);
        let n = parsed.steps.len();
        self.steps = parsed.steps;
        if let Some(target) = parsed.target {
            self.target = target;
        }
        self.edited();
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DpllRule::{Backtrack, Decide, Unit};

    /// The 13-step textbook derivation over p,q,r,s,t with 8 clauses, entered
    /// with the designer's own operations. Every parameter is set directly, so
    /// this is also the click count: one add plus at most one parameter pick
    /// per step.
    #[test]
    fn the_textbook_derivation_can_be_entered_step_by_step() {
        let mut s = DpllGenerateState::default();
        s.clear_steps();
        for _ in 0..3 {
            s.inc_vars();
        }
        for _ in 0..6 {
            s.inc_clauses();
        }

        let decide = |s: &mut DpllGenerateState, vi: usize| {
            s.push_rule(Decide);
            s.set_var(s.steps.len() - 1, vi);
        };
        let unit = |s: &mut DpllGenerateState, ci: usize| {
            s.push_rule(Unit);
            s.set_clause(s.steps.len() - 1, ci);
        };

        decide(&mut s, 1); // 1  decide p
        unit(&mut s, 1); //    2  unit (2)
        decide(&mut s, 2); // 3  decide q
        unit(&mut s, 4); //    4  unit (5)
        s.push_rule(Backtrack); // 5  conflict (3)
        unit(&mut s, 3); //    6  unit (4)
        s.push_rule(Backtrack); // 7  conflict (6)
        decide(&mut s, 2); // 8  decide q again, the flip freed it
        unit(&mut s, 0); //    9  unit (1)
        unit(&mut s, 6); //    10 unit (7)
        s.push_rule(Backtrack); // 11 conflict (8)
        decide(&mut s, 3); // 12 decide r
        decide(&mut s, 4); // 13 decide s

        assert_eq!(
            s.steps.iter().map(|x| x.rule).collect::<Vec<_>>(),
            [
                Decide, Unit, Decide, Unit, Backtrack, Unit, Backtrack, Decide, Unit, Unit,
                Backtrack, Decide, Decide
            ]
        );
        assert_eq!(
            s.steps
                .iter()
                .filter(|x| matches!(x.rule, Unit))
                .map(|x| x.ci)
                .collect::<Vec<_>>(),
            [1, 4, 3, 0, 6]
        );
        assert_eq!(
            s.steps
                .iter()
                .filter(|x| matches!(x.rule, Decide))
                .map(|x| x.lit())
                .collect::<Vec<_>>(),
            [1, 2, 2, 3, 4]
        );
        assert_eq!(
            preview(&s.steps, s.vars, s.clauses, s.target).blocker(),
            None
        );
    }

    const TEXTBOOK: &str = "\
decide 0
unit 1
decide 2
unit 4
backtrack 0
unit 3
backtrack 5
decide 2
unit 0
unit 6
backtrack 2
decide 4
decide 6
sat
";

    /// The same trace, pasted. This is the path that makes a 13-step exercise
    /// one action instead of dozens.
    #[test]
    fn the_textbook_derivation_can_be_pasted() {
        let mut s = DpllGenerateState::default();
        let n = s.apply_trace(TEXTBOOK).expect("parses");

        assert_eq!(n, 13);
        assert_eq!(s.target, Sat::Sat);
        // Four variables and seven clauses are what the trace *proves* it
        // needs: it names four variables, its trail reaches four deep, and it
        // propagates on c6. Whether that is enough is a question about the
        // formula, not the trace — this one in fact needs a fifth variable,
        // which only the generator can discover.
        assert_eq!((s.vars, s.clauses), (4, 7));
        assert_eq!(
            preview(&s.steps, s.vars, s.clauses, s.target).blocker(),
            None
        );
    }

    /// Which is why the example files record their bounds in a comment.
    #[test]
    fn an_expects_comment_pins_the_bounds() {
        let mut s = DpllGenerateState::default();
        s.apply_trace(&format!("# expects: --vars 5 --clauses 8\n{TEXTBOOK}"))
            .expect("parses");
        assert_eq!((s.vars, s.clauses), (5, 8));
    }

    /// A trace can need more variables than it names: the trail is one slot per
    /// variable, and propagated literals occupy slots without being written
    /// down. Fitting only the named variables would leave the trace unrealisable.
    #[test]
    fn applying_a_trace_fits_the_variable_bound_to_the_trail_depth() {
        let mut s = DpllGenerateState::default();
        s.apply_trace("decide 0\nunit 0\nunit 1\nunit 2\nsat\n")
            .unwrap();

        assert_eq!(s.steps.len(), 4);
        // Only x0 is named, but the trail reaches four literals.
        assert_eq!(s.vars, 4);
        assert_eq!(s.clauses, 3);
    }

    #[test]
    fn applying_a_trace_never_shrinks_the_bounds() {
        let mut s = DpllGenerateState::default();
        for _ in 0..5 {
            s.inc_vars();
            s.inc_clauses();
        }
        let (vars, clauses) = (s.vars, s.clauses);
        s.apply_trace("unit 0\nsat\n").unwrap();
        assert_eq!((s.vars, s.clauses), (vars, clauses));
    }

    #[test]
    fn a_trace_that_cannot_fit_the_bounds_is_refused() {
        let mut s = DpllGenerateState::default();
        let steps = "unit 0\n".repeat(MAX_VARS + 1);
        let e = s.apply_trace(&steps).unwrap_err();
        assert!(e.contains("variables"), "{e}");
        // Refusing leaves the design untouched.
        assert_eq!(s.steps, DpllGenerateState::default().steps);

        let e = s
            .apply_trace(&format!("unit c{MAX_CLAUSES}\n"))
            .unwrap_err();
        assert!(e.contains(&format!("c{MAX_CLAUSES}")), "{e}");
    }

    #[test]
    fn a_parse_error_is_reported_with_its_line() {
        let mut s = DpllGenerateState::default();
        let e = s.apply_trace("decide 0\nwobble 1\n").unwrap_err();
        assert!(e.starts_with("line 2:"), "{e}");
    }

    #[test]
    fn inserting_copies_the_step_below_itself() {
        let mut s = DpllGenerateState::default();
        s.insert_after(0);
        assert_eq!(s.steps.len(), 4);
        assert_eq!(s.steps[0], s.steps[1]);
        assert_eq!(s.steps[2].rule, Unit);
    }

    /// Lowering a bound must not leave a step pointing outside it.
    #[test]
    fn lowering_a_bound_pulls_the_steps_back_inside_it() {
        let mut s = DpllGenerateState::default();
        for _ in 0..6 {
            s.inc_clauses();
        }
        for _ in 0..3 {
            s.inc_vars();
        }
        s.set_clause(1, 7);
        s.set_var(0, 5);

        s.dec_clauses();
        s.dec_vars();
        assert_eq!(s.steps[1].ci, 6);
        assert_eq!(s.steps[0].vi, 4);
    }

    #[test]
    fn the_bounds_do_not_run_past_their_limits() {
        let mut s = DpllGenerateState::default();
        for _ in 0..50 {
            s.inc_vars();
            s.inc_clauses();
        }
        assert_eq!((s.vars, s.clauses), (MAX_VARS, MAX_CLAUSES));
        for _ in 0..50 {
            s.dec_vars();
            s.dec_clauses();
        }
        assert_eq!((s.vars, s.clauses), (MIN_LIMIT, MIN_LIMIT));
    }
}

// ---------- Practice module ----------

/// Which submodule of DPLL is on screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DpllAppState {
    pub module: Module,
}

impl Default for DpllAppState {
    fn default() -> Self {
        Self {
            module: Module::Practice,
        }
    }
}

pub type DpllGradedStep = GradedStep<Vec<TrailEntry>, DpllRule>;

#[derive(Clone, PartialEq, Debug)]
pub struct DpllPracticeHistoryEntry {
    pub step: DpllStep,
    pub verdict: DpllGradedStep,
}

/// Unlike Davis-Putnam, the formula never changes — only the trail does, so
/// `formula` is fixed for the whole run and `working` is the trail under edit.
#[derive(Clone, PartialEq, Debug)]
pub struct DpllPracticeState {
    pub formula: Vec<Vec<i32>>,
    pub history: Vec<DpllPracticeHistoryEntry>,
    pub working: Vec<TrailEntry>,
    pub draft_rule: DpllRule,
    /// Read by `Decide`, in 1-based signed form.
    pub draft_lit: Option<i32>,
    /// Read by `Unit` and `Backtrack`, 0-based.
    pub draft_ci: Option<usize>,
    pub concluded: Option<Sat>,
    pub verdict: Option<TraceVerdict>,
    pub grading: bool,

    // Practice (random exercise) difficulty knobs.
    pub practice_max_vars: usize,
    pub practice_max_clauses: usize,
    pub practice_max_steps: usize,
    pub practice_loading: bool,
    pub practice_error: Option<String>,
}

impl Default for DpllPracticeState {
    fn default() -> Self {
        // The XOR formula over α, β: UNSAT, and the only way there is to decide,
        // propagate into a conflict, and backtrack out of it.
        let xor: Vec<Vec<i32>> = vec![vec![1, 2], vec![-1, 2], vec![1, -2], vec![-1, -2]];
        Self {
            formula: xor,
            history: vec![],
            working: vec![],
            draft_rule: DpllRule::Decide,
            draft_lit: None,
            draft_ci: None,
            concluded: None,
            verdict: None,
            grading: false,
            practice_max_vars: 3,
            practice_max_clauses: 4,
            practice_max_steps: 4,
            practice_loading: false,
            practice_error: None,
        }
    }
}

impl DpllPracticeState {
    /// The trail the next step starts from: the last claimed one, or empty.
    pub fn current_trail(&self) -> Vec<TrailEntry> {
        self.history
            .last()
            .map(|h| h.verdict.claimed.clone())
            .unwrap_or_default()
    }
}
