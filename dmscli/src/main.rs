use std::path::Path;
use std::{io::Write, path::PathBuf};

use dmslib::io::{BenchmarkResult, OptimizationBenchmarkResult, OptimizationInfo, TeamProblem};
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

fn read_and_parse_team_problem<P: AsRef<Path>>(path: P) -> dmslib::teams::Problem {
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
    problem
}

fn benchmark<F: FnOnce()>(
    problem: &dmslib::teams::Problem,
    action: &str,
    transition: &str,
    loading_indicator: F,
) -> BenchmarkResult {
    eprintln!("Action:           {}", action);
    eprintln!("Transition:       {}", transition);

    loading_indicator();

    let result = match dmslib::teams::benchmark_custom(
        &problem.graph,
        problem.initial_teams.clone(),
        action,
        transition,
    ) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Cannot solve team problem: {}", err);
            std::process::exit(1);
        }
    };

    eprintln!("Number of states: {}", result.states);
    eprintln!("Generation time:  {}", result.generation_time);
    eprintln!("Total time:       {}", result.total_time);
    eprintln!("Min Value:        {}", result.value);

    result
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
            eprintln!();

            let problem = read_and_parse_team_problem(path);
            let _ = benchmark(&problem, &action, &transition, || {
                eprint!("Solving...\r");
                let _ = std::io::stderr().flush();
            });
        }

        Command::Benchmark { path, json } => {
            eprintln!("Benchmarking team problem: {}", path.to_str().unwrap());

            let problem = read_and_parse_team_problem(path);

            let total_optimizations = iter_optimizations().count();

            let results: Vec<OptimizationBenchmarkResult> = iter_optimizations()
                .enumerate()
                .map(|(i, (action_set, action_applier))| {
                    eprintln!();
                    let result = benchmark(&problem, &action_set, &action_applier, || {
                        eprint!("Solving {}/{}...\r", i + 1, total_optimizations);
                        let _ = std::io::stderr().flush();
                    });

                    OptimizationBenchmarkResult {
                        result,
                        optimizations: OptimizationInfo {
                            actions: action_set.to_string(),
                            transitions: action_applier.to_string(),
                        },
                    }
                })
                .collect();

            if json {
                let serialized = match serde_json::to_string_pretty(&results) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error while serializing results: {}", e);
                        std::process::exit(1);
                    }
                };
                println!("{}", serialized);
            }

            eprintln!();
            eprintln!("Done!");
        }
    }
}
