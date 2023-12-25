use dmslib::{
    io::Experiment,
    teams::{Config, Problem},
};

/// Commands related to running experiments and solving problems.
use super::*;

fn read_and_parse_team_problem<P: AsRef<Path>>(path: P) -> (String, teams::Problem, teams::Config) {
    let mut problem = match TeamProblem::read_from_file(path) {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Cannot read team problem: {}", err),
    };
    let name = problem.name.take().unwrap_or_else(|| "-".to_string());
    let (problem, config) = match problem.prepare() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };
    (name, problem, config)
}

fn solve(
    problem: &teams::Problem,
    config: &teams::Config,
    optimization: &OptimizationInfo,
) -> Result<GenericTeamSolution, SolveFailure> {
    teams::solve_custom(
        &problem.graph,
        problem.initial_teams.clone(),
        config,
        &optimization.indexer,
        &optimization.actions,
        &optimization.transitions,
    )
}

fn get_optimization_result(
    solution: &Result<GenericTeamSolution, SolveFailure>,
    optimization: OptimizationInfo,
) -> OptimizationBenchmarkResult {
    OptimizationBenchmarkResult {
        result: match solution {
            Ok(solution) => Ok(solution.get_benchmark_result()),
            Err(e) => Err(e.clone()),
        },
        optimizations: optimization,
    }
}

fn print_optimizations(optimization: &OptimizationInfo) {
    eprintln!("{:18}{}", "Indexer:".bold(), optimization.indexer);
    eprintln!("{:18}{}", "Actions:".bold(), optimization.actions);
    eprintln!("{:18}{}", "Transitions:".bold(), optimization.transitions);
}

fn print_benchmark_result(result: &Result<BenchmarkResult, SolveFailure>) {
    match result {
        Ok(result) => {
            eprintln!("{:18}{}", "Number of states:".bold(), result.states);
            eprintln!("{:18}{}", "Max memory usage:".bold(), result.max_memory);
            eprintln!("{:18}{}", "Generation time:".bold(), result.generation_time);
            eprintln!("{:18}{}", "Total time:".bold(), result.total_time);
            eprintln!("{:18}{}", "Min Value:".bold(), result.value);
            eprintln!("{:18}{}", "Horizon:".bold(), result.horizon);
        }
        Err(failure) => {
            eprintln!("{}", "Benchmark failed!".red().bold());
            eprintln!("{}", failure);
        }
    }
}

/// Run a single task in experiment.
#[allow(clippy::too_many_arguments)]
fn run_experiment_task(
    team_problem: &TeamProblem,
    optimization: &OptimizationInfo,
    problem: &Problem,
    config: &Config,
    solutions_dir: Option<&PathBuf>,
    simulate: bool,
    current: usize,
) -> serde_json::Value {
    eprintln!();
    print_optimizations(optimization);

    let solution = solve(problem, config, optimization);
    let result = get_optimization_result(&solution, optimization.clone());

    print_benchmark_result(&result.result);
    eprintln!();

    let mut result = match serde_json::to_value(result) {
        Ok(s) => s,
        Err(e) => fatal_error!(1, "Error while serializing results: {}", e),
    };
    let result_obj = result.as_object_mut().unwrap();

    if let Some(name) = &team_problem.name {
        result_obj.insert("name".to_string(), serde_json::Value::String(name.clone()));
    }

    if let Ok(solution) = solution {
        if simulate {
            let simulation_result = solution.simulate_all();
            result_obj.insert(
                "simulation".to_string(),
                serde_json::to_value(simulation_result)
                    .expect("Cannot serialize simulation result"),
            );
        }
        // Save solution
        if let Some(solutions_dir) = solutions_dir {
            let mut path = solutions_dir.clone();
            path.push(format!("{:03}.bin", current));
            let err = dmslib::io::fs::save_solution(team_problem.clone(), solution, &path);
            if let Err(e) = err {
                log::error!("Failed to save solution {}: {}", current, e);
            } else {
                result_obj.insert(
                    "solution".to_string(),
                    serde_json::Value::String(path.to_string_lossy().to_string()),
                );
            }
        }
    }

    result
}

/// Run all tasks in experiment.
fn run_experiment(
    experiment: Experiment,
    solutions_dir: Option<PathBuf>,
    simulate: bool,
) -> Vec<serde_json::Value> {
    eprintln!(
        "{:18}{}\n",
        "Experiment Name:".bold(),
        experiment.name.as_ref().map(String::as_ref).unwrap_or("-")
    );

    let mut current: usize = 1;
    let total_benchmarks: usize = experiment
        .tasks
        .iter()
        .map(|task| task.problems.len() * task.optimizations.len())
        .sum();

    let mut results: Vec<serde_json::Value> = Vec::new();

    for task in experiment.tasks.into_iter() {
        let ExperimentTask {
            problems,
            optimizations,
        } = task;
        for mut problem in problems {
            let team_problem = problem.clone();

            let name = problem.name.take();

            eprintln!(
                "{:18}{}",
                "Problem Name:".bold(),
                name.as_ref().map(String::as_ref).unwrap_or("-")
            );

            let (problem, config) = match problem.prepare() {
                Ok(x) => x,
                Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
            };

            for optimization in &optimizations {
                eprintln!(
                    "{}",
                    format!("Solving {}/{}...", current, total_benchmarks)
                        .green()
                        .bold()
                );

                results.push(run_experiment_task(
                    &team_problem,
                    optimization,
                    &problem,
                    &config,
                    solutions_dir.as_ref(),
                    simulate,
                    current,
                ));

                current += 1;
            }
        }
    }

    results
}

impl Run {
    pub fn run(self) {
        let Run {
            path,
            no_save,
            no_sim,
        } = self;

        let mut results_path = match std::env::current_dir() {
            Ok(p) => p,
            Err(e) => fatal_error!(1, "Cannot open current working directory: {}", e),
        };

        results_path.push(RESULTS_DIR);
        if let Err(e) = std::fs::create_dir_all(&results_path) {
            fatal_error!(1, "Cannot create results directory: {e}");
        }
        results_path.push(path.file_name().unwrap());
        if results_path.exists() {
            // TODO: overwrite this
            fatal_error!(
                1,
                "Results file is present: {}",
                results_path.to_string_lossy()
            );
        }
        let results_path = results_path;

        let solutions_dir = if no_save {
            None
        } else {
            let dir = results_path.with_extension("d");
            if let Err(e) = std::fs::create_dir_all(&dir) {
                fatal_error!(1, "Cannot create solutions directory: {e}");
            }
            Some(dir)
        };

        let experiment = match read_experiment_from_file(&path) {
            Ok(s) => s,
            Err(err) => fatal_error!(1, "Cannot parse experiment: {}", err),
        };

        let results = run_experiment(experiment, solutions_dir, !no_sim);

        let serialized = match serde_json::to_string_pretty(&results) {
            Ok(s) => s,
            Err(e) => fatal_error!(1, "Error while serializing results: {}", e),
        };

        // Save to file.
        let mut results_file = match std::fs::File::create(results_path) {
            Ok(f) => f,
            Err(e) => fatal_error!(1, "Cannot open results file: {}", e),
        };
        writeln!(&mut results_file, "{}", serialized).unwrap();

        eprintln!("{}", "Done!".green().bold());
    }
}

impl Solve {
    pub fn run(self) {
        let Solve {
            path,
            indexer,
            action,
            transition,
            json,
        } = self;

        let (name, problem, config) = read_and_parse_team_problem(path);

        eprintln!("{:18}{}", "Problem Name:".bold(), name);

        let optimizations = OptimizationInfo {
            indexer,
            actions: action,
            transitions: transition,
        };

        print_optimizations(&optimizations);

        eprint!("{}\r", "Solving...".green().bold());
        std::io::stderr().flush().unwrap();

        let solution = solve(&problem, &config, &optimizations);
        // TODO: save solution

        let result = get_optimization_result(&solution, optimizations);

        print_benchmark_result(&result.result);

        if json {
            let serialized = match serde_json::to_string_pretty(&result) {
                Ok(s) => s,
                Err(e) => fatal_error!(1, "Error while serializing results: {}", e),
            };
            println!("{}", serialized);
        }
    }
}
