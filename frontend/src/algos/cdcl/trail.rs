use super::state::{CdclRule, CdclStep, Sat};

const MAX_BRANCHES: usize = 4096;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct PreviewSlot {
    pub lit: Option<i32>,
    pub decision: bool,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Branch {
    trail: Vec<PreviewSlot>,
    peak: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StepIssue {
    NoDecisionToUndo,
    ConflictAfterDecide,
    DecideAfterConflict,
    AlreadyAssigned,
    TrailOverflow,
    ClauseOutOfRange,
    VarOutOfRange,
    DecisionOutOfOrder,
}

impl StepIssue {
    pub fn message(self) -> &'static str {
        match self {
            StepIssue::NoDecisionToUndo => {
                "A conflict without a decision on the trail ends the run as unsatisfiable; there is nothing to learn from."
            }
            StepIssue::ConflictAfterDecide => {
                "A decision can never conflict straight away, so a conflict cannot follow it. Propagate first."
            }
            StepIssue::DecideAfterConflict => {
                "After backjumping the learned clause is unit, so the next step has to propagate it rather than decide."
            }
            StepIssue::AlreadyAssigned => {
                "This variable is already on the trail; CDCL only decides unassigned variables."
            }
            StepIssue::TrailOverflow => {
                "The trail is out of slots — it holds at most one literal per variable."
            }
            StepIssue::ClauseOutOfRange => {
                "The formula will not have a clause with this index at this point — learned clauses only exist after their conflict."
            }
            StepIssue::VarOutOfRange => "This variable is outside the variable bound.",
            StepIssue::DecisionOutOfOrder => {
                "CDCL branches on the lowest unassigned variable, and an earlier one is still free."
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StepPreview {
    pub min_depth: usize,
    pub max_depth: usize,
    pub issue: Option<StepIssue>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TracePreview {
    pub steps: Vec<StepPreview>,
    pub peak_depth: usize,
    pub ends_with_decision: bool,
    pub ends_on_conflict: bool,
    pub target: Sat,
}

impl TracePreview {
    pub fn blocker(&self) -> Option<String> {
        if let Some((i, issue)) = self
            .steps
            .iter()
            .enumerate()
            .find_map(|(i, p)| p.issue.map(|issue| (i, issue)))
        {
            return Some(format!("Step {}: {}", i + 1, issue.message()));
        }
        if self.ends_on_conflict {
            return Some(
                "A run cannot end on a conflict: after backjumping the learned clause is unit and still has to be propagated."
                    .to_string(),
            );
        }
        if self.target == Sat::Unsat && self.ends_with_decision {
            return Some(
                "An unsatisfiable run ends on a conflict with no decision left — this trace keeps a decision whichever level each conflict jumps back to."
                    .to_string(),
            );
        }
        None
    }
}

fn var_of(lit: i32) -> usize {
    lit.unsigned_abs() as usize
}

fn decide_issue(trail: &[PreviewSlot], step: &CdclStep, vars: usize) -> Option<StepIssue> {
    let assigned = |v: usize| trail.iter().any(|s| s.lit.is_some_and(|l| var_of(l) == v));
    if step.vi == 0 || step.vi > vars {
        Some(StepIssue::VarOutOfRange)
    } else if assigned(step.vi) {
        Some(StepIssue::AlreadyAssigned)
    } else if trail.iter().all(|s| s.lit.is_some()) && (1..step.vi).any(|v| !assigned(v)) {
        Some(StepIssue::DecisionOutOfOrder)
    } else {
        None
    }
}

fn advance(
    branch: &Branch,
    step: &CdclStep,
    previous: Option<CdclRule>,
    vars: usize,
    available: usize,
) -> Vec<(Branch, Option<StepIssue>)> {
    let trail = &branch.trail;
    let (issue, trails): (Option<StepIssue>, Vec<Vec<PreviewSlot>>) = match step.rule {
        CdclRule::Decide => {
            let issue = if previous == Some(CdclRule::Conflict) {
                Some(StepIssue::DecideAfterConflict)
            } else {
                decide_issue(trail, step, vars)
            };
            let mut next = trail.clone();
            next.push(PreviewSlot {
                lit: Some(step.lit()),
                decision: true,
            });
            (issue, vec![next])
        }
        CdclRule::Unit => {
            let issue = (step.ci >= available).then_some(StepIssue::ClauseOutOfRange);
            let mut next = trail.clone();
            next.push(PreviewSlot {
                lit: None,
                decision: false,
            });
            (issue, vec![next])
        }
        CdclRule::Conflict => {
            let decision_slots: Vec<usize> = trail
                .iter()
                .enumerate()
                .filter(|(_, s)| s.decision)
                .map(|(q, _)| q)
                .collect();
            let issue = if step.ci >= available {
                Some(StepIssue::ClauseOutOfRange)
            } else if previous == Some(CdclRule::Decide) {
                Some(StepIssue::ConflictAfterDecide)
            } else if decision_slots.is_empty() {
                Some(StepIssue::NoDecisionToUndo)
            } else {
                None
            };
            let trails = if decision_slots.is_empty() {
                vec![trail.clone()]
            } else {
                decision_slots
                    .iter()
                    .map(|&q| trail[..q].to_vec())
                    .collect()
            };
            (issue, trails)
        }
    };

    trails
        .into_iter()
        .map(|t| {
            let issue = issue.or((t.len() > vars).then_some(StepIssue::TrailOverflow));
            (
                Branch {
                    peak: branch.peak.max(t.len()),
                    trail: t,
                },
                issue,
            )
        })
        .collect()
}

pub fn preview(steps: &[CdclStep], vars: usize, clauses: usize, target: Sat) -> TracePreview {
    let mut branches = vec![Branch {
        trail: Vec::new(),
        peak: 0,
    }];
    let mut out: Vec<StepPreview> = Vec::with_capacity(steps.len());
    let mut learned = 0usize;

    for (i, step) in steps.iter().enumerate() {
        let previous = i.checked_sub(1).map(|p| steps[p].rule);
        let available = clauses.saturating_add(learned);
        let outcomes: Vec<(Branch, Option<StepIssue>)> = branches
            .iter()
            .flat_map(|b| advance(b, step, previous, vars, available))
            .collect();

        let issue = if outcomes.iter().all(|(_, issue)| issue.is_some()) {
            outcomes.first().and_then(|(_, issue)| *issue)
        } else {
            None
        };
        let mut next: Vec<Branch> = outcomes
            .into_iter()
            .filter(|(_, own)| issue.is_some() || own.is_none())
            .map(|(b, _)| b)
            .collect();
        next.sort();
        next.dedup_by(|later, earlier| later.trail == earlier.trail);
        next.truncate(MAX_BRANCHES);

        out.push(StepPreview {
            min_depth: next.iter().map(|b| b.trail.len()).min().unwrap_or(0),
            max_depth: next.iter().map(|b| b.trail.len()).max().unwrap_or(0),
            issue,
        });
        branches = next;
        if step.rule == CdclRule::Conflict {
            learned += 1;
        }
    }

    TracePreview {
        steps: out,
        peak_depth: branches.iter().map(|b| b.peak).min().unwrap_or(0),
        ends_with_decision: branches.iter().all(|b| b.trail.iter().any(|s| s.decision)),
        ends_on_conflict: steps.last().is_some_and(|s| s.rule == CdclRule::Conflict),
        target,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decide(vi: usize) -> CdclStep {
        CdclStep {
            rule: CdclRule::Decide,
            vi,
            neg: false,
            ci: 0,
        }
    }
    fn unit(ci: usize) -> CdclStep {
        CdclStep {
            rule: CdclRule::Unit,
            vi: 1,
            neg: false,
            ci,
        }
    }
    fn conflict(ci: usize) -> CdclStep {
        CdclStep {
            rule: CdclRule::Conflict,
            vi: 1,
            neg: false,
            ci,
        }
    }

    #[test]
    fn the_backjump_level_is_left_open() {
        let steps = [
            decide(1),
            decide(2),
            decide(3),
            unit(0),
            conflict(1),
            unit(4),
        ];
        let p = preview(&steps, 4, 4, Sat::Sat);
        assert_eq!(p.blocker(), None);
        assert_eq!((p.steps[4].min_depth, p.steps[4].max_depth), (0, 2));
        assert_eq!((p.steps[5].min_depth, p.steps[5].max_depth), (1, 3));
    }

    #[test]
    fn a_decision_may_be_retaken_when_some_backjump_frees_it() {
        let steps = [
            decide(1),
            decide(2),
            decide(3),
            unit(0),
            conflict(1),
            unit(4),
            decide(2),
        ];
        assert_eq!(preview(&steps, 4, 4, Sat::Sat).blocker(), None);
    }

    #[test]
    fn a_decided_variable_cannot_be_decided_again() {
        let steps = [decide(1), unit(0), decide(1)];
        let p = preview(&steps, 3, 3, Sat::Sat);
        assert_eq!(p.steps[2].issue, Some(StepIssue::AlreadyAssigned));
    }

    #[test]
    fn learned_clauses_only_exist_after_their_conflict() {
        let p = preview(&[decide(1), unit(2)], 2, 2, Sat::Sat);
        assert_eq!(p.steps[1].issue, Some(StepIssue::ClauseOutOfRange));

        let p = preview(&[decide(1), unit(0), conflict(1), unit(2)], 2, 2, Sat::Sat);
        assert_eq!(p.blocker(), None);
    }

    #[test]
    fn the_learned_clause_must_be_propagated_before_deciding() {
        let p = preview(
            &[decide(1), unit(0), conflict(1), decide(2)],
            2,
            2,
            Sat::Sat,
        );
        assert_eq!(p.steps[3].issue, Some(StepIssue::DecideAfterConflict));
    }

    #[test]
    fn a_run_cannot_end_on_a_conflict() {
        let p = preview(&[decide(1), unit(0), conflict(1)], 2, 2, Sat::Sat);
        assert!(p.ends_on_conflict);
        assert!(p.blocker().unwrap().contains("cannot end on a conflict"));
    }

    #[test]
    fn a_conflict_needs_a_decision_and_cannot_follow_one() {
        let p = preview(&[unit(0), conflict(1)], 2, 2, Sat::Sat);
        assert_eq!(p.steps[1].issue, Some(StepIssue::NoDecisionToUndo));

        let p = preview(&[decide(1), conflict(0)], 2, 2, Sat::Sat);
        assert_eq!(p.steps[1].issue, Some(StepIssue::ConflictAfterDecide));
    }

    #[test]
    fn unsat_is_blocked_only_when_every_backjump_keeps_a_decision() {
        let reachable = [decide(1), decide(2), unit(0), conflict(1), unit(2)];
        assert_eq!(preview(&reachable, 3, 3, Sat::Unsat).blocker(), None);

        let kept = [
            decide(1),
            decide(2),
            unit(0),
            conflict(1),
            unit(3),
            decide(3),
        ];
        let p = preview(&kept, 3, 3, Sat::Unsat);
        assert!(p.ends_with_decision);
        assert!(p.blocker().is_some());
    }

    #[test]
    fn the_needed_depth_is_the_shallowest_possible_run() {
        let steps = [decide(1), decide(2), unit(0), conflict(1), unit(2), unit(0)];
        let p = preview(&steps, 5, 3, Sat::Sat);
        assert_eq!(p.peak_depth, 3);
    }
}
