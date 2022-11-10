use std::path::Path;
use std::{io::Write, path::PathBuf};

use dmslib::io::{BenchmarkResult, OptimizationBenchmarkResult, OptimizationInfo, TeamProblem};
use dmslib::teams::iter_optimizations;

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
    /// Run an experiment with custom optimizations.
    BenchmarkSingle {
        /// Path to the experiment JSON file.
        path: PathBuf,
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
    /// Run an experiment for all optimization combinations.
    Benchmark {
        /// Path to the experiment JSON file.
        path: PathBuf,
        /// Print the results as JSON (Hint: redirect stdout)
        #[arg(short, long, default_value_t = false)]
        json: bool,
    },
    /// Print the travel time matrix for an experiment.
    Tt {
        /// Path to the experiment JSON file.
        path: PathBuf,
    },
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
    action: &str,
    transition: &str,
) -> std::io::Result<()> {
    let mut bold = ColorSpec::new();
    bold.set_bold(true);

    out.set_color(&bold)?;
    write!(out, "Action:           ")?;
    out.reset()?;
    writeln!(out, "{}", action)?;
    out.set_color(&bold)?;
    write!(out, "Transition:       ")?;
    out.reset()?;
    writeln!(out, "{}", transition)?;
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
    Ok(())
}

fn benchmark(
    problem: &dmslib::teams::Problem,
    action: &str,
    transition: &str,
) -> OptimizationBenchmarkResult {
    let result = dmslib::teams::benchmark_custom(
        &problem.graph,
        problem.initial_teams.clone(),
        action,
        transition,
    );
    let result = match result {
        Ok(s) => s,
        Err(err) => fatal_error!(1, "Cannot solve team problem: {}", err),
    };

    OptimizationBenchmarkResult {
        result,
        optimizations: OptimizationInfo {
            actions: action.to_string(),
            transitions: transition.to_string(),
        },
    }
}

fn main() {
    let args = Args::parse();

    let mut stderr = StandardStream::stderr(ColorChoice::Auto);

    match args.command {
        Command::BenchmarkSingle {
            path,
            action,
            transition,
            json,
        } => {
            let (name, problem) = read_and_parse_team_problem(path);

            stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
            write!(&mut stderr, "Experiment Name:  ").unwrap();
            stderr.reset().unwrap();
            writeln!(&mut stderr, "{}", name).unwrap();

            print_optimizations(&mut stderr, &action, &transition).unwrap();

            stderr
                .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
                .unwrap();
            write!(&mut stderr, "Solving...\r").unwrap();
            stderr.reset().unwrap();
            stderr.flush().unwrap();

            let result = benchmark(&problem, &action, &transition);

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
            write!(&mut stderr, "Experiment Name:  ").unwrap();
            stderr.reset().unwrap();
            writeln!(&mut stderr, "{}", name).unwrap();

            let total_optimizations = iter_optimizations().count();

            let results: Vec<OptimizationBenchmarkResult> = iter_optimizations()
                .enumerate()
                .map(|(i, (action_set, action_applier))| {
                    writeln!(&mut stderr).unwrap();
                    print_optimizations(&mut stderr, action_set, action_applier).unwrap();

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

                    let result = benchmark(&problem, action_set, action_applier);

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
            let (name, problem) = read_and_parse_team_problem(path);
            let travel_times = problem.graph.travel_times;

            stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
            write!(&mut stderr, "Experiment Name:  ").unwrap();
            stderr.reset().unwrap();
            writeln!(&mut stderr, "{}", name).unwrap();

            stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
            write!(&mut stderr, "Average Time: ").unwrap();
            stderr.reset().unwrap();
            let avg: f64 = travel_times.sum() as f64
                / (travel_times.shape()[0] * travel_times.shape()[1]) as f64;
            writeln!(&mut stderr, "{}", avg).unwrap();

            stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
            write!(&mut stderr, "Maximum Time: ").unwrap();
            stderr.reset().unwrap();
            writeln!(&mut stderr, "{}", travel_times.iter().max().unwrap()).unwrap();

            println!("{}", &travel_times);
        }
    }
}
