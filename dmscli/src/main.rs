use std::path::PathBuf;

use dmslib::io::TeamProblem;

use clap::{Parser, Subcommand};

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run a given experiment and print benchmark info.
    BenchmarkSingle {
        /// Path to the experiment JSON file.
        path: PathBuf,
        /// Action set class to use.
        #[arg(short, long, default_value = "NaiveActions")]
        action: String,
        /// Action applier class to use.
        #[arg(short, long, default_value = "NaiveActionApplier")]
        transition: String,
    },
}

fn run_benchmark_single(path: &PathBuf, action: &str, transition: &str) {
    println!("Benchmarking team problem: {}", path.to_str().unwrap());
    println!("Action Set: {}", action);
    println!("Action Applier: {}", transition);

    let problem = match TeamProblem::read_from_file(path) {
        Ok(x) => x,
        Err(err) => {
            eprintln!("Cannot read team problem: {}", err);
            return;
        }
    };

    let result = match problem.benchmark_custom(action, transition) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Cannot solve team problem: {}", err);
            return;
        }
    };
    println!("Number of states: {}", result.states);
    println!("Generation time: {}", result.generation_time);
    println!("Total time: {}", result.total_time);
    println!("MinValue: {}", result.value);
}

fn main() {
    let args = Args::parse();

    if let Some(command) = args.command {
        match command {
            Command::BenchmarkSingle {
                path,
                action,
                transition,
            } => {
                run_benchmark_single(&path, &action, &transition);
            }
        }
    }
}
