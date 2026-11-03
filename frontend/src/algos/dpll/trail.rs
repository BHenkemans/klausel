//! A static replay of a designed trace, run before Z3 is asked anything.
//!
//! The DPLL encoding pins the trail down completely except for *which* literal
//! a unit step derives, so we can replay the shape of the trail from the trace
//! alone: decisions name their own literal, propagations occupy an unknown
//! slot, and backtracking rewinds to the last decision. That is enough to catch
//! the traces that no formula can realise, and to show the trail depth a design
//! needs before the teacher raises the variable bound by trial and error.

use super::state::{DpllRule, DpllStep, Sat};

/// A trail slot as far as the static replay can know it. `lit` is `None` for a
/// literal a unit step derived: the clause fires, but the formula that decides
/// *what* it forces does not exist yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PreviewSlot {
    pub lit: Option<i32>,
    pub decision: bool,
}

/// A reason the step can never fire, each read straight off a rule's guard.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StepIssue {
    /// `Backtrack` needs a decision on the trail; there is none left.
    NoDecisionToUndo,
    /// A decision cannot conflict immediately — a clause falsified by the
    /// decided literal alone would have been unit before the decision, and
    /// `Decide` refuses to fire while a unit clause is present.
    BacktrackAfterDecide,
    /// `Decide` refuses a variable that already has a value on the trail.
    AlreadyAssigned,
    /// The trail has one slot per variable and this step needs one more.
    TrailOverflow,
    /// The step names a clause the formula will not have.
    ClauseOutOfRange,
    /// The step names a variable outside the universe.
    VarOutOfRange,
    /// `Decide` branches on the lowest unassigned variable, and a lower one is
    /// demonstrably still free. Only raised when the replay can be sure: a
    /// propagated slot holds an unknown literal, so any trail carrying one
    /// might already have covered the variable.
    DecisionOutOfOrder,
}

impl StepIssue {
    pub fn message(self) -> &'static str {
        match self {
            StepIssue::NoDecisionToUndo => {
                "Nothing to backtrack to — the trail carries no decision at this point."
            }
            StepIssue::BacktrackAfterDecide => {
                "A decision can never conflict straight away, so backtracking cannot follow it. \
                 Propagate first."
            }
            StepIssue::AlreadyAssigned => {
                "This variable is already on the trail; DPLL only decides unassigned variables."
            }
            StepIssue::TrailOverflow => {
                "The trail is out of slots — it holds at most one literal per variable."
            }
            StepIssue::ClauseOutOfRange => "The formula will not have a clause with this index.",
            StepIssue::VarOutOfRange => "This variable is outside the variable bound.",
            StepIssue::DecisionOutOfOrder => {
                "DPLL branches on the lowest unassigned variable, and an earlier one is still free."
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StepPreview {
    /// Trail depth after the step.
    pub depth: usize,
    pub issue: Option<StepIssue>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TracePreview {
    pub steps: Vec<StepPreview>,
    /// Deepest the trail ever gets, i.e. how many variable slots the trace needs.
    pub peak_depth: usize,
    /// The trail the trace ends on.
    pub final_trail: Vec<PreviewSlot>,
    /// An UNSAT verdict needs an empty decision set, and this trace ends with
    /// one still on the trail.
    pub decision_survives: bool,
}

impl TracePreview {
    /// The first reason generation is pointless, if any.
    pub fn blocker(&self) -> Option<String> {
        if let Some((i, p)) = self
            .steps
            .iter()
            .enumerate()
            .find(|(_, p)| p.issue.is_some())
        {
            let issue = p.issue.expect("filtered on is_some");
            return Some(format!("Step {}: {}", i + 1, issue.message()));
        }
        if self.decision_survives {
            return Some(
                "An unsatisfiable run ends with an empty decision set — backtrack the remaining \
                 decisions, or target satisfiable instead."
                    .to_string(),
            );
        }
        None
    }
}

pub fn preview(steps: &[DpllStep], vars: usize, clauses: usize, target: Sat) -> TracePreview {
    let mut trail: Vec<PreviewSlot> = Vec::new();
    let mut out: Vec<StepPreview> = Vec::with_capacity(steps.len());
    let mut peak_depth = 0;

    for (i, step) in steps.iter().enumerate() {
        let mut issue = None;

        match step.rule {
            DpllRule::Decide => {
                let lit = step.lit();
                if step.vi == 0 || step.vi > vars {
                    issue = Some(StepIssue::VarOutOfRange);
                } else if trail
                    .iter()
                    .any(|s| s.lit.is_some_and(|l| l.abs() == lit.abs()))
                {
                    issue = Some(StepIssue::AlreadyAssigned);
                } else if trail.iter().all(|s| s.lit.is_some())
                    && (1..step.vi).any(|v| {
                        !trail
                            .iter()
                            .any(|s| s.lit.is_some_and(|l| l.unsigned_abs() as usize == v))
                    })
                {
                    // Every slot is a known literal, so the assigned set is
                    // exactly what the replay can see and the check is exact.
                    issue = Some(StepIssue::DecisionOutOfOrder);
                }
                trail.push(PreviewSlot {
                    lit: Some(lit),
                    decision: true,
                });
            }
            DpllRule::Unit => {
                if step.ci >= clauses {
                    issue = Some(StepIssue::ClauseOutOfRange);
                }
                trail.push(PreviewSlot {
                    lit: None,
                    decision: false,
                });
            }
            DpllRule::Backtrack => {
                if step.ci >= clauses {
                    issue = Some(StepIssue::ClauseOutOfRange);
                }
                if matches!(
                    steps.get(i.wrapping_sub(1)).map(|p| p.rule),
                    Some(DpllRule::Decide)
                ) {
                    issue = Some(StepIssue::BacktrackAfterDecide);
                }
                match trail.iter().rposition(|s| s.decision) {
                    Some(q) => {
                        trail.truncate(q + 1);
                        trail[q].lit = trail[q].lit.map(|l| -l);
                        trail[q].decision = false;
                    }
                    None => issue = issue.or(Some(StepIssue::NoDecisionToUndo)),
                }
            }
        }

        if issue.is_none() && trail.len() > vars {
            issue = Some(StepIssue::TrailOverflow);
        }
        peak_depth = peak_depth.max(trail.len());
        out.push(StepPreview {
            depth: trail.len(),
            issue,
        });
    }

    let decision_survives = matches!(target, Sat::Unsat) && trail.iter().any(|s| s.decision);

    TracePreview {
        steps: out,
        peak_depth,
        final_trail: trail,
        decision_survives,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decide(vi: usize, neg: bool) -> DpllStep {
        DpllStep {
            rule: DpllRule::Decide,
            vi,
            neg,
            ci: 0,
        }
    }
    fn unit(ci: usize) -> DpllStep {
        DpllStep {
            rule: DpllRule::Unit,
            vi: 1,
            neg: false,
            ci,
        }
    }
    fn backtrack() -> DpllStep {
        DpllStep {
            rule: DpllRule::Backtrack,
            vi: 1,
            neg: false,
            ci: 0,
        }
    }

    fn known(trail: &[PreviewSlot]) -> Vec<(Option<i32>, bool)> {
        trail.iter().map(|s| (s.lit, s.decision)).collect()
    }

    /// Branching on the second variable first is explained, not left to the
    /// solver to refuse silently.
    #[test]
    fn an_out_of_order_decision_is_named() {
        let p = preview(&[decide(2, false)], 3, 4, Sat::Sat);
        assert_eq!(p.steps[0].issue, Some(StepIssue::DecisionOutOfOrder));
        assert!(p.blocker().unwrap().contains("lowest unassigned"));
    }

    /// In order, so nothing to say.
    #[test]
    fn an_in_order_decision_is_fine() {
        let p = preview(&[decide(1, false), decide(2, false)], 3, 4, Sat::Sat);
        assert!(p.steps.iter().all(|s| s.issue.is_none()));
    }

    /// A propagated slot hides which variable it took, so the replay must not
    /// guess: this trace may well be in order.
    #[test]
    fn a_propagation_makes_the_check_inconclusive() {
        let p = preview(&[unit(0), decide(2, false)], 3, 4, Sat::Sat);
        assert_eq!(
            p.steps[1].issue, None,
            "cannot know what the propagation assigned"
        );
    }

    /// The backend prints this trace's solved trail as `t4: x0* !x1` — the
    /// replay has to agree.
    #[test]
    fn replays_a_deep_backtrack() {
        let steps = [decide(1, false), decide(2, false), unit(0), backtrack()];
        let p = preview(&steps, 3, 4, Sat::Sat);

        assert!(p.blocker().is_none());
        assert_eq!(
            p.steps.iter().map(|s| s.depth).collect::<Vec<_>>(),
            [1, 2, 3, 2]
        );
        assert_eq!(p.peak_depth, 3);
        assert_eq!(known(&p.final_trail), [(Some(1), true), (Some(-2), false)]);
    }

    /// Both branches fail, so the trail ends with no decision left and the
    /// UNSAT target is reachable.
    #[test]
    fn unsat_target_accepts_an_empty_decision_set() {
        let steps = [decide(1, false), unit(0), backtrack(), unit(2)];
        let p = preview(&steps, 2, 4, Sat::Unsat);

        assert!(!p.decision_survives);
        assert!(p.blocker().is_none());
        assert_eq!(known(&p.final_trail), [(Some(-1), false), (None, false)]);
    }

    #[test]
    fn unsat_target_rejects_a_surviving_decision() {
        let p = preview(&[decide(1, false), unit(0)], 3, 3, Sat::Unsat);
        assert!(p.decision_survives);
        assert!(p.blocker().is_some());

        // The same trace is fine when the target is satisfiability.
        assert!(!preview(&[decide(1, false), unit(0)], 3, 3, Sat::Sat).decision_survives);
    }

    #[test]
    fn backtrack_cannot_follow_a_decision() {
        let p = preview(&[decide(1, false), backtrack()], 3, 3, Sat::Sat);
        assert_eq!(p.steps[1].issue, Some(StepIssue::BacktrackAfterDecide));
    }

    #[test]
    fn backtrack_needs_a_decision_on_the_trail() {
        let p = preview(&[backtrack()], 2, 2, Sat::Sat);
        assert_eq!(p.steps[0].issue, Some(StepIssue::NoDecisionToUndo));

        // A flipped decision is no longer one, so a second undo has nothing left.
        let steps = [decide(1, false), unit(0), backtrack(), backtrack()];
        let p = preview(&steps, 3, 3, Sat::Sat);
        assert_eq!(p.steps[3].issue, Some(StepIssue::NoDecisionToUndo));
    }

    #[test]
    fn a_variable_is_only_decided_once() {
        let steps = [decide(1, false), unit(0), decide(1, true)];
        let p = preview(&steps, 3, 3, Sat::Sat);
        assert_eq!(p.steps[2].issue, Some(StepIssue::AlreadyAssigned));

        // Backtracking does not free the variable — it flips it in place.
        let steps = [decide(1, false), unit(0), backtrack(), decide(1, false)];
        let p = preview(&steps, 3, 3, Sat::Sat);
        assert_eq!(p.steps[3].issue, Some(StepIssue::AlreadyAssigned));
    }

    #[test]
    fn the_trail_holds_one_literal_per_variable() {
        let p = preview(&[unit(0), unit(1), unit(2)], 2, 3, Sat::Sat);
        assert_eq!(
            p.steps.iter().map(|s| s.issue).collect::<Vec<_>>(),
            [None, None, Some(StepIssue::TrailOverflow)]
        );
    }

    #[test]
    fn out_of_range_parameters_are_named() {
        let p = preview(&[unit(3)], 2, 3, Sat::Sat);
        assert_eq!(p.steps[0].issue, Some(StepIssue::ClauseOutOfRange));

        let p = preview(&[decide(4, false)], 2, 3, Sat::Sat);
        assert_eq!(p.steps[0].issue, Some(StepIssue::VarOutOfRange));
    }

    /// A real 13-step textbook exercise over p,q,r,s,t with 8 clauses: decide,
    /// propagate, backtrack twice, re-decide the same variable on the other
    /// branch, then decide out to a model. The static replay must not stand in
    /// the way of it — Z3 realises this trace in ~40ms.
    #[test]
    fn a_full_textbook_derivation_passes() {
        // p=1 q=2 r=3 s=4 t=5; clause (n) of the exercise is index n-1.
        let steps = [
            decide(1, false), // decide p
            unit(1),          // (2) -> t
            decide(2, false), // decide q
            unit(4),          // (5) -> r
            backtrack(),      // conflict (3), flip q
            unit(3),          // (4) -> s
            backtrack(),      // conflict (6), flip p
            decide(2, false), // decide q again, now that the flip freed it
            unit(0),          // (1) -> !r
            unit(6),          // (7) -> !s
            backtrack(),      // conflict (8), flip q
            decide(3, false), // decide r
            decide(4, false), // decide s
        ];
        let p = preview(&steps, 5, 8, Sat::Sat);

        assert_eq!(p.blocker(), None);
        // Deepest the run ever gets is four literals, so five slots suffice.
        assert_eq!(p.peak_depth, 4);
        assert_eq!(
            known(&p.final_trail),
            [
                (Some(-1), false),
                (Some(-2), false),
                (Some(3), true),
                (Some(4), true)
            ]
        );
    }

    #[test]
    fn the_first_failing_step_is_the_one_reported() {
        let steps = [unit(0), backtrack(), backtrack()];
        let blocker = preview(&steps, 3, 3, Sat::Sat).blocker().unwrap();
        assert!(blocker.starts_with("Step 2:"), "{blocker}");
    }
}
