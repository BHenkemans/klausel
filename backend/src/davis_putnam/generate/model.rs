use std::time::Duration;

use z3::ast::Bool;

use crate::davis_putnam::generate::trace::trace_var_amount;
use crate::davis_putnam::trace::{Rule, Step, Trace};
use crate::generate::{SolveOutcome, format_clause, holds, lit_name};

#[allow(non_snake_case)]
pub struct Model {
    pub(crate) var_amount: usize,
    pub(crate) initial_clause_amount: usize,
    pub(crate) clause_amount: usize,
    pub(crate) step_amount: usize,
    pub(crate) A: Vec<Vec<Bool>>,
    pub(crate) X: Vec<Vec<Vec<Bool>>>,
}

impl Model {
    pub fn new(var_amount: usize, initial_clause_amount: usize, step_amount: usize) -> Self {
        let k = initial_clause_amount;
        let clause_amount = (k + 3).min(k * 5 / 4).max(k);
        Model {
            var_amount,
            initial_clause_amount,
            clause_amount,
            step_amount,
            A: (0..step_amount)
                .map(|s| {
                    (0..clause_amount)
                        .map(|c| Bool::new_const(format!("A_{s}_{c}")))
                        .collect()
                })
                .collect(),
            X: (0..step_amount)
                .map(|s| {
                    (0..var_amount * 2)
                        .map(|l| {
                            (0..clause_amount)
                                .map(|c| Bool::new_const(format!("X_{s}_{l}_{c}")))
                                .collect()
                        })
                        .collect()
                })
                .collect(),
        }
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
                self.result_state(sat),
                self.initial_clause_bound(),
                self.no_copies(0),
            ];
            constraints.extend((0..self.step_amount).map(|t| self.no_taut(t)));
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
            Rule::Unit => self.unit_rule(t, step.param),
            Rule::Pure => self.pure_rule(t, step.param),
            Rule::Elim => self.elim_rule(t, step.param),
        }
    }

    pub fn from_trace(trace: &Trace, initial_clause_amount: usize) -> Self {
        let var_amount = trace_var_amount(trace);
        let step_amount = trace.len() + 1;

        Model::new(var_amount, initial_clause_amount, step_amount)
    }

    pub fn from_trace_with_vars(
        trace: &Trace,
        initial_clause_amount: usize,
        var_amount: usize,
    ) -> Self {
        debug_assert!(
            trace
                .iter()
                .map(|step| match step.rule {
                    Rule::Unit | Rule::Pure => step.param / 2,
                    Rule::Elim => step.param,
                })
                .max()
                .is_none_or(|m| m < var_amount),
            "trace references a variable outside var_amount={var_amount}"
        );
        let step_amount = trace.len() + 1;
        Model::new(var_amount, initial_clause_amount, step_amount)
    }

    pub fn format_final_model(&self, model: &z3::Model) -> String {
        let mut res = String::new();
        for c in (0..self.clause_amount).filter(|&c| holds(model, &self.A[0][c])) {
            let lits: Vec<String> = (0..self.var_amount * 2)
                .filter(|&l| holds(model, &self.X[0][l][c]))
                .map(lit_name)
                .collect();
            res.push_str(&format_clause(c, &lits));
        }
        res
    }

    pub fn print_final_model(&self, model: &z3::Model) {
        println!("{}", self.format_final_model(model))
    }

    pub fn format_final_model_variable_ordered(&self, model: &z3::Model, perm: &[usize]) -> String {
        debug_assert!(
            perm.len() == self.var_amount,
            "perm length {} does not match var_amount {}",
            perm.len(),
            self.var_amount
        );

        let mut clauses: Vec<(usize, Vec<(usize, bool)>)> = Vec::new();

        for c in (0..self.clause_amount).filter(|&c| holds(model, &self.A[0][c])) {
            let mut lits: Vec<(usize, bool)> = (0..self.var_amount * 2)
                .filter(|&l| holds(model, &self.X[0][l][c]))
                .map(|l| (perm[l / 2], l % 2 == 1))
                .collect();

            lits.sort();
            let sort_key = lits.first().map(|(v, _)| *v).unwrap_or(usize::MAX);
            clauses.push((sort_key, lits));
        }

        clauses.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));

        let mut res = String::new();
        for (idx, (_, lits)) in clauses.iter().enumerate() {
            let names: Vec<String> = lits
                .iter()
                .map(|&(v, is_neg)| lit_name(2 * v + usize::from(is_neg)))
                .collect();
            res.push_str(&format_clause(idx, &names));
        }
        res
    }

    pub fn print_final_model_variable_ordered(&self, model: &z3::Model, perm: &[usize]) {
        println!("{}", self.format_final_model_variable_ordered(model, perm))
    }

    pub fn var_amount(&self) -> usize {
        self.var_amount
    }
}
