use z3::ast::Bool;

use crate::dpll::Model;
use crate::generate::compl;

impl Model {
    /// \texttt{UnitLiteral}(t,c,\ell) \equiv \varphi_{c,\ell} \land \bigwedge_p \lnot M_{t,p,\ell} \land \bigwedge_p \lnot M_{t,p,\ell^c}
    ///     \land \bigwedge_{k\neq\ell}\left(\varphi_{c,k} \implies \bigvee_{p}M_{t,p,k^c}\right)
    fn unit_literal(&self, t: usize, c: usize, l: usize) -> Bool {
        let others_falsified: Vec<Bool> = (0..self.var_amount * 2)
            .filter(|&k| k != l)
            .map(|k| self.Phi[c][k].implies(self.on_trail(t, compl(k))))
            .collect();
        Bool::and(&[
            &self.Phi[c][l],
            &self.on_trail(t, l).not(),
            &self.on_trail(t, compl(l)).not(),
            &Bool::and(&others_falsified),
        ])
    }

    /// \texttt{UnitLitPresent}(t,c) \equiv \bigvee_{\ell} \texttt{UnitLiteral}(t,c,\ell)
    pub(crate) fn unit_lit_present(&self, t: usize, c: usize) -> Bool {
        let per_lit: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| self.unit_literal(t, c, l))
            .collect();
        Bool::or(&per_lit)
    }

    /// \texttt{UnitEffect}(t,\ell) \equiv \bigwedge_{p}\Bigg(\texttt{Next}(t,p) \implies \bigg( M_{t+1,p,\ell} \land \bigwedge_{k\neq\ell}\lnot M_{t+1,p,k}
    ///     \land \bigwedge_{p'<p}\bigwedge_{k}\left(M_{t,p',k} \iff M_{t+1,p',k}\right)
    ///     \land \bigwedge_{p'>p}\bigwedge_{k}\lnot M_{t+1,p',k} \bigg)\Bigg)
    ///     \land \lnot D_{t+1,\ell} \land \bigwedge_{k\neq\ell}\left(D_{t,k} \iff D_{t+1,k}\right)
    fn unit_effect(&self, t: usize, l: usize) -> Bool {
        Bool::and(&[
            &self.trail_push(t, l),
            &self.D[t + 1][l].not(),
            &self.other_decisions_kept(t, l),
        ])
    }

    /// \texttt{UnitProp}(t, c) \equiv \texttt{UnitGuard}(t, c) \land \bigwedge_{\ell}\left(\texttt{UnitLiteral}(t,c,\ell)\implies \texttt{UnitEffect}(t, \ell)\right)
    /// \texttt{UnitGuard}(t,c) \equiv \texttt{UnitLitPresent}(t,c) \land \lnot \texttt{Conflict}(t)
    pub fn unit_prop_rule(&self, t: usize, c: usize) -> Bool {
        let per_lit: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| self.unit_literal(t, c, l).implies(&self.unit_effect(t, l)))
            .collect();
        Bool::and(&[
            &self.unit_lit_present(t, c),
            &self.conflict(t).not(),
            &Bool::and(&per_lit),
        ])
    }
}
