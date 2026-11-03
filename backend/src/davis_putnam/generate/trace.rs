use crate::davis_putnam::trace::{Rule, Step, Trace};

pub fn step_var(step: &Step) -> usize {
    match step.rule {
        Rule::Unit | Rule::Pure => step.param / 2,
        Rule::Elim => step.param,
    }
}

pub fn trace_var_amount(trace: &Trace) -> usize {
    match trace.iter().map(step_var).max() {
        Some(a) => a + 1,
        None => 1,
    }
}

pub fn var_order_perm(trace: &Trace, var_amount: usize) -> Vec<usize> {
    let mut perm: Vec<Option<usize>> = vec![None; var_amount];
    let mut next = 0usize;

    for step in trace {
        let v = step_var(step);
        debug_assert!(
            v < var_amount,
            "trace step references var {v} >= var_amount {var_amount}"
        );
        if perm[v].is_none() {
            perm[v] = Some(next);
            next += 1;
        }
    }

    for slot in perm.iter_mut() {
        if slot.is_none() {
            *slot = Some(next);
            next += 1;
        }
    }

    perm.into_iter().map(|o| o.unwrap()).collect()
}
