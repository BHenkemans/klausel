use std::time::Duration;

use z3::ast::Bool;

use crate::dpll::trace::{Rule, Step, Trace};
use crate::generate::{SolveOutcome, format_clause, holds, lit_name};

#[allow(non_snake_case)]
pub struct Model {
    pub(crate) var_amount: usize,
    pub(crate) clause_amount: usize,
    pub(crate) step_amount: usize,
    pub(crate) Phi: Vec<Vec<Bool>>,
    pub(crate) M: Vec<Vec<Vec<Bool>>>,
    pub(crate) D: Vec<Vec<Bool>>,
    pub(crate) ordered_decisions: bool,
}

impl Model {
    pub fn new(var_amount: usize, clause_amount: usize, step_amount: usize) -> Self {
        Model {
            var_amount,
            clause_amount,
            step_amount,
            Phi: (0..clause_amount)
                .map(|c| {
                    (0..var_amount * 2)
                        .map(|l| Bool::new_const(format!("Phi_{c}_{l}")))
                        .collect()
                })
                .collect(),
            M: (0..step_amount)
                .map(|t| {
                    (0..var_amount)
                        .map(|p| {
                            (0..var_amount * 2)
                                .map(|l| Bool::new_const(format!("M_{t}_{p}_{l}")))
                                .collect()
                        })
                        .collect()
                })
                .collect(),
            D: (0..step_amount)
                .map(|t| {
                    (0..var_amount * 2)
                        .map(|l| Bool::new_const(format!("D_{t}_{l}")))
                        .collect()
                })
                .collect(),
            ordered_decisions: true,
        }
    }

    pub fn from_trace(trace: &Trace, var_amount: usize, clause_amount: usize) -> Self {
        debug_assert!(
            trace.iter().all(|step| match step.rule {
                Rule::Decide => step.param < var_amount * 2,
                Rule::UnitProp | Rule::Backtrack => step.param < clause_amount,
            }),
            "trace references a literal outside var_amount={var_amount} \
             or a clause outside clause_amount={clause_amount}"
        );
        Model::new(var_amount, clause_amount, trace.len() + 1)
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
            constraints.extend(
                trace
                    .iter()
                    .enumerate()
                    .map(|(t, step)| self.convert_rule(t, step)),
            );
            constraints
        })
    }

    pub(crate) fn convert_rule(&self, t: usize, step: &Step) -> Bool {
        match step.rule {
            Rule::Backtrack => self.backtrack_rule(t, step.param),
            Rule::UnitProp => self.unit_prop_rule(t, step.param),
            Rule::Decide => self.decide_rule(t, step.param),
        }
    }

    pub fn format_final_model(&self, model: &z3::Model) -> String {
        let mut res = String::new();
        for c in 0..self.clause_amount {
            let lits: Vec<String> = (0..self.var_amount * 2)
                .filter(|&l| holds(model, &self.Phi[c][l]))
                .map(lit_name)
                .collect();
            res.push_str(&format_clause(c, &lits));
        }
        res
    }

    pub fn print_final_model(&self, model: &z3::Model) {
        println!("{}", self.format_final_model(model))
    }

    pub fn format_trail(&self, model: &z3::Model, t: usize) -> String {
        let mut slots: Vec<String> = Vec::new();
        for p in 0..self.var_amount {
            for l in (0..self.var_amount * 2).filter(|&l| holds(model, &self.M[t][p][l])) {
                let mark = if holds(model, &self.D[t][l]) { "*" } else { "" };
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
