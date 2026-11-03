use z3::ast::Bool;

use crate::davis_putnam::Model;
use crate::generate::compl;

impl Model {
    /// \texttt{NoPure}(t) \equiv \bigwedge_{\ell' \in L}\left(\left(\bigvee_{c\in C} \big(A_{t,c} \land X_{t,c,\ell'}\big)\right) \implies \left(\bigvee_{c\in C} \big(A_{t,c} \land X_{t,c,(\ell')^c}\big)\right)\right)
    fn no_pure(&self, t: usize) -> Bool {
        let per_lit: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| {
                let occurs_l: Vec<Bool> = (0..self.clause_amount)
                    .map(|c| Bool::and(&[&self.A[t][c], &self.X[t][l][c]]))
                    .collect();
                let occurs_compl: Vec<Bool> = (0..self.clause_amount)
                    .map(|c| Bool::and(&[&self.A[t][c], &self.X[t][compl(l)][c]]))
                    .collect();
                Bool::or(&occurs_l).implies(&Bool::or(&occurs_compl))
            })
            .collect();
        Bool::and(&per_lit)
    }

    /// \texttt{ElimOccurs}(t,p) \equiv \bigvee_{c\in C} \big(A_{t,c} \land X_{t,c,\ell}\big)
    fn elim_occurs(&self, t: usize, atom: usize) -> Bool {
        let p: usize = atom * 2;
        let occurs_p: Vec<Bool> = (0..self.clause_amount)
            .map(|c| Bool::and(&[&self.A[t][c], &self.X[t][p][c]]))
            .collect();
        Bool::or(&occurs_p)
    }

    /// \texttt{None}(t,p,c) \equiv A_{t,c} \land \lnot X_{t,c,\ell} \land \lnot X_{t,c,\ell^c}
    fn none(&self, t: usize, c: usize, p: usize) -> Bool {
        Bool::and(&[
            &self.A[t][c],
            &self.X[t][p][c].not(),
            &self.X[t][compl(p)][c].not(),
        ])
    }

    /// \texttt{Clash}(t,p,c,d) \equiv A_{t,c} \land X_{t,c,\ell} \land A_{t,d} \land X_{t,d,\ell^c}
    fn clash(&self, t: usize, c: usize, d: usize, p: usize) -> Bool {
        Bool::and(&[
            &self.A[t][c],
            &self.X[t][p][c],
            &self.A[t][d],
            &self.X[t][compl(p)][d],
        ])
    }

    /// \texttt{Copy}(t,e,c) \equiv \bigwedge_{k \in L} \big( X_{t+1,e,k} \iff X_{t,c,k} \big)
    fn copy(&self, t: usize, c: usize, e: usize) -> Bool {
        let per_var: Vec<Bool> = (0..self.var_amount * 2)
            .map(|lp| self.X[t][lp][c].iff(&self.X[t + 1][lp][e]))
            .collect();
        Bool::and(&per_var)
    }

    /// \texttt{Res}(t,p,c,d,e) \equiv \lnot X_{t+1,e,\ell} \land \lnot X_{t+1,e,\ell^c}
    ///     \land \bigwedge_{k \in L\setminus\{\ell, \ell^c\}}\big(X_{t+1,e,k}\iff(X_{t,c,k}\lor X_{t,d,k})\big)
    fn res(&self, t: usize, c: usize, d: usize, e: usize, atom: usize) -> Bool {
        let p: usize = atom * 2;
        let pc: usize = compl(p);
        let rhs: Vec<Bool> = (0..self.var_amount * 2)
            .filter(|k| k != &p && k != &pc)
            .map(|k| self.X[t + 1][k][e].iff(&Bool::or(&[&self.X[t][k][c], &self.X[t][k][d]])))
            .collect();
        let rhs_and: Bool = Bool::and(&rhs);
        Bool::and(&[
            &self.X[t + 1][p][e].not(),
            &self.X[t + 1][pc][e].not(),
            &rhs_and,
        ])
    }

    /// \texttt{TautRes}(t,p,c,d) \equiv \bigvee_{k\in V,\,k\neq p}\big((X_{t,c,k}\lor X_{t,d,k}) \land (X_{t,c,k^c}\lor X_{t,d,k^c})\big)
    fn taut_res(&self, t: usize, c: usize, d: usize, atom: usize) -> Bool {
        let per_atom: Vec<Bool> = (0..self.var_amount)
            .filter(|&r| r != atom)
            .map(|r| {
                let pos = Bool::or(&[&self.X[t][2 * r][c], &self.X[t][2 * r][d]]);
                let neg = Bool::or(&[&self.X[t][2 * r + 1][c], &self.X[t][2 * r + 1][d]]);
                Bool::and(&[&pos, &neg])
            })
            .collect();
        Bool::or(&per_atom)
    }

    /// \texttt{ElimCopy}(t,p) \equiv \bigwedge_{c\in C} \left(\texttt{None}(t,p,c) \implies A_{t+1,c} \land \texttt{Copy}(t,c,c)\right)
    fn elim_copy(&self, t: usize, atom: usize) -> Bool {
        let p = 2 * atom;
        let per_c: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let kept = Bool::and(&[&self.A[t + 1][c], &self.copy(t, c, c)]);
                self.none(t, c, p).implies(&kept)
            })
            .collect();
        Bool::and(&per_c)
    }

    /// \texttt{ElimResolvents}(t,p) \equiv \bigwedge_{c,d\in C}\Big(\big(\texttt{Clash}(t,p,c,d)\land\lnot \texttt{TautRes}(t,p,c,d)\big)
    ///     \implies \bigvee_{e\in C} \big(A_{t+1,e} \land \texttt{Res}(t,p,c,d,e)\big)\Big)
    fn elim_resolvents(&self, t: usize, atom: usize) -> Bool {
        let p = 2 * atom;
        let mut per_cd: Vec<Bool> = Vec::new();
        for c in 0..self.clause_amount {
            for d in 0..self.clause_amount {
                let per_e: Vec<Bool> = (0..self.clause_amount)
                    .map(|e| Bool::and(&[&self.A[t + 1][e], &self.res(t, c, d, e, atom)]))
                    .collect();
                let antecedent =
                    Bool::and(&[&self.clash(t, c, d, p), &self.taut_res(t, c, d, atom).not()]);
                per_cd.push(antecedent.implies(&Bool::or(&per_e)));
            }
        }
        Bool::and(&per_cd)
    }

    /// \texttt{ElimOnly}(t,p) \equiv \bigwedge_{e\in C}\Big(A_{t+1, e} \implies \Big[ \big(\texttt{None}(t,p,e) \land \texttt{Copy}(t,e,e)\big)
    ///     \lor \bigvee_{c,d\in C}\big(\texttt{Clash}(t,p,c,d)\land\lnot \texttt{TautRes}(t,p,c,d)
    ///     \land \texttt{Res}(t,p,c,d,e)\big) \Big]\Big)
    fn elim_only(&self, t: usize, atom: usize) -> Bool {
        let p = 2 * atom;
        let per_e: Vec<Bool> = (0..self.clause_amount)
            .map(|e| {
                let kept = Bool::and(&[&self.none(t, e, p), &self.copy(t, e, e)]);

                let mut res: Vec<Bool> = Vec::new();
                for c in 0..self.clause_amount {
                    for d in 0..self.clause_amount {
                        res.push(Bool::and(&[
                            &self.clash(t, c, d, p),
                            &self.taut_res(t, c, d, atom).not(),
                            &self.res(t, c, d, e, atom),
                        ]));
                    }
                }

                let res = Bool::or(&[&kept, &Bool::or(&res)]);
                self.A[t + 1][e].implies(&res)
            })
            .collect();
        Bool::and(&per_e)
    }

    /// \texttt{Elim}(t,p) \equiv \texttt{ElimGuard}(t,p) \land \texttt{ElimEffect}(t,p)
    /// \texttt{ElimGuard}(t,p) \equiv \texttt{ElimOccurs}(t,p) \land \texttt{AtLeastTwo}(t) \land \texttt{NoPure}(t)
    /// \texttt{ElimEffect}(t,p) \equiv \texttt{ElimCopy}(t,p) \land \texttt{ElimResolvents}(t,p) \land \texttt{ElimOnly}(t,p)
    pub fn elim_rule(&self, t: usize, atom: usize) -> Bool {
        Bool::and(&[
            &self.elim_occurs(t, atom),
            &self.at_least_two(t),
            &self.no_pure(t),
            &self.elim_copy(t, atom),
            &self.elim_resolvents(t, atom),
            &self.elim_only(t, atom),
        ])
    }
}
