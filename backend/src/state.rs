use std::collections::HashSet;
use std::fmt::Write;

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
