use std::collections::HashSet;
use std::fmt::Write;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sat {
    Sat,
    Unsat,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Lit(pub usize);

impl Lit {
    pub fn compl(self) -> Lit {
        Lit(self.0 ^ 1)
    }

    pub fn atom(self) -> usize {
        self.0 / 2
    }
}

pub type Clause = Vec<Lit>;
pub type ClauseSet = Vec<Clause>;

pub fn norm_clause(mut c: Clause) -> Option<Clause> {
    c.sort();
    c.dedup();
    let taut = c.windows(2).any(|w| w[1].0 == w[0].0 ^ 1);
    if taut { None } else { Some(c) }
}

pub fn norm_set(s: ClauseSet) -> ClauseSet {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: ClauseSet = Vec::new();
    for c in s {
        let Some(nc) = norm_clause(c) else { continue };
        let k = clause_key(&nc);
        if seen.insert(k) {
            out.push(nc);
        }
    }
    out
}

fn clause_key(c: &Clause) -> String {
    let mut out = String::new();
    for (i, lit) in c.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write!(out, "{}", lit.0).unwrap();
    }
    out
}

pub fn literals(s: &ClauseSet) -> Vec<Lit> {
    let mut lits: Vec<Lit> = s.iter().flatten().copied().collect();
    lits.sort();
    lits.dedup();
    lits
}

pub fn canonical_set(s: ClauseSet) -> ClauseSet {
    let mut s: ClauseSet = s.into_iter().filter_map(norm_clause).collect();
    s.sort();
    s.dedup();
    s
}
