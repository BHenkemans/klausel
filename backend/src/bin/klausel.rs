use std::fs;
use std::path::{Path, PathBuf};

use backend::davis_putnam::generate::trace::var_order_perm;
use backend::generate::SolveOutcome;
use backend::trace::{ParseError, ParsedTrace};
use backend::{cdcl, davis_putnam, dpll};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Check a DP, DPLL or CDCL trace against Z3")]
struct Cli {
    #[command(subcommand)]
    algorithm: Algorithm,
}

#[derive(Subcommand)]
enum Algorithm {
    #[command(about = "Davis-Putnam")]
    Dp {
        #[arg(short, long)]
        reorder: bool,
        trace_file: PathBuf,
        #[arg(default_value_t = 9)]
        initial_clauses: usize,
    },
    #[command(about = "Davis-Putnam-Logemann-Loveland")]
    Dpll(TrailArgs),
    #[command(about = "Conflict-driven clause learning")]
    Cdcl(TrailArgs),
}

#[derive(Args)]
struct TrailArgs {
    #[arg(long)]
    trail: bool,
    #[arg(long)]
    unordered: bool,
    trace_file: PathBuf,
    #[arg(long)]
    vars: usize,
    #[arg(long)]
    clauses: usize,
}

fn main() {
    match Cli::parse().algorithm {
        Algorithm::Dp {
            reorder,
            trace_file,
            initial_clauses,
        } => run_dp(reorder, &trace_file, initial_clauses),
        Algorithm::Dpll(args) => run_dpll(&args),
        Algorithm::Cdcl(args) => run_cdcl(&args),
    }
}

fn load<R>(
    path: &Path,
    parse_trace: fn(&str) -> Result<ParsedTrace<R>, ParseError>,
) -> ParsedTrace<R> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));
    parse_trace(&text).unwrap_or_else(|e| panic!("parse error: {e:?}"))
}

fn print_timings(outcome: &SolveOutcome) {
    println!("sat   = {}", outcome.sat);
    println!("build = {:?}", outcome.build);
    println!("solve = {:?}", outcome.solve);
    println!("total = {:?}", outcome.build + outcome.solve);
}

fn run_dp(reorder: bool, path: &Path, initial_clauses: usize) {
    let ParsedTrace { trace, sat } = load(path, davis_putnam::trace::parse_trace);
    let model = davis_putnam::Model::from_trace(&trace, initial_clauses);
    let outcome = model.solve(&trace, sat, None);
    print_timings(&outcome);

    if outcome.sat
        && let Some(z3_model) = &outcome.model
    {
        if reorder {
            let perm = var_order_perm(&trace, model.var_amount());
            model.print_final_model_variable_ordered(z3_model, &perm);
        } else {
            model.print_final_model(z3_model);
        }
    }
}

fn run_dpll(args: &TrailArgs) {
    let ParsedTrace { trace, sat } = load(&args.trace_file, dpll::trace::parse_trace);
    let model = dpll::Model::from_trace(&trace, args.vars, args.clauses)
        .with_ordered_decisions(!args.unordered);
    let outcome = model.solve(&trace, sat, None);
    print_timings(&outcome);

    if let Some(z3_model) = &outcome.model {
        model.print_final_model(z3_model);

        if args.trail {
            println!("trail:");
            for t in 0..model.step_amount() {
                println!("  t{t}: {}", model.format_trail(z3_model, t));
            }
        }
    }
}

fn run_cdcl(args: &TrailArgs) {
    let ParsedTrace { trace, sat } = load(&args.trace_file, cdcl::trace::parse_trace);
    let model = cdcl::Model::from_trace(&trace, args.vars, args.clauses)
        .with_ordered_decisions(!args.unordered);
    let outcome = model.solve(&trace, sat, None);
    print_timings(&outcome);

    if let Some(z3_model) = &outcome.model {
        model.print_final_model(z3_model);

        if args.trail {
            let last = model.step_amount() - 1;
            println!("trail:");
            for t in 0..model.step_amount() {
                println!("  t{t}: {}", model.format_trail(z3_model, t));
            }
            println!("clauses at t{last}:");
            print!("{}", model.format_clauses(z3_model, last));
        }
    }
}
