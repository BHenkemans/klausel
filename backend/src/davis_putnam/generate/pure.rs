use z3::ast::Bool;

use crate::davis_putnam::Model;
use crate::generate::compl;

impl Model {
    /// \texttt{PureOccurs}(t,\ell) \equiv \left(\bigvee_{c\in C} \big(A_{t,c} \land X_{t,c,\ell}\big)\right) \land \left( \bigwedge_{c\in C} \big(A_{t,c} \implies \lnot X_{t,c,\ell^c}\big)\right)
    fn pure_occurs(&self, t: usize, l: usize) -> Bool {
        let occurs_vec: Vec<Bool> = (0..self.clause_amount)
            .map(|c| Bool::and(&[&self.A[t][c], &self.X[t][l][c]]))
            .collect();
        let occurs = Bool::or(&occurs_vec);

        let absent_vec: Vec<Bool> = (0..self.clause_amount)
            .map(|c| self.A[t][c].implies(&self.X[t][compl(l)][c].not()))
            .collect();
        let comp_absent = Bool::and(&absent_vec);

        Bool::and(&[&occurs, &comp_absent])
    }

    /// \texttt{PureDelete}(t,\ell) \equiv \bigwedge_{c\in C} \left(A_{t+1,c}\iff (A_{t,c} \land \lnot X_{t,c,\ell})\right)
    fn pure_delete(&self, t: usize, l: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| self.A[t + 1][c].iff(&Bool::and(&[&self.A[t][c], &self.X[t][l][c].not()])))
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{PureCopy}(t,\ell) \equiv \bigwedge_{c\in C} \left(A_{t+1,c} \implies \bigwedge_{\ell'\in L}\big(X_{t,c,\ell'} \iff X_{t+1,c,\ell'}\big) \right)
    fn pure_copy(&self, t: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let rhs: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|lp| self.X[t][lp][c].iff(&self.X[t + 1][lp][c]))
                    .collect();
                self.A[t + 1][c].implies(&Bool::and(&rhs))
            })
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{Pure}(t,\ell) \equiv \texttt{PureGuard}(t,\ell) \land \texttt{PureEffect}(t,\ell)
    /// \texttt{PureGuard}(t,\ell) \equiv \texttt{PureOccurs}(t,\ell) \land \texttt{AtLeastTwo}(t)
    /// \texttt{PureEffect}(t,\ell) \equiv \texttt{PureDelete}(t,\ell) \land \texttt{PureCopy}(t,\ell)
    pub fn pure_rule(&self, t: usize, l: usize) -> Bool {
        Bool::and(&[
            &self.pure_occurs(t, l),
            &self.at_least_two(t),
            &self.pure_delete(t, l),
            &self.pure_copy(t),
        ])
    }
}
