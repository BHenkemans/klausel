use std::time::{Duration, Instant};

use z3::{Params, SatResult, Solver, ast::Bool};

pub struct SolveOutcome {
    pub sat: bool,
    pub unknown: bool,
    pub build: Duration,
    pub solve: Duration,
    pub model: Option<z3::Model>,
}

pub fn compl(l: usize) -> usize {
    l ^ 1
}

pub(crate) fn solve(
    timeout: Option<Duration>,
    constraints: impl FnOnce() -> Vec<Bool>,
) -> SolveOutcome {
    let build_start = Instant::now();
    let solver = Solver::new();

    if let Some(t) = timeout {
        let mut params = Params::new();
        params.set_u32("timeout", t.as_millis() as u32);
        solver.set_params(&params);
    }

    for constraint in constraints() {
        solver.assert(&constraint);
    }

    let build = build_start.elapsed();
    let solve_start = Instant::now();
    let result = solver.check();
    let solve = solve_start.elapsed();

    let sat = matches!(result, SatResult::Sat);
    SolveOutcome {
        sat,
        unknown: matches!(result, SatResult::Unknown),
        build,
        solve,
        model: if sat { solver.get_model() } else { None },
    }
}

pub(crate) fn holds(model: &z3::Model, b: &Bool) -> bool {
    model.eval(b, true).and_then(|b| b.as_bool()) == Some(true)
}

pub(crate) fn lit_name(l: usize) -> String {
    format!("{}x{}", if l.is_multiple_of(2) { "" } else { "!" }, l / 2)
}

pub(crate) fn format_clause(c: usize, lits: &[String]) -> String {
    let body = if lits.is_empty() {
        "[]".to_string()
    } else {
        lits.join(" v ")
    };
    format!("  c{c}: {body}\n")
}
