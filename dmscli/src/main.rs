use std::{io::Write, path::PathBuf};

use dmslib::io::{TeamProblem, OptimizationBenchmarkResult, OptimizationInfo};
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
    /// Run an experiment with custom optimizations.
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
    /// Run an experiment for all optimization combinations.
    Benchmark {
        /// Path to the experiment JSON file.
        path: PathBuf,
        /// Print the results as JSON (Hint: redirect stdout)
        #[arg(short, long, default_value_t = false)]
        json: bool,
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
            eprintln!("Benchmarking team problem: {}", path.to_str().unwrap());
            println!("Action:           {}", action);
            println!("Transition:       {}", transition);

            eprint!("Solving...\r");
            let _ = std::io::stderr().flush();

            let problem = match TeamProblem::read_from_file(path) {
                Ok(x) => x,
                Err(err) => {
                    eprintln!("Cannot read team problem: {}", err);
                    std::process::exit(1);
                }
            };

            let result = match problem.benchmark_custom(&action, &transition) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("Cannot solve team problem: {}", err);
                    std::process::exit(1);
                }
            };

            println!("Number of states: {}", result.states);
            println!("Generation time:  {}", result.generation_time);
            println!("Total time:       {}", result.total_time);
            println!("Min Value:        {}", result.value);
        }

        Command::Benchmark { path, json } => {
            eprintln!("Benchmarking team problem: {}", path.to_str().unwrap());

            let problem = match TeamProblem::read_from_file(path) {
                Ok(x) => x,
                Err(err) => {
                    eprintln!("Cannot read team problem: {}", err);
                    std::process::exit(1);
                }
            };
            let problem = match problem.prepare() {
                Ok(x) => x,
                Err(err) => {
                    eprintln!("Error while parsing team problem: {}", err);
                    std::process::exit(1);
                }
            };

            let total_optimizations = iter_optimizations().count();

            if json {
                let results: Vec<OptimizationBenchmarkResult>  = iter_optimizations()
                    .enumerate()
                    .map(|(i, (action_set, action_applier))| {
                    eprint!("Solving {}/{}...\r", i + 1, total_optimizations);
                    let _ = std::io::stderr().flush();

                        let result = dmslib::teams::benchmark_custom(
                            &problem.graph,
                            problem.initial_teams.clone(),
                            action_set,
                            action_applier,
                            )
                            .expect("Invalid optimization class name from iter_optimizations");

                        OptimizationBenchmarkResult {
                            result,
                            optimizations: OptimizationInfo {
                                actions: action_set.to_string(),
                                transitions: action_applier.to_string(),
                            }
                        }
                    })
                    .collect();
                eprintln!("Done!");
                let serialized = match serde_json::to_string_pretty(&results) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error while serializing results: {}", e);
                        std::process::exit(1);
                    }
                };
                println!("{}", serialized);
            } else {
                for (i, (action_set, action_applier)) in iter_optimizations().enumerate() {
                    println!();
                    println!("Actions:          {}", action_set);
                    println!("Transitions:      {}", action_applier);

                    eprint!("Solving {}/{}...\r", i + 1, total_optimizations);
                    let _ = std::io::stderr().flush();

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
}
