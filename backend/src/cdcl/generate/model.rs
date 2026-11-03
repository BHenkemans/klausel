use std::time::Duration;

use z3::ast::Bool;

use crate::cdcl::trace::{Rule, Step, Trace};
use crate::generate::{SolveOutcome, format_clause, holds, lit_name};

#[allow(non_snake_case)]
pub struct Model {
    pub(crate) var_amount: usize,
    pub(crate) original_clause_amount: usize,
    pub(crate) clause_amount: usize,
    pub(crate) step_amount: usize,
    pub(crate) Phi: Vec<Vec<Vec<Bool>>>,
    pub(crate) Act: Vec<Vec<Bool>>,
    pub(crate) M: Vec<Vec<Vec<Bool>>>,
    pub(crate) D: Vec<Vec<Bool>>,
    pub(crate) Ant: Vec<Vec<Vec<Bool>>>,
    pub(crate) ordered_decisions: bool,
}

fn consts2(name: &str, a: usize, b: usize) -> Vec<Vec<Bool>> {
    (0..a)
        .map(|i| {
            (0..b)
                .map(|j| Bool::new_const(format!("{name}_{i}_{j}")))
                .collect()
        })
        .collect()
}

fn consts3(name: &str, a: usize, b: usize, c: usize) -> Vec<Vec<Vec<Bool>>> {
    (0..a)
        .map(|i| consts2(&format!("{name}_{i}"), b, c))
        .collect()
}

impl Model {
    pub fn new(
        var_amount: usize,
        original_clause_amount: usize,
        learned_clause_amount: usize,
        step_amount: usize,
    ) -> Self {
        let clause_amount = original_clause_amount + learned_clause_amount;
        Model {
            var_amount,
            original_clause_amount,
            clause_amount,
            step_amount,
            Phi: consts3("Phi", step_amount, clause_amount, var_amount * 2),
            Act: consts2("Act", step_amount, clause_amount),
            M: consts3("M", step_amount, var_amount, var_amount * 2),
            D: consts2("D", step_amount, var_amount * 2),
            Ant: consts3("Ant", step_amount, var_amount, clause_amount),
            ordered_decisions: true,
        }
    }

    pub fn from_trace(trace: &Trace, var_amount: usize, clause_amount: usize) -> Self {
        let learned_clause_amount = trace.iter().filter(|s| s.rule == Rule::Conflict).count();
        debug_assert!(
            trace.iter().all(|step| match step.rule {
                Rule::Decide => step.param < var_amount * 2,
                Rule::UnitProp | Rule::Conflict =>
                    step.param < clause_amount + learned_clause_amount,
            }),
            "trace references a literal outside var_amount={var_amount} \
             or a clause outside clause_amount={clause_amount} plus its learned clauses"
        );
        Model::new(
            var_amount,
            clause_amount,
            learned_clause_amount,
            trace.len() + 1,
        )
    }

    pub fn with_ordered_decisions(mut self, ordered: bool) -> Self {
        self.ordered_decisions = ordered;
        self
    }

    pub fn solve(&self, trace: &Trace, sat: bool, timeout: Option<Duration>) -> SolveOutcome {
        debug_assert!(
            trace.len() + 1 == self.step_amount,
            "trace length {} does not match step_amount {}",
            trace.len(),
            self.step_amount
        );

        crate::generate::solve(timeout, || {
            let mut constraints = vec![
                self.no_clause_empty(),
                self.no_taut(),
                self.no_duplicates(),
                self.initial_state(),
                self.result_state(sat),
            ];
            let mut learned_slot = self.original_clause_amount;
            for (t, step) in trace.iter().enumerate() {
                constraints.push(self.convert_rule(t, step, learned_slot));
                if step.rule == Rule::Conflict {
                    learned_slot += 1;
                }
            }
            constraints
        })
    }

    pub(crate) fn convert_rule(&self, t: usize, step: &Step, learned_slot: usize) -> Bool {
        match step.rule {
            Rule::Conflict => self.conflict_rule(t, step.param, learned_slot),
            Rule::UnitProp => self.unit_prop_rule(t, step.param),
            Rule::Decide => self.decide_rule(t, step.param),
        }
    }

    pub fn format_clauses(&self, model: &z3::Model, t: usize) -> String {
        let mut res = String::new();
        for c in (0..self.clause_amount).filter(|&c| holds(model, &self.Act[t][c])) {
            let lits: Vec<String> = (0..self.var_amount * 2)
                .filter(|&l| holds(model, &self.Phi[t][c][l]))
                .map(lit_name)
                .collect();
            res.push_str(&format_clause(c, &lits));
        }
        res
    }

    pub fn format_final_model(&self, model: &z3::Model) -> String {
        self.format_clauses(model, 0)
    }

    pub fn print_final_model(&self, model: &z3::Model) {
        println!("{}", self.format_final_model(model))
    }

    pub fn format_trail(&self, model: &z3::Model, t: usize) -> String {
        let mut slots: Vec<String> = Vec::new();
        for p in 0..self.var_amount {
            for l in (0..self.var_amount * 2).filter(|&l| holds(model, &self.M[t][p][l])) {
                let mark = if holds(model, &self.D[t][l]) {
                    "*".to_string()
                } else {
                    (0..self.clause_amount)
                        .find(|&c| holds(model, &self.Ant[t][p][c]))
                        .map_or(String::new(), |c| format!("(c{c})"))
                };
                slots.push(format!("{}{mark}", lit_name(l)));
            }
        }
        if slots.is_empty() {
            "[]".to_string()
        } else {
            slots.join(" ")
        }
    }

    pub fn step_amount(&self) -> usize {
        self.step_amount
    }
}
