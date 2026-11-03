use z3::ast::{Bool, Int};

use crate::davis_putnam::Model;

impl Model {
    /// \texttt{AtLeastTwo}(t) \equiv \bigwedge_{c\in C}\left(A_{t,c}\implies \bigvee_{\ell',\ell''\in L,\ \ell' < \ell''} \big( X_{t,c,\ell'}\land X_{t,c,\ell''}\big)\right)
    pub(crate) fn at_least_two(&self, t: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let terms: Vec<(&Bool, i32)> = (0..self.var_amount * 2)
                    .map(|lp| (&self.X[t][lp][c], 1))
                    .collect();
                self.A[t][c].implies(&Bool::pb_ge(&terms, 2))
            })
            .collect();
        Bool::and(&per_clause)
    }

    pub(crate) fn result_state(&self, sat: bool) -> Bool {
        let t = self.step_amount - 1;
        if sat { self.sat(t) } else { self.unsat(t) }
    }

    /// \texttt{SAT}(t) \equiv \bigwedge_{c\in C} \lnot A_{t,c}
    fn sat(&self, t: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| self.A[t][c].not())
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{UNSAT}(t) \equiv \bigvee_{c\in C}\left( A_{t,c} \land \bigwedge_{\ell'\in L} \lnot X_{t,c,\ell'} \right)
    pub(crate) fn unsat(&self, t: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|lp| self.X[t][lp][c].not())
                    .collect();
                Bool::and(&[&self.A[t][c], &Bool::and(&per_lit)])
            })
            .collect();
        Bool::or(&per_clause)
    }

    pub(crate) fn active_clause_count(&self, t: usize) -> Int {
        let zero: Int = Int::from_i64(0);
        let one: Int = Int::from_i64(1);
        (0..self.clause_amount)
            .map(|c| self.A[t][c].ite(&one, &zero))
            .sum()
    }

    pub(crate) fn initial_clause_bound(&self) -> Bool {
        self.active_clause_count(0)
            .le(Int::from_i64(self.initial_clause_amount as i64))
    }

    pub(crate) fn no_copies(&self, t: usize) -> Bool {
        let mut per_pair: Vec<Bool> = Vec::new();
        for c in 0..self.clause_amount {
            for d in (c + 1)..self.clause_amount {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|l| self.X[t][l][c].iff(&self.X[t][l][d]))
                    .collect();
                let both_active = Bool::and(&[&self.A[t][c], &self.A[t][d]]);
                per_pair.push(both_active.implies(&Bool::and(&per_lit).not()));
            }
        }
        Bool::and(&per_pair)
    }

    /// \texttt{NoTaut}(t) \equiv \bigwedge_{c\in C} A_{t,c} \implies \bigwedge_{\ell\in V} \lnot\left(X_{t,c,\ell} \land X_{t,c,-\ell}\right)
    pub(crate) fn no_taut(&self, t: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let per_atom: Vec<Bool> = (0..self.var_amount)
                    .map(|j| Bool::and(&[&self.X[t][2 * j][c], &self.X[t][2 * j + 1][c]]).not())
                    .collect();
                self.A[t][c].implies(&Bool::and(&per_atom))
            })
            .collect();
        Bool::and(&per_clause)
    }
}
