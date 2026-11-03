use z3::ast::Bool;

use crate::dpll::Model;
use crate::generate::compl;

impl Model {
    /// \bigwedge_{p} \left(\lnot M_{t,p,\ell} \land \lnot M_{t,p,\ell^c}\right)
    fn decide_unassigned(&self, t: usize, l: usize) -> Bool {
        Bool::and(&[
            &self.on_trail(t, l).not(),
            &self.on_trail(t, compl(l)).not(),
        ])
    }

    fn decide_in_order(&self, t: usize, l: usize) -> Bool {
        // Small function which check whether variable ordering is being followed
        // Added for frontend, by default on but can be turned off
        if !self.ordered_decisions {
            return Bool::from_bool(true);
        }
        let per_earlier: Vec<Bool> = (0..l / 2)
            .map(|v| Bool::or(&[&self.on_trail(t, 2 * v), &self.on_trail(t, 2 * v + 1)]))
            .collect();
        Bool::and(&per_earlier)
    }

    /// \bigwedge_c \lnot\texttt{UnitLitPresent}(t,c)
    fn no_unit_lit_present(&self, t: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| self.unit_lit_present(t, c).not())
            .collect();
        Bool::and(&per_clause)
    }

    /// \texttt{DecideEffect}(t,\ell) \equiv \bigwedge_{p}\Bigg(\texttt{Next}(t,p) \implies \bigg( M_{t+1,p,\ell} \land \bigwedge_{k\neq\ell}\lnot M_{t+1,p,k}
    ///     \land \bigwedge_{p'<p}\bigwedge_{k}\left(M_{t,p',k} \iff M_{t+1,p',k}\right)
    ///     \land \bigwedge_{p'>p}\bigwedge_{k}\lnot M_{t+1,p',k} \bigg)\Bigg)
    ///     \land D_{t+1,\ell} \land \bigwedge_{k\neq\ell}\left(D_{t,k} \iff D_{t+1,k}\right)
    fn decide_effect(&self, t: usize, l: usize) -> Bool {
        Bool::and(&[
            &self.trail_push(t, l),
            &self.D[t + 1][l],
            &self.other_decisions_kept(t, l),
        ])
    }

    /// \texttt{Decide}(t, \ell) \equiv \texttt{DecideGuard}(t, \ell) \land \texttt{DecideEffect}(t, \ell)
    /// \texttt{DecideGuard}(t, \ell) \equiv \bigwedge_{p} \left(\lnot M_{t,p,\ell} \land \lnot M_{t,p,\ell^c}\right)
    ///     \land \lnot\texttt{SAT}(t)
    ///     \land \bigwedge_c \lnot\texttt{UnitLitPresent}(t,c)
    ///     \land \lnot\texttt{Conflict}(t)
    pub fn decide_rule(&self, t: usize, l: usize) -> Bool {
        Bool::and(&[
            &self.decide_unassigned(t, l),
            &self.sat(t).not(),
            &self.no_unit_lit_present(t),
            &self.conflict(t).not(),
            &self.decide_in_order(t, l),
            &self.decide_effect(t, l),
        ])
    }
}
