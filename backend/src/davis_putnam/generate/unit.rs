use z3::ast::Bool;

use crate::davis_putnam::Model;
use crate::generate::compl;

impl Model {
    /// \bigvee_{c\in C}\left( A_{t,c} \;\land\; X_{t,c,\ell} \land \bigwedge_{\ell'\in L,\,\ell'\ne \ell} \lnot X_{t,c,\ell'}\right)
    fn unit_literal_present(&self, t: usize, l: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let others_negated: Vec<Bool> = (0..self.var_amount * 2)
                    .filter(|&lp| lp != l)
                    .map(|lp| self.X[t][lp][c].not())
                    .collect();
                Bool::and(&[&self.A[t][c], &self.X[t][l][c], &Bool::and(&others_negated)])
            })
            .collect();
        Bool::or(&per_clause)
    }

    /// \texttt{UnitClauses}(t,\ell) \equiv \bigwedge_{c\in C}\left( A_{t+1,c} \iff (A_{t,c} \land \lnot X_{t,c,\ell})\right)
    fn unit_clauses(&self, t: usize, l: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let rhs: Bool = Bool::and(&[&self.A[t][c], &self.X[t][l][c].not()]);
                self.A[t + 1][c].iff(&rhs)
            })
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{UnitLiterals}(t,\ell) \equiv \bigwedge_{c\in C}\left( A_{t+1,c} \implies \left( \bigwedge_{\ell'\in L,\,\ell'\ne \ell^c} \big( X_{t,c,\ell'} \iff X_{t+1,c,\ell'} \big) \land \lnot X_{t+1,c,\ell^c} \right) \right)
    fn unit_literals(&self, t: usize, l: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let rhs_1: Vec<Bool> = (0..self.var_amount * 2)
                    .filter(|&lp| lp != compl(l))
                    .map(|lp| self.X[t][lp][c].iff(&self.X[t + 1][lp][c]))
                    .collect();
                let rhs = Bool::and(&[&self.X[t + 1][compl(l)][c].not(), &Bool::and(&rhs_1)]);
                self.A[t + 1][c].implies(&rhs)
            })
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{Unit}(t,\ell) \equiv \texttt{UnitGuard}(t,\ell) \land \texttt{UnitEffect}(t,\ell)
    /// \texttt{UnitGuard}(t,\ell) \equiv \lnot\text{UNSAT}(t) \land\bigvee_{c\in C}\left( A_{t,c} \;\land\; X_{t,c,\ell} \land \bigwedge_{\ell'\in L,\,\ell'\ne \ell} \lnot X_{t,c,\ell'}\right)
    /// \texttt{UnitEffect}(t,\ell) \equiv \texttt{UnitClauses}(t,\ell) \land \texttt{UnitLiterals}(t,\ell)
    pub fn unit_rule(&self, t: usize, l: usize) -> Bool {
        Bool::and(&[
            &self.unsat(t).not(),
            &self.unit_literal_present(t, l),
            &self.unit_clauses(t, l),
            &self.unit_literals(t, l),
        ])
    }
}
