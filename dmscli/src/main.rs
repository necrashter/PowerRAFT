use std::{io::Write, path::PathBuf};

use dmslib::io::TeamProblem;
use dmslib::teams::iter_optimizations;

use clap::{Parser, Subcommand};

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the given experiment with custom optimizations and print benchmark results.
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
    /// Run the given experiment for all optimization combinations and print results.
    Benchmark {
        /// Path to the experiment JSON file.
        path: PathBuf,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::BenchmarkSingle {
            path,
            action,
            transition,
        } => {
            println!("Benchmarking team problem: {}", path.to_str().unwrap());
            println!("Action:           {}", action);
            println!("Transition:       {}", transition);

            print!("Solving...\r");
            let _ = std::io::stdout().flush();

            let problem = match TeamProblem::read_from_file(path) {
                Ok(x) => x,
                Err(err) => {
                    eprintln!("Cannot read team problem: {}", err);
                    return;
                }
            };

            let result = match problem.benchmark_custom(&action, &transition) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("Cannot solve team problem: {}", err);
                    return;
                }
            };

            println!("Number of states: {}", result.states);
            println!("Generation time:  {}", result.generation_time);
            println!("Total time:       {}", result.total_time);
            println!("Min Value:        {}", result.value);
        }

        Command::Benchmark { path } => {
            println!("Benchmarking team problem: {}", path.to_str().unwrap());

            let problem = match TeamProblem::read_from_file(path) {
                Ok(x) => x,
                Err(err) => {
                    eprintln!("Cannot read team problem: {}", err);
                    return;
                }
            };
            let problem = match problem.prepare() {
                Ok(x) => x,
                Err(err) => {
                    eprintln!("Error while parsing team problem: {}", err);
                    return;
                }
            };

            let total_optimizations = iter_optimizations().count();
            for (i, (action_set, action_applier)) in iter_optimizations().enumerate() {
                println!();
                println!("Actions:          {}", action_set);
                println!("Transitions:      {}", action_applier);

                print!("Solving {}/{}...\r", i + 1, total_optimizations);
                let _ = std::io::stdout().flush();

                let result = dmslib::teams::benchmark_custom(
                    &problem.graph,
                    problem.initial_teams.clone(),
                    action_set,
                    action_applier,
                )
                .expect("Invalid optimization class name from iter_optimizations");

                println!("Number of states: {}", result.states);
                println!("Generation time:  {}", result.generation_time);
                println!("Total time:       {}", result.total_time);
                println!("Min Value:        {}", result.value);
            }
        }
    }
}
