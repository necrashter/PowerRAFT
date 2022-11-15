use std::path::Path;
use std::{io::Write, path::PathBuf};

use dmslib::io::fs::read_problems_from_file;
use dmslib::io::{
    read_experiment_from_file, BenchmarkResult, ExperimentTask, OptimizationBenchmarkResult,
    OptimizationInfo, TeamProblem,
};
use dmslib::teams::all_optimizations;

use clap::{Parser, Subcommand};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run an experiment.
    Run {
        /// Path to the experiment JSON file.
        path: PathBuf,
        /// Print the results as JSON (Hint: redirect stdout)
        #[arg(short, long, default_value_t = false)]
        json: bool,
    },
    /// Solve a problem with custom optimizations.
    Solve {
        /// Path to the JSON file containing the problem.
        path: PathBuf,
        /// State indexer class.
        #[arg(short, long, default_value = "NaiveStateIndexer")]
        indexer: String,
        /// Action set class.
        #[arg(short, long, default_value = "NaiveActions")]
        action: String,
        /// Action applier class.
        #[arg(short, long, default_value = "NaiveActionApplier")]
        transition: String,
        /// Print the results as JSON (Hint: redirect stdout)
        #[arg(short, long, default_value_t = false)]
        json: bool,
    },
    /// Solve a problem for all optimization combinations.
    Benchmark {
        /// Path to the JSON file containing the problem.
        path: PathBuf,
        /// Print the results as JSON (Hint: redirect stdout)
        #[arg(short, long, default_value_t = false)]
        json: bool,
    },
    /// Print the travel time matrix for a field-teams problem.
    Tt {
        /// Path to the JSON file containing the problem.
        path: PathBuf,
    },
    /// Print the direct distance matrix for a field-teams problem.
    D {
        /// Path to the JSON file containing the problem.
        path: PathBuf,
        /// Number of decimal places in output.
        #[arg(short, long, default_value_t = 3)]
        precision: usize,
    },
    /// Print the list of all possible optimizations.
    ListAllOpt,
}

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

fn read_and_parse_team_problem<P: AsRef<Path>>(path: P) -> (String, dmslib::teams::Problem) {
    let mut problem = match TeamProblem::read_from_file(path) {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Cannot read team problem: {}", err),
    };
    let name = problem.name.take().unwrap_or_else(|| "-".to_string());
    let problem = match problem.prepare() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };
    (name, problem)
}

fn print_optimizations(
    out: &mut StandardStream,
    optimization: &OptimizationInfo,
) -> std::io::Result<()> {
    let mut bold = ColorSpec::new();
    bold.set_bold(true);

    out.set_color(&bold)?;
    write!(out, "Indexer:          ")?;
    out.reset()?;
    writeln!(out, "{}", optimization.indexer)?;
    out.set_color(&bold)?;
    write!(out, "Action:           ")?;
    out.reset()?;
    writeln!(out, "{}", optimization.actions)?;
    out.set_color(&bold)?;
    write!(out, "Transition:       ")?;
    out.reset()?;
    writeln!(out, "{}", optimization.transitions)?;
    Ok(())
}

fn print_benchmark_result(
    out: &mut StandardStream,
    result: &BenchmarkResult,
) -> std::io::Result<()> {
    let mut bold = ColorSpec::new();
    bold.set_bold(true);

    out.set_color(&bold)?;
    write!(out, "Number of states: ")?;
    out.reset()?;
    writeln!(out, "{}", result.states)?;
    out.set_color(&bold)?;
    write!(out, "Generation time:  ")?;
    out.reset()?;
    writeln!(out, "{}", result.generation_time)?;
    out.set_color(&bold)?;
    write!(out, "Total time:       ")?;
    out.reset()?;
    writeln!(out, "{}", result.total_time)?;
    out.set_color(&bold)?;
    write!(out, "Min Value:        ")?;
    out.reset()?;
    writeln!(out, "{}", result.value)?;
    out.set_color(&bold)?;
    write!(out, "Horizon:          ")?;
    out.reset()?;
    writeln!(out, "{}", result.horizon)?;
    Ok(())
}

fn print_distances(out: &mut StandardStream, mut problem: TeamProblem, precision: usize) {
    let name = problem.name.take().unwrap_or_else(|| "-".to_string());
    let distances = match problem.get_distances() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Problem Name:     ").unwrap();
    out.reset().unwrap();
    writeln!(out, "{}", name).unwrap();

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Average Distance: ").unwrap();
    out.reset().unwrap();
    let avg: f64 = dmslib::utils::distance_matrix_average(&distances);
    writeln!(out, "{}", avg).unwrap();

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Maximum Distance: ").unwrap();
    out.reset().unwrap();
    writeln!(
        out,
        "{}",
        distances
            .iter()
            .max_by(|a, b| {
                a.partial_cmp(b)
                    .expect("Distance values must be comparable (not NaN)")
            })
            .unwrap()
    )
    .unwrap();

    let problem = match problem.prepare() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };
    let neighbor_dists = dmslib::utils::neighbor_distances(&distances, &problem.graph.branches);

    if !neighbor_dists.is_empty() {
        out.set_color(ColorSpec::new().set_bold(true)).unwrap();
        writeln!(out, "Neighbor Distances:").unwrap();
        out.reset().unwrap();

        let min = neighbor_dists
            .iter()
            .min_by(|x, y| x.partial_cmp(y).expect("Distances cannot be compared"))
            .unwrap();
        out.set_color(ColorSpec::new().set_bold(true)).unwrap();
        write!(out, "         Minimum: ").unwrap();
        out.reset().unwrap();
        writeln!(out, "{}", min).unwrap();

        let avg: f64 = neighbor_dists.iter().sum::<f64>() / (neighbor_dists.len() as f64);
        out.set_color(ColorSpec::new().set_bold(true)).unwrap();
        write!(out, "         Average: ").unwrap();
        out.reset().unwrap();
        writeln!(out, "{}", avg).unwrap();

        let max = neighbor_dists
            .iter()
            .max_by(|x, y| x.partial_cmp(y).expect("Distances cannot be compared"))
            .unwrap();
        out.set_color(ColorSpec::new().set_bold(true)).unwrap();
        write!(out, "         Maximum: ").unwrap();
        out.reset().unwrap();
        writeln!(out, "{}", max).unwrap();
    }

    println!("{:.1$}", &distances, precision);
}

fn print_travel_times(out: &mut StandardStream, mut problem: TeamProblem) {
    let name = problem.name.take().unwrap_or_else(|| "-".to_string());
    let problem = match problem.prepare() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };
    let travel_times = problem.graph.travel_times;

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Problem Name: ").unwrap();
    out.reset().unwrap();
    writeln!(out, "{}", name).unwrap();

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Average Time: ").unwrap();
    out.reset().unwrap();
    let avg: f64 = dmslib::utils::distance_matrix_average(&travel_times);
    writeln!(out, "{}", avg).unwrap();

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Maximum Time: ").unwrap();
    out.reset().unwrap();
    writeln!(out, "{}", travel_times.iter().max().unwrap()).unwrap();

    println!("{}", &travel_times);
}

fn benchmark(
    problem: &dmslib::teams::Problem,
    optimization: &OptimizationInfo,
) -> OptimizationBenchmarkResult {
    let result = dmslib::teams::benchmark_custom(
        &problem.graph,
        problem.initial_teams.clone(),
        problem.horizon,
        &optimization.indexer,
        &optimization.actions,
        &optimization.transitions,
    );
    let result = match result {
        Ok(s) => s,
        Err(err) => fatal_error!(1, "Cannot solve team problem: {}", err),
    };

    OptimizationBenchmarkResult {
        result,
        optimizations: optimization.clone(),
    }
}

fn main() {
    let args = Args::parse();

    let mut stderr = StandardStream::stderr(ColorChoice::Auto);

    match args.command {
        Command::Run { path, json } => {
            let experiment = match read_experiment_from_file(path) {
                Ok(s) => s,
                Err(err) => fatal_error!(1, "Cannot parse experiment: {}", err),
            };

            stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
            write!(&mut stderr, "Experiment Name:  ").unwrap();
            stderr.reset().unwrap();
            writeln!(
                &mut stderr,
                "{}\n",
                experiment.name.as_ref().map(String::as_ref).unwrap_or("-")
            )
            .unwrap();

            let mut current: usize = 1;
            let total_benchmarks: usize = experiment
                .tasks
                .iter()
                .map(|task| task.problems.len() * task.optimizations.len())
                .sum();

            let mut results: Vec<OptimizationBenchmarkResult> = Vec::new();

            for task in experiment.tasks.into_iter() {
                let ExperimentTask {
                    problems,
                    optimizations,
                } = task;
                for problem in problems {
                    stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
                    write!(&mut stderr, "Problem Name:     ").unwrap();
                    stderr.reset().unwrap();
                    writeln!(
                        &mut stderr,
                        "{}",
                        problem.name.as_ref().map(String::as_ref).unwrap_or("-")
                    )
                    .unwrap();

                    let problem = match problem.prepare() {
                        Ok(x) => x,
                        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
                    };

                    for optimization in &optimizations {
                        writeln!(&mut stderr).unwrap();
                        print_optimizations(&mut stderr, optimization).unwrap();

                        stderr
                            .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
                            .unwrap();
                        write!(&mut stderr, "Solving {}/{}...\r", current, total_benchmarks)
                            .unwrap();
                        stderr.reset().unwrap();
                        stderr.flush().unwrap();

                        let result = benchmark(&problem, optimization);

                        print_benchmark_result(&mut stderr, &result.result).unwrap();

                        results.push(result);

                        current += 1;
                    }
                }
            }

            if json {
                let serialized = match serde_json::to_string_pretty(&results) {
                    Ok(s) => s,
                    Err(e) => fatal_error!(1, "Error while serializing results: {}", e),
                };
                println!("{}", serialized);
            }

            stderr
                .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
                .unwrap();
            writeln!(&mut stderr, "\nDone!").unwrap();
            stderr.reset().unwrap();
        }

        Command::Solve {
            path,
            indexer,
            action,
            transition,
            json,
        } => {
            let (name, problem) = read_and_parse_team_problem(path);

            stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
            write!(&mut stderr, "Problem Name:     ").unwrap();
            stderr.reset().unwrap();
            writeln!(&mut stderr, "{}", name).unwrap();

            let optimizations = OptimizationInfo {
                indexer,
                actions: action,
                transitions: transition,
            };

            print_optimizations(&mut stderr, &optimizations).unwrap();

            stderr
                .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
                .unwrap();
            write!(&mut stderr, "Solving...\r").unwrap();
            stderr.reset().unwrap();
            stderr.flush().unwrap();

            let result = benchmark(&problem, &optimizations);

            print_benchmark_result(&mut stderr, &result.result).unwrap();

            if json {
                let serialized = match serde_json::to_string_pretty(&result) {
                    Ok(s) => s,
                    Err(e) => fatal_error!(1, "Error while serializing results: {}", e),
                };
                println!("{}", serialized);
            }
        }

        Command::Benchmark { path, json } => {
            let (name, problem) = read_and_parse_team_problem(path);

            stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
            write!(&mut stderr, "Problem Name:     ").unwrap();
            stderr.reset().unwrap();
            writeln!(&mut stderr, "{}", name).unwrap();

            let opt_list = all_optimizations();
            let total_optimizations = opt_list.len();

            let results: Vec<OptimizationBenchmarkResult> = opt_list
                .into_iter()
                .enumerate()
                .map(|(i, optimizations)| {
                    writeln!(&mut stderr).unwrap();
                    print_optimizations(&mut stderr, &optimizations).unwrap();

                    stderr
                        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
                        .unwrap();
                    write!(
                        &mut stderr,
                        "Solving {}/{}...\r",
                        i + 1,
                        total_optimizations
                    )
                    .unwrap();
                    stderr.reset().unwrap();
                    stderr.flush().unwrap();

                    let result = benchmark(&problem, &optimizations);

                    print_benchmark_result(&mut stderr, &result.result).unwrap();

                    result
                })
                .collect();

            if json {
                let serialized = match serde_json::to_string_pretty(&results) {
                    Ok(s) => s,
                    Err(e) => fatal_error!(1, "Error while serializing results: {}", e),
                };
                println!("{}", serialized);
            }

            stderr
                .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
                .unwrap();
            writeln!(&mut stderr, "\nDone!").unwrap();
            stderr.reset().unwrap();
        }

        Command::Tt { path } => {
            let problems = match read_problems_from_file(path) {
                Ok(x) => x,
                Err(err) => fatal_error!(1, "Cannot read team problem(s): {}", err),
            };
            for problem in problems {
                print_travel_times(&mut stderr, problem);
            }
        }

        Command::D { path, precision } => {
            let problems = match read_problems_from_file(path) {
                Ok(x) => x,
                Err(err) => fatal_error!(1, "Cannot read team problem(s): {}", err),
            };
            for problem in problems {
                print_distances(&mut stderr, problem, precision);
            }
        }

        Command::ListAllOpt => {
            let result = dmslib::teams::all_optimizations();
            let serialized = match serde_json::to_string_pretty(&result) {
                Ok(s) => s,
                Err(e) => fatal_error!(1, "Error while serializing results: {}", e),
            };
            println!("{}", serialized);
        }
    }
}
