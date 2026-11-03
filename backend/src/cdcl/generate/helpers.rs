use z3::ast::Bool;

use crate::cdcl::Model;
use crate::generate::compl;

impl Model {
    /// \texttt{NoClauseEmpty} \equiv \bigwedge_{c < K_0}\bigvee_\ell \varphi_{0,c,\ell}
    pub(crate) fn no_clause_empty(&self) -> Bool {
        let per_clause: Vec<Bool> = (0..self.original_clause_amount)
            .map(|c| {
                let per_lit: Vec<&Bool> = (0..self.var_amount * 2)
                    .map(|l| &self.Phi[0][c][l])
                    .collect();
                Bool::or(&per_lit)
            })
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{NoTaut} \equiv \bigwedge_{c < K_0}\bigwedge_{\ell} \left(\varphi_{0,c,\ell} \implies \lnot \varphi_{0,c,\ell^c}\right)
    pub(crate) fn no_taut(&self) -> Bool {
        let per_clause: Vec<Bool> = (0..self.original_clause_amount)
            .map(|c| {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|l| self.Phi[0][c][l].implies(self.Phi[0][c][compl(l)].not()))
                    .collect();
                Bool::and(&per_lit)
            })
            .collect();
        Bool::and(&per_clause)
    }

    /// \lnot\bigvee_{c_1<c_2<K_0}\bigwedge_{\ell}\left(\varphi_{0,c_1,\ell} \iff \varphi_{0,c_2,\ell}\right)
    pub(crate) fn no_duplicates(&self) -> Bool {
        let mut per_pair: Vec<Bool> = Vec::new();
        for c1 in 0..self.original_clause_amount {
            for c2 in (c1 + 1)..self.original_clause_amount {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|l| self.Phi[0][c1][l].iff(&self.Phi[0][c2][l]))
                    .collect();
                per_pair.push(Bool::and(&per_lit));
            }
        }
        Bool::or(&per_pair).not()
    }

    /// \texttt{ClauseKept}(t,c) \equiv \left(\texttt{Act}_{t,c} \iff \texttt{Act}_{t+1,c}\right) \land \bigwedge_{k}\left(\varphi_{t,c,k} \iff \varphi_{t+1,c,k}\right)
    pub(crate) fn clause_kept(&self, t: usize, c: usize) -> Bool {
        let per_lit: Vec<Bool> = (0..self.var_amount * 2)
            .map(|k| self.Phi[t][c][k].iff(&self.Phi[t + 1][c][k]))
            .collect();
        Bool::and(&[
            &self.Act[t][c].iff(&self.Act[t + 1][c]),
            &Bool::and(&per_lit),
        ])
    }

    /// \bigwedge_{c}\texttt{ClauseKept}(t,c)
    pub(crate) fn all_clauses_kept(&self, t: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| self.clause_kept(t, c))
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{AntecedentsKept}(t,p) \equiv \bigwedge_{c}\left(\texttt{Ant}_{t,p,c} \iff \texttt{Ant}_{t+1,p,c}\right)
    pub(crate) fn antecedents_kept(&self, t: usize, p: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| self.Ant[t][p][c].iff(&self.Ant[t + 1][p][c]))
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{Init} \equiv \bigwedge_{p}\bigwedge_{\ell}\lnot M_{0,p,\ell} \land \bigwedge_{p}\bigwedge_{c}\lnot \texttt{Ant}_{0,p,c} \land \bigwedge_{\ell}\lnot D_{0,\ell}
    ///     \land \bigwedge_{c < K_0}\texttt{Act}_{0,c} \land \bigwedge_{c \geq K_0}\left(\lnot\texttt{Act}_{0,c} \land \bigwedge_{\ell}\lnot\varphi_{0,c,\ell}\right)
    pub(crate) fn initial_state(&self) -> Bool {
        let mut parts: Vec<Bool> = Vec::new();
        for p in 0..self.var_amount {
            for l in 0..self.var_amount * 2 {
                parts.push(self.M[0][p][l].not());
            }
            for c in 0..self.clause_amount {
                parts.push(self.Ant[0][p][c].not());
            }
        }
        for l in 0..self.var_amount * 2 {
            parts.push(self.D[0][l].not());
        }
        for c in 0..self.clause_amount {
            if c < self.original_clause_amount {
                parts.push(self.Act[0][c].clone());
            } else {
                parts.push(self.Act[0][c].not());
                for l in 0..self.var_amount * 2 {
                    parts.push(self.Phi[0][c][l].not());
                }
            }
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

    /// \texttt{SAT}(t) \equiv \bigwedge_{c} \left(\texttt{Act}_{t,c} \implies \bigvee_{\ell}\left( \varphi_{t,c,\ell} \land \bigvee_{p} M_{t,p,\ell}\right)\right)
    pub(crate) fn sat(&self, t: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|l| Bool::and(&[&self.Phi[t][c][l], &self.on_trail(t, l)]))
                    .collect();
                self.Act[t][c].implies(Bool::or(&per_lit))
            })
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{ConflictInClause}(t,c) \equiv \texttt{Act}_{t,c}\land \bigwedge_{\ell} \left(\varphi_{t,c,\ell} \implies \texttt{OnTrail}(t,\ell^c)\right)
    pub(crate) fn conflict_in_clause(&self, t: usize, c: usize) -> Bool {
        let per_lit: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| self.Phi[t][c][l].implies(self.on_trail(t, compl(l))))
            .collect();
        Bool::and(&[&self.Act[t][c], &Bool::and(&per_lit)])
    }

    /// \texttt{Conflict}(t) \equiv \bigvee_{c} \left(\texttt{Act}_{t,c} \land \bigwedge_{\ell}\left( \varphi_{t,c,\ell} \implies \bigvee_{p} M_{t,p,\ell^c}\right)\right)
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
