use std::path::Path;
use std::sync::atomic::{self, AtomicUsize};
use std::time::Duration;
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

pub static RUNNING_STATE: AtomicUsize = AtomicUsize::new(0);

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    ctrlc::set_handler(|| {
        let prev = RUNNING_STATE.fetch_add(1, atomic::Ordering::SeqCst);
        if prev == 0 {
            println!("\r{}", "Ctrl+C received, exiting...".yellow());
            std::process::exit(130);
        } else if prev & 1 == 1 {
            println!(
                "\r{}",
                "Second Ctrl+C received, exiting immediately...".yellow()
            );
            std::process::exit(130);
        } else if prev & 2 != 0 {
            println!(
                "\r{}",
                "Ctrl+C received, will exit after the next checkpoint...".yellow()
            );
        }
    })
    .expect("Failed to set Ctrl+C handler");

    let Args { command, seed } = Args::parse();

    if let Some(seed) = seed {
        log::debug!("Setting random seed to {seed}");
        dmslib::RANDOM_SEED.with_borrow_mut(|random_seed| {
            *random_seed = Some(seed);
        });
    }

    command.run();
}

pub fn format_duration(duration: &Duration) -> String {
    let seconds = duration.as_secs();
    let millis = duration.subsec_millis();

    let hours = seconds / 3600;
    let mut remainder = seconds % 3600;
    let minutes = remainder / 60;
    remainder %= 60;
    let seconds = remainder;

    if hours > 0 {
        format!("{hours}:{minutes}:{seconds}.{millis}")
    } else if minutes > 0 {
        format!("{minutes}:{seconds}.{millis}")
    } else {
        format!("{seconds}.{millis}")
    }
}
