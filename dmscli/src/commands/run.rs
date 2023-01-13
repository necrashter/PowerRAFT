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
    result: &Result<BenchmarkResult, SolveFailure>,
) -> std::io::Result<()> {
    let mut bold = ColorSpec::new();
    bold.set_bold(true);

    match result {
        Ok(result) => {
            out.set_color(&bold)?;
            write!(out, "Number of states: ")?;
            out.reset()?;
            writeln!(out, "{}", result.states)?;
            out.set_color(&bold)?;
            write!(out, "Max memory usage: ")?;
            out.reset()?;
            writeln!(out, "{}", result.max_memory)?;

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
        }
        Err(failure) => {
            out.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Red)))?;
            writeln!(out, "Benchmark failed!")?;
            out.reset()?;
            writeln!(out, "{}", failure)?;
        }
    }

    Ok(())
}

/// Run a single task in experiment.
fn run_experiment_task(
    stderr: &mut StandardStream,
    team_problem: &TeamProblem,
    optimization: &OptimizationInfo,
    problem: &Problem,
    config: &Config,
    solutions_dir: Option<&PathBuf>,
    current: usize,
) -> serde_json::Value {
    writeln!(stderr).unwrap();
    print_optimizations(stderr, optimization).unwrap();

    let solution = solve(problem, config, optimization);
    let result = get_optimization_result(&solution, optimization.clone());

    print_benchmark_result(stderr, &result.result).unwrap();
    writeln!(stderr).unwrap();

    let mut result = match serde_json::to_value(result) {
        Ok(s) => s,
        Err(e) => fatal_error!(1, "Error while serializing results: {}", e),
    };
    let result_obj = result.as_object_mut().unwrap();

    if let Some(name) = &team_problem.name {
        result_obj.insert("name".to_string(), serde_json::Value::String(name.clone()));
    }

    // Save solution
    if let Ok(solution) = solution {
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
    solutions_dir: Option<&PathBuf>,
) -> Vec<serde_json::Value> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);

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

    let mut results: Vec<serde_json::Value> = Vec::new();

    for task in experiment.tasks.into_iter() {
        let ExperimentTask {
            problems,
            optimizations,
        } = task;
        for mut problem in problems {
            let team_problem = problem.clone();

            let name = problem.name.take();

            stderr.set_color(ColorSpec::new().set_bold(true)).unwrap();
            write!(&mut stderr, "Problem Name:     ").unwrap();
            stderr.reset().unwrap();
            writeln!(
                &mut stderr,
                "{}",
                name.as_ref().map(String::as_ref).unwrap_or("-")
            )
            .unwrap();

            let (problem, config) = match problem.prepare() {
                Ok(x) => x,
                Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
            };

            for optimization in &optimizations {
                stderr
                    .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
                    .unwrap();
                writeln!(&mut stderr, "Solving {}/{}...", current, total_benchmarks).unwrap();
                stderr.reset().unwrap();
                stderr.flush().unwrap();

                results.push(run_experiment_task(
                    &mut stderr,
                    &team_problem,
                    optimization,
                    &problem,
                    &config,
                    solutions_dir,
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
        let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        let Run { path } = self;

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

        let solutions_dir = results_path.with_extension("d");
        if let Err(e) = std::fs::create_dir_all(&solutions_dir) {
            fatal_error!(1, "Cannot create solutions directory: {e}");
        }

        let experiment = match read_experiment_from_file(&path) {
            Ok(s) => s,
            Err(err) => fatal_error!(1, "Cannot parse experiment: {}", err),
        };

        let results = run_experiment(experiment, Some(&solutions_dir));

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

        stderr
            .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
            .unwrap();
        writeln!(&mut stderr, "Done!").unwrap();
        stderr.reset().unwrap();
    }
}

impl Solve {
    pub fn run(self) {
        let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        let Solve {
            path,
            indexer,
            action,
            transition,
            json,
        } = self;

        let (name, problem, config) = read_and_parse_team_problem(path);

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

        let solution = solve(&problem, &config, &optimizations);
        // TODO: save solution

        let result = get_optimization_result(&solution, optimizations);

        print_benchmark_result(&mut stderr, &result.result).unwrap();

        if json {
            let serialized = match serde_json::to_string_pretty(&result) {
                Ok(s) => s,
                Err(e) => fatal_error!(1, "Error while serializing results: {}", e),
            };
            println!("{}", serialized);
        }
    }
}
