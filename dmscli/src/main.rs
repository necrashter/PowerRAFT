use std::path::Path;
use std::{io::Write, path::PathBuf};

use dmslib::io::fs::read_problems_from_file;
use dmslib::io::{
    read_experiment_from_file, BenchmarkResult, ExperimentTask, GenericTeamSolution,
    OptimizationBenchmarkResult, OptimizationInfo, TeamProblem,
};
use dmslib::teams;
use dmslib::SolveFailure;

use clap::Parser;
use colored::*;

mod commands;

const RESULTS_DIR: &str = "results";

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: commands::Command,

    /// Optional random seed used by some components.
    #[arg(long, global = true)]
    seed: Option<u64>,
}

#[macro_export]
macro_rules! fatal_error {
    ($ec:expr, $($arg:tt)*) => {{
        eprint!("{}", "FATAL ERROR: ".red().bold());
        eprintln!($($arg)*);
        std::process::exit($ec);
    }};
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let Args { command, seed } = Args::parse();

    if let Some(seed) = seed {
        log::info!("Setting random seed to {seed}");
        dmslib::RANDOM_SEED.with_borrow_mut(|random_seed| {
            *random_seed = Some(seed);
        });
    }

    command.run();
}
