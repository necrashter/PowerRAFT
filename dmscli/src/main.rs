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
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod commands;

const RESULTS_DIR: &str = "results";

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: commands::Command,
}

#[macro_export]
macro_rules! fatal_error {
    ($ec:expr, $($arg:tt)*) => {{
        let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true)).unwrap();
        write!(&mut stderr, "FATAL ERROR: ").unwrap();
        stderr.reset().unwrap();
        writeln!(&mut stderr, $($arg)*).unwrap();
        std::process::exit($ec);
    }};
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    args.command.run();
}
