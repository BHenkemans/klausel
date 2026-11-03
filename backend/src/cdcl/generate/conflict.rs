use z3::ast::Bool;

use crate::cdcl::Model;
use crate::generate::compl;

impl Model {
    /// \texttt{LastDec}(t,q) \equiv \bigvee_{\ell} (M_{t,q,\ell} \land D_{t,\ell})
    ///     \land \bigwedge_{q'>q}\bigwedge_\ell\left(M_{t,q',\ell} \implies \lnot D_{t,\ell}\right)
    fn last_dec(&self, t: usize, q: usize) -> Bool {
        let decision_here: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| Bool::and(&[&self.M[t][q][l], &self.D[t][l]]))
            .collect();

        let mut none_above: Vec<Bool> = Vec::new();
        for qp in (q + 1)..self.var_amount {
            for l in 0..self.var_amount * 2 {
                none_above.push(self.M[t][qp][l].implies(self.D[t][l].not()));
            }
        }

        Bool::and(&[&Bool::or(&decision_here), &Bool::and(&none_above)])
    }

    /// \texttt{Start}(t,\ell) \equiv \texttt{OnTrail}(t,\ell) \land\varphi_{t,c^*,\ell^c}
    fn start(&self, t: usize, c: usize, l: usize) -> Bool {
        Bool::and(&[&self.on_trail(t, l), &self.Phi[t][c][compl(l)]])
    }

    /// \texttt{Stop}(t,\ell)\equiv D_{t,\ell} \lor \bigvee_{q}\bigvee_{p<q}(\texttt{LastDec}(t,q)\land M_{t,p,\ell})
    fn stop(&self, t: usize, l: usize, last_dec: &[Bool]) -> Bool {
        let mut before_last_dec: Vec<Bool> = Vec::new();
        for (q, last_dec_q) in last_dec.iter().enumerate() {
            for p in 0..q {
                before_last_dec.push(Bool::and(&[last_dec_q, &self.M[t][p][l]]));
            }
        }
        Bool::or(&[&self.D[t][l], &Bool::or(&before_last_dec)])
    }

    /// \texttt{UP}(t,c,\ell) \equiv \bigvee_p(M_{t,p,\ell} \land \texttt{Ant}_{t,p,c})
    fn up(&self, t: usize, c: usize, l: usize) -> Bool {
        let per_slot: Vec<Bool> = (0..self.var_amount)
            .map(|p| Bool::and(&[&self.M[t][p][l], &self.Ant[t][p][c]]))
            .collect();
        Bool::or(&per_slot)
    }

    /// \texttt{Conn}(t,\ell',\ell) \equiv \bigvee_c (\texttt{UP}(t,c,\ell') \land \varphi_{t,c,\ell^c})
    fn conn(&self, t: usize, up: &[Vec<Bool>], lp: usize, l: usize) -> Bool {
        let per_clause: Vec<Bool> = (0..self.clause_amount)
            .map(|c| Bool::and(&[&up[c][lp], &self.Phi[t][c][compl(l)]]))
            .collect();
        Bool::or(&per_clause)
    }

    /// \texttt{Involvement}(t) \equiv \bigwedge_\ell \Big(\texttt{Inv}_{t,\ell} \iff \texttt{Start}(t,\ell)
    ///     \lor \bigvee_{\ell'\neq \ell}(\texttt{Inv}_{t,\ell'} \land \texttt{Conn}(t,\ell',\ell)\land \lnot\texttt{Stop}(t,\ell'))\Big)
    fn involvement(&self, t: usize, c_star: usize, inv: &[Bool], stop: &[Bool]) -> Bool {
        let up: Vec<Vec<Bool>> = (0..self.clause_amount)
            .map(|c| (0..self.var_amount * 2).map(|l| self.up(t, c, l)).collect())
            .collect();

        let per_lit: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| {
                let via: Vec<Bool> = (0..self.var_amount * 2)
                    .filter(|&lp| lp != l)
                    .map(|lp| Bool::and(&[&inv[lp], &self.conn(t, &up, lp, l), &stop[lp].not()]))
                    .collect();
                inv[l].iff(Bool::or(&[&self.start(t, c_star, l), &Bool::or(&via)]))
            })
            .collect();
        Bool::and(&per_lit)
    }

    /// \texttt{LearnEffect}(t) \equiv \texttt{Act}_{t+1,c_{new}}
    ///     \land \bigwedge_{k}\left(\varphi_{t+1,c_{new}, k} \iff \texttt{Learned}(t,k^c)\right)
    ///     \land \bigwedge_{c\neq c_{new}}\left(\texttt{Act}_{t,c} \iff \texttt{Act}_{t+1,c}\right)
    ///     \land \bigwedge_{c\neq c_{new}}\bigwedge_{k}\left(\varphi_{t,c,k} \iff \varphi_{t+1,c,k}\right)
    /// \texttt{Learned}(t,\ell) \equiv \texttt{Inv}_{t,\ell} \land \texttt{Stop}(t,\ell)
    fn learn_effect(&self, t: usize, c_new: usize, learned: &[Bool]) -> Bool {
        let content: Vec<Bool> = (0..self.var_amount * 2)
            .map(|k| self.Phi[t + 1][c_new][k].iff(&learned[compl(k)]))
            .collect();
        let others_kept: Vec<Bool> = (0..self.clause_amount)
            .filter(|&c| c != c_new)
            .map(|c| self.clause_kept(t, c))
            .collect();
        Bool::and(&[
            &self.Act[t + 1][c_new],
            &Bool::and(&content),
            &Bool::and(&others_kept),
        ])
    }

    /// \texttt{BackjumpLit}(t,\ell)\equiv \bigvee_{\ell'}\bigvee_{j}\bigvee_{i < j}\Bigl( \texttt{Learned}(t,\ell') \land D_{t,\ell}
    ///     \land M_{t,i,\ell'}\land M_{t,j,\ell}
    ///     \land \bigwedge_{i < i' < j} \bigwedge_{k} \left(M_{t,i',k}\implies \lnot D_{t,k}\right)
    ///     \land \bigwedge_{j \leq j'}\Bigl(\texttt{LastDec}(t,j') \lor \bigwedge_{k} \left(M_{t,j',k} \implies \lnot \texttt{Learned}(t,k)\right)\Bigr)\Bigr)
    ///     \lor \Bigl(\bigwedge_{i}\bigwedge_k \left(\left(M_{t,i,k} \land \texttt{Learned}(t,k)\right) \implies \texttt{LastDec}(t,i)\right)
    ///     \land \bigvee_{i} \Bigl(M_{t,i,\ell} \land D_{t,\ell}
    ///     \land \bigwedge_{i'<i}\bigwedge_{k}\left(M_{t,i',k}\implies \lnot D_{t,k}\right)\Bigr)\Bigr)
    /// \texttt{BackjumpSlot}(t,p) \equiv \bigvee_{\ell} \left(M_{t,p,\ell} \land \texttt{BackjumpLit}(t,\ell)\right)
    fn backjump_slots(&self, t: usize, learned: &[Bool], last_dec: &[Bool]) -> Vec<Bool> {
        let learned_at: Vec<Bool> = (0..self.var_amount)
            .map(|i| {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|k| Bool::and(&[&self.M[t][i][k], &learned[k]]))
                    .collect();
                Bool::or(&per_lit)
            })
            .collect();

        let no_dec_at: Vec<Bool> = (0..self.var_amount)
            .map(|i| {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|k| self.M[t][i][k].implies(self.D[t][k].not()))
                    .collect();
                Bool::and(&per_lit)
            })
            .collect();

        let only_final_learned_from: Vec<Bool> = (0..self.var_amount)
            .map(|j| {
                let per_slot: Vec<Bool> = (j..self.var_amount)
                    .map(|jp| Bool::or(&[&last_dec[jp], &learned_at[jp].not()]))
                    .collect();
                Bool::and(&per_slot)
            })
            .collect();

        let only_final_learned: Vec<Bool> = (0..self.var_amount)
            .map(|i| learned_at[i].implies(&last_dec[i]))
            .collect();
        let only_final_learned = Bool::and(&only_final_learned);

        let backjump_lit: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| {
                let mut after_learned: Vec<Bool> = Vec::new();
                for j in 0..self.var_amount {
                    for i in 0..j {
                        after_learned.push(Bool::and(&[
                            &learned_at[i],
                            &self.D[t][l],
                            &self.M[t][j][l],
                            &Bool::and(&no_dec_at[(i + 1)..j]),
                            &only_final_learned_from[j],
                        ]));
                    }
                }

                let first_decision: Vec<Bool> = (0..self.var_amount)
                    .map(|i| {
                        Bool::and(&[&self.M[t][i][l], &self.D[t][l], &Bool::and(&no_dec_at[..i])])
                    })
                    .collect();

                Bool::or(&[
                    &Bool::or(&after_learned),
                    &Bool::and(&[&only_final_learned, &Bool::or(&first_decision)]),
                ])
            })
            .collect();

        (0..self.var_amount)
            .map(|p| {
                let per_lit: Vec<Bool> = (0..self.var_amount * 2)
                    .map(|l| Bool::and(&[&self.M[t][p][l], &backjump_lit[l]]))
                    .collect();
                Bool::or(&per_lit)
            })
            .collect()
    }

    /// \bigwedge_{p \geq s}\bigwedge_{\ell}\lnot M_{t+1,p,\ell}
    fn backjump_trail_cleared(&self, t: usize, s: usize) -> Bool {
        let mut from_s: Vec<Bool> = Vec::new();
        for p in s..self.var_amount {
            for l in 0..self.var_amount * 2 {
                from_s.push(self.M[t + 1][p][l].not());
            }
        }
        Bool::and(&from_s)
    }

    /// \bigwedge_{p \geq s}\bigwedge_{c}\lnot \texttt{Ant}_{t+1,p,c}
    fn backjump_antecedents_cleared(&self, t: usize, s: usize) -> Bool {
        let mut from_s: Vec<Bool> = Vec::new();
        for p in s..self.var_amount {
            for c in 0..self.clause_amount {
                from_s.push(self.Ant[t + 1][p][c].not());
            }
        }
        Bool::and(&from_s)
    }

    /// \bigwedge_{p < s}\bigwedge_{c}\left(\texttt{Ant}_{t,p,c} \iff \texttt{Ant}_{t+1,p,c}\right)
    fn backjump_antecedents_kept(&self, t: usize, s: usize) -> Bool {
        let before_s: Vec<Bool> = (0..s).map(|p| self.antecedents_kept(t, p)).collect();
        Bool::and(&before_s)
    }

    /// \bigwedge_{\ell}\Bigl(D_{t+1,\ell} \iff \bigl(D_{t,\ell} \land \bigvee_{p < s} M_{t,p,\ell}\bigr)\Bigr)
    fn backjump_decisions_kept(&self, t: usize, s: usize) -> Bool {
        let decisions_kept: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| {
                let below: Vec<&Bool> = (0..s).map(|p| &self.M[t][p][l]).collect();
                self.D[t + 1][l].iff(Bool::and(&[&self.D[t][l], &Bool::or(&below)]))
            })
            .collect();
        Bool::and(&decisions_kept)
    }

    /// \texttt{BackjumpEffect}(t) \equiv \bigwedge_{s}\Bigl(\texttt{BackjumpSlot}(t,s) \implies \bigwedge_{p \geq s}\bigwedge_{\ell}\lnot M_{t+1,p,\ell}
    ///     \land \bigwedge_{p< s}\bigwedge_{\ell}\left(M_{t,p,\ell} \iff M_{t+1,p,\ell}\right)
    ///     \land \bigwedge_{p \geq s}\bigwedge_{c}\lnot \texttt{Ant}_{t+1,p,c}
    ///     \land \bigwedge_{p < s}\bigwedge_{c}\left(\texttt{Ant}_{t,p,c} \iff \texttt{Ant}_{t+1,p,c}\right)
    ///     \land \bigwedge_{\ell}\Bigl(D_{t+1,\ell} \iff \bigl(D_{t,\ell} \land \bigvee_{p < s} M_{t,p,\ell}\bigr)\Bigr)\Bigr)
    fn backjump_effect(&self, t: usize, backjump_slot: &[Bool]) -> Bool {
        let per_s_slot: Vec<Bool> = backjump_slot
            .iter()
            .enumerate()
            .map(|(s, backjump_here)| {
                let consequent = Bool::and(&[
                    &self.backjump_trail_cleared(t, s),
                    &self.trail_kept_before(t, s),
                    &self.backjump_antecedents_cleared(t, s),
                    &self.backjump_antecedents_kept(t, s),
                    &self.backjump_decisions_kept(t, s),
                ]);
                backjump_here.implies(&consequent)
            })
            .collect();
        Bool::and(&per_s_slot)
    }

    /// \texttt{Conflict}(t,c^*) \equiv \texttt{ConflictGuard}(t,c^*) \land \texttt{Involvement}(t) \land \texttt{LearnEffect}(t) \land \texttt{BackjumpEffect}(t)
    /// \texttt{ConflictGuard}(t,c^*) \equiv \texttt{ConflictInClause}(t,c^*) \land \texttt{DecisionOnTrail}(t)
    pub fn conflict_rule(&self, t: usize, c_star: usize, c_new: usize) -> Bool {
        let inv: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| Bool::new_const(format!("Inv_{t}_{l}")))
            .collect();
        let last_dec: Vec<Bool> = (0..self.var_amount).map(|q| self.last_dec(t, q)).collect();
        let stop: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| self.stop(t, l, &last_dec))
            .collect();
        let learned: Vec<Bool> = (0..self.var_amount * 2)
            .map(|l| Bool::and(&[&inv[l], &stop[l]]))
            .collect();
        let backjump_slot = self.backjump_slots(t, &learned, &last_dec);

        Bool::and(&[
            &self.conflict_in_clause(t, c_star),
            &self.decision_on_trail(t),
            &self.involvement(t, c_star, &inv, &stop),
            &self.learn_effect(t, c_new, &learned),
            &self.backjump_effect(t, &backjump_slot),
        ])
    }
}
