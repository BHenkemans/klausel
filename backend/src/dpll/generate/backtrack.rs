use z3::ast::Bool;

use crate::dpll::Model;
use crate::generate::compl;

impl Model {
    /// \texttt{LastDec}(t,q) \equiv \bigvee_{\ell} (M_{t,q,\ell} \land D_{t,\ell})
    ///     \land \bigwedge_{q'>q}\bigwedge_\ell\left(M_{t,q',\ell} \implies \lnot D_{t,\ell}\right)
    fn last_dec(&self, t: usize, q: usize) -> Bool {
        let dec_here: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| Bool::and(&[&self.M[t][q][l], &self.D[t][l]]))
            .collect();

        let mut none_further: Vec<Bool> = Vec::new();
        for qp in (q + 1)..self.var_amount {
            for l in 0..self.var_amount * 2 {
                none_further.push(self.M[t][qp][l].implies(self.D[t][l].not()));
            }
        }

        Bool::and(&[&Bool::or(&dec_here), &Bool::and(&none_further)])
    }

    /// \bigwedge_\ell \left(M_{t,q,\ell}\implies \left(M_{t+1,q,\ell^c} \land \bigwedge_{k\neq \ell^c} \lnot M_{t+1,q,k}\right)\right)
    fn backtrack_flip(&self, t: usize, q: usize) -> Bool {
        let flipped: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| {
                let slot_free: Vec<Bool> = (0..self.var_amount * 2)
                    .filter(|&k| k != compl(l))
                    .map(|k| self.M[t + 1][q][k].not())
                    .collect();
                self.M[t][q][l].implies(Bool::and(&[
                    &self.M[t + 1][q][compl(l)],
                    &Bool::and(&slot_free),
                ]))
            })
            .collect();
        Bool::and(&flipped)
    }

    /// \bigwedge_{\ell} \left( D_{t+1,\ell} \iff \left(D_{t,\ell} \land \bigvee_{p<q} M_{t,p,\ell}\right)\right)
    fn backtrack_decisions_kept(&self, t: usize, q: usize) -> Bool {
        let decisions_kept: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| {
                let below: Vec<&Bool> = (0..q).map(|p| &self.M[t][p][l]).collect();
                self.D[t + 1][l].iff(Bool::and(&[&self.D[t][l], &Bool::or(&below)]))
            })
            .collect();
        Bool::and(&decisions_kept)
    }

    /// \texttt{BacktrackEffect}(t) \equiv \bigwedge_q \Big(\texttt{LastDec}(t,q) \implies \bigwedge_{q' > q}\bigwedge_{\ell} \lnot M_{t+1,q',\ell}
    ///     \land \bigwedge_{q' < q}\bigwedge_{\ell}\left(M_{t,q',\ell} \iff M_{t+1,q',\ell}\right)
    ///     \land \bigwedge_\ell \left(M_{t,q,\ell}\implies \left(M_{t+1,q,\ell^c} \land \bigwedge_{k\neq \ell^c} \lnot M_{t+1,q,k}\right)\right)
    ///     \land \bigwedge_{\ell} \left( D_{t+1,\ell} \iff \left(D_{t,\ell} \land \bigvee_{p<q} M_{t,p,\ell}\right)\right)\Big)
    fn backtrack_effect(&self, t: usize) -> Bool {
        let per_q_slot: Vec<Bool> = (0..self.var_amount)
            .map(|q| {
                let consequent = Bool::and(&[
                    &self.trail_empty_after(t, q),
                    &self.trail_kept_before(t, q),
                    &self.backtrack_flip(t, q),
                    &self.backtrack_decisions_kept(t, q),
                ]);
                self.last_dec(t, q).implies(&consequent)
            })
            .collect();
        Bool::and(&per_q_slot)
    }

    /// \texttt{Backtrack}(t, c) \equiv \texttt{BacktrackGuard}(t, c) \land \texttt{BacktrackEffect}(t)
    /// \texttt{BacktrackGuard}(t,c) \equiv \texttt{ConflictInClause}(t,c) \land \texttt{DecisionOnTrail}(t)
    pub fn backtrack_rule(&self, t: usize, c: usize) -> Bool {
        Bool::and(&[
            &self.conflict_in_clause(t, c),
            &self.decision_on_trail(t),
            &self.backtrack_effect(t),
        ])
    }
}
