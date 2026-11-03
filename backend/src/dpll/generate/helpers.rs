use z3::ast::Bool;

use crate::dpll::Model;
use crate::generate::compl;

impl Model {
    /// \texttt{ClauseNotEmpty}(c) \equiv \bigvee_\ell \varphi_{c,\ell}
    /// \texttt{NoClauseEmpty} \equiv \bigwedge_{c}\texttt{ClauseNotEmpty}(c)
    pub(crate) fn no_clause_empty(&self) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let per_lit: Vec<&Bool> =
                    (0..self.var_amount * 2).map(|l| &self.Phi[c][l]).collect();
                Bool::or(&per_lit)
            })
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{ClauseNoTaut}(c) \equiv \bigwedge_{\ell} \left(\varphi_{c,\ell} \implies \lnot \varphi_{c,\ell^c}\right)
    /// \texttt{NoTaut} \equiv \bigwedge_c \texttt{ClauseNoTaut}(c)
    pub(crate) fn no_taut(&self) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|l| self.Phi[c][l].implies(self.Phi[c][compl(l)].not()))
                    .collect();
                Bool::and(&per_lit)
            })
            .collect();
        Bool::and(&per_clause)
    }

    /// \lnot\bigvee_{c_1<c_2}\bigwedge_{\ell}\varphi_{c_1,\ell} \iff \varphi_{c_2,\ell}
    pub(crate) fn no_duplicates(&self) -> Bool {
        let mut per_pair: Vec<Bool> = Vec::new();
        for c1 in 0..self.clause_amount {
            for c2 in (c1 + 1)..self.clause_amount {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|l| self.Phi[c1][l].iff(&self.Phi[c2][l]))
                    .collect();
                per_pair.push(Bool::and(&per_lit));
            }
        }
        Bool::or(&per_pair).not()
    }

    pub(crate) fn initial_state(&self) -> Bool {
        let mut parts: Vec<Bool> = Vec::new();
        for p in 0..self.var_amount {
            for l in 0..self.var_amount * 2 {
                parts.push(self.M[0][p][l].not());
            }
        }
        for l in 0..self.var_amount * 2 {
            parts.push(self.D[0][l].not());
        }
        Bool::and(&parts)
    }

    pub(crate) fn result_state(&self, sat: bool) -> Bool {
        let t = self.step_amount - 1;
        if sat { self.sat(t) } else { self.unsat(t) }
    }

    /// \texttt{UNSAT}(t) \equiv \texttt{Conflict}(t) \land \lnot \texttt{DecisionOnTrail}(t)
    fn unsat(&self, t: usize) -> Bool {
        Bool::and(&[&self.conflict(t), &self.decision_on_trail(t).not()])
    }

    /// \texttt{OnTrail}(t,\ell) \equiv \bigvee_{p}M_{t,p,\ell}
    pub(crate) fn on_trail(&self, t: usize, l: usize) -> Bool {
        let per_slot: Vec<&Bool> = (0..self.var_amount).map(|p| &self.M[t][p][l]).collect();
        Bool::or(&per_slot)
    }

    /// \texttt{DecisionOnTrail}(t) \equiv \bigvee_{\ell} D_{t,\ell}
    pub(crate) fn decision_on_trail(&self, t: usize) -> Bool {
        let per_lit: Vec<&Bool> = (0..self.var_amount * 2).map(|l| &self.D[t][l]).collect();
        Bool::or(&per_lit)
    }

    /// \texttt{Next}(t,p) \equiv \bigwedge_\ell \lnot M_{t,p,\ell} \land \bigwedge_{p' < p}\bigvee_{\ell} M_{t,p',\ell}
    pub(crate) fn next(&self, t: usize, p: usize) -> Bool {
        let empty: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| self.M[t][p][l].not())
            .collect();
        let earlier_occupied: Vec<Bool> = (0..p)
            .map(|pp| {
                let per_lit: Vec<&Bool> = (0..self.var_amount * 2)
                    .map(|l| &self.M[t][pp][l])
                    .collect();
                Bool::or(&per_lit)
            })
            .collect();
        Bool::and(&[&Bool::and(&empty), &Bool::and(&earlier_occupied)])
    }

    /// \texttt{SAT}(t) \equiv \bigwedge_{c}\bigvee_{\ell}\left(\varphi_{c,\ell} \land \bigvee_{p} M_{t,p,\ell}\right)
    pub(crate) fn sat(&self, t: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|l| Bool::and(&[&self.Phi[c][l], &self.on_trail(t, l)]))
                    .collect();
                Bool::or(&per_lit)
            })
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{ConflictInClause}(t,c) \equiv \bigwedge_\ell (\varphi_{c,\ell}\implies \bigvee_{q}M_{t,q,\ell^c})
    pub(crate) fn conflict_in_clause(&self, t: usize, c: usize) -> Bool {
        let per_lit: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| self.Phi[c][l].implies(self.on_trail(t, compl(l))))
            .collect();
        Bool::and(&per_lit)
    }

    /// \texttt{Conflict}(t) \equiv \bigvee_{c}\bigwedge_{\ell}\left(\varphi_{c,\ell} \implies \bigvee_{p} M_{t,p,\ell^c}\right)
    pub(crate) fn conflict(&self, t: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| self.conflict_in_clause(t, c))
            .collect();
        Bool::or(&per_clause)
    }

    /// M_{t+1,p,\ell} \land \bigwedge_{k\neq\ell}\lnot M_{t+1,p,k}
    fn trail_push_at_p(&self, t: usize, p: usize, l: usize) -> Bool {
        let slot_exclusive: Vec<Bool> = (0..self.var_amount * 2)
            .filter(|&k| k != l)
            .map(|k| self.M[t + 1][p][k].not())
            .collect();
        Bool::and(&[&self.M[t + 1][p][l], &Bool::and(&slot_exclusive)])
    }

    /// \bigwedge_{p'<p}\bigwedge_{k}\left(M_{t,p',k} \iff M_{t+1,p',k}\right)
    pub(crate) fn trail_kept_before(&self, t: usize, p: usize) -> Bool {
        let mut before_p: Vec<Bool> = Vec::new();
        for pp in 0..p {
            for k in 0..self.var_amount * 2 {
                before_p.push(self.M[t][pp][k].iff(&self.M[t + 1][pp][k]));
            }
        }
        Bool::and(&before_p)
    }

    /// \bigwedge_{p'>p}\bigwedge_{k}\lnot M_{t+1,p',k}
    pub(crate) fn trail_empty_after(&self, t: usize, p: usize) -> Bool {
        let mut after_p: Vec<Bool> = Vec::new();
        for pp in (p + 1)..self.var_amount {
            for k in 0..self.var_amount * 2 {
                after_p.push(self.M[t + 1][pp][k].not());
            }
        }
        Bool::and(&after_p)
    }

    /// \bigwedge_{p}\Bigg(\texttt{Next}(t,p) \implies \bigg( M_{t+1,p,\ell} \land \bigwedge_{k\neq\ell}\lnot M_{t+1,p,k}
    ///     \land \bigwedge_{p'<p}\bigwedge_{k}\left(M_{t,p',k} \iff M_{t+1,p',k}\right)
    ///     \land \bigwedge_{p'>p}\bigwedge_{k}\lnot M_{t+1,p',k} \bigg)\Bigg)
    pub(crate) fn trail_push(&self, t: usize, l: usize) -> Bool {
        let per_p_slot: Vec<Bool> = (0..self.var_amount)
            .map(|p| {
                let consequent = Bool::and(&[
                    &self.trail_push_at_p(t, p, l),
                    &self.trail_kept_before(t, p),
                    &self.trail_empty_after(t, p),
                ]);
                self.next(t, p).implies(&consequent)
            })
            .collect();
        Bool::and(&per_p_slot)
    }

    /// \bigwedge_{k\neq\ell}\left(D_{t,k} \iff D_{t+1,k}\right)
    pub(crate) fn other_decisions_kept(&self, t: usize, l: usize) -> Bool {
        let per_lit: Vec<Bool> = (0..self.var_amount * 2)
            .filter(|&k| k != l)
            .map(|k| self.D[t][k].iff(&self.D[t + 1][k]))
            .collect();
        Bool::and(&per_lit)
    }
}
