use std::{
    fmt::Display,
    io::{stdout, Write},
    time::Instant,
};

use dmslib::{
    dqn::{load_torch_seed, EvaluationResult, EvaluationSettings},
    io::DqnModel,
};

use super::*;

/// Deep Q-Learning Commands
#[derive(clap::Subcommand, Debug)]
pub enum DqnCommand {
    /// Train a DQN.
    Train(TrainArgs),
    /// Run a DQN.
    Run(ModelArgs),
}

#[derive(clap::Args, Debug)]
pub struct ModelArgs {
    /// Path to the model YAML file.
    path: PathBuf,
    /// Force Torch to use CPU. By default, CUDA will be used if available.
    #[arg(long, default_value_t = false)]
    cpu: bool,
    /// How many actions to select from the network in each state.
    #[arg(long)]
    top_k: Option<usize>,
}

#[derive(clap::Args, Debug)]
pub struct TrainArgs {
    #[command(flatten)]
    model: ModelArgs,
    /// Number of checkpoints to train.
    #[arg(short, long)]
    checkpoints: Option<usize>,
}

/// Load model and print name information.
fn load_model(path: &Path) -> DqnModel {
    match DqnModel::read_yaml_file(path) {
        Ok(model) => {
            println!(
                "{:14}{}",
                "Model Name:".bold(),
                model.name.as_deref().unwrap_or("-")
            );
            println!(
                "{:14}{}",
                "Problem Name:".bold(),
                model.problem.name.as_deref().unwrap_or("-")
            );
            model
        }
        Err(err) => fatal_error!(1, "Cannot read model info: {}", err),
    }
}

fn get_device(cpu_flag: bool) -> tch::Device {
    let device = if cpu_flag {
        tch::Device::Cpu
    } else {
        tch::Device::cuda_if_available()
    };
    println!(
        "Selected Torch device: {}",
        format!("{device:?}").bold().blue(),
    );
    device
}

fn get_latest_checkpoint<P: AsRef<Path>>(model_dir: P) -> Option<(usize, PathBuf)> {
    if let Err(e) = std::fs::create_dir_all(&model_dir) {
        fatal_error!(1, "Cannot create model directory: {e}");
    }

    // Read the directory and filter files with the ".safetensors" extension
    let safetensor_files = match std::fs::read_dir(model_dir) {
        Ok(entries) => entries,
        Err(e) => fatal_error!(1, "Cannot read model directory: {e}"),
    };
    let safetensor_files: Vec<_> = safetensor_files
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.is_file()
                    && path.extension().and_then(|ext| ext.to_str()) == Some("safetensors")
                {
                    path.file_stem()
                        .and_then(|stem| stem.to_str())
                        .and_then(|name| name.parse::<usize>().ok())
                        .map(|number| (number, path))
                } else {
                    None
                }
            })
        })
        .collect();

    // Find the file with the largest number in the name
    safetensor_files.into_iter().max_by_key(|e| e.0)
}

fn append_to_file<P: AsRef<Path>, V: Display>(path: P, value: V) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    writeln!(file, "{}", value)
}

fn update_evaluation_settings(
    mut settings: EvaluationSettings,
    args: &ModelArgs,
) -> EvaluationSettings {
    if let Some(top_k) = args.top_k {
        settings.top_k = top_k
    }
    settings
}

const VALUES_FILE: &str = "values.txt";

fn train(args: TrainArgs) {
    let mut model_dir = args.model.path.with_extension("d");
    let mut values_file = model_dir.clone();
    values_file.push(VALUES_FILE);

    let DqnModel {
        name: _,
        problem,
        model,
        trainer,
        evaluation,
        checkpoint_iterations,
    } = load_model(&args.model.path);
    let evaluation = update_evaluation_settings(evaluation, &args.model);

    println!("\nInitializing...");

    load_torch_seed();

    let (problem, config) = match problem.prepare() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };

    let device = get_device(args.model.cpu);

    let mut trainer = match trainer.build(
        &problem.graph,
        problem.initial_teams.clone(),
        model,
        config,
        device,
    ) {
        Ok(trainer) => trainer,
        Err(e) => fatal_error!(1, "{}", e),
    };

    // Load checkpoint if present
    let mut checkpoint: usize = if let Some((i, path)) = get_latest_checkpoint(&model_dir) {
        println!("Loading checkpoint: {}", format!("{i}").bold());
        if let Err(e) = trainer.load_checkpoint(&path) {
            fatal_error!(1, "Error while loading the checkpoint: {}", e);
        }
        i
    } else {
        println!("No checkpoint found, starting over.");
        0
    };

    let EvaluationResult { value, states } = trainer.evaluate(evaluation);
    println!(
        "\n{:24} || Value: {} | States: {}",
        "Pre-training Evaluation".dimmed().bold(),
        format!("{:>8.2}", value).bold(),
        format!("{:>8}", states).bold(),
    );

    println!("{}", "Starting training...".green());
    let training_start = Instant::now();

    let mut results = Vec::<(usize, EvaluationResult)>::new();

    RUNNING_STATE.store(2, atomic::Ordering::SeqCst);
    let mut i = 0;
    loop {
        i += 1;
        checkpoint += 1;

        let start = Instant::now();
        let loss = trainer.train(checkpoint_iterations);
        let evaluation_result = trainer.evaluate(evaluation);
        println!(
            "{} Loss: {} || Value: {} | States: {}",
            format!("[{:>5}]", checkpoint).green().bold(),
            format!("{:>10.4}", loss).bold(),
            format!("{:>8.2}", evaluation_result.value).bold(),
            format!("{:>8}", evaluation_result.states).bold(),
        );
        if let Err(e) = append_to_file(&values_file, evaluation_result.value) {
            eprintln!("{} Failed to append value: {e}", "[ERROR]".red().bold());
        }
        results.push((checkpoint, evaluation_result));

        model_dir.push(format!("{checkpoint}.safetensors"));
        if let Err(e) = trainer.save_checkpoint(&model_dir) {
            eprintln!("{} Failed to save checkpoint: {e}", "[ERROR]".red().bold());
        }
        model_dir.pop();

        // Check if we reached the checkpoint limit.
        if let Some(limit) = args.checkpoints {
            if i >= limit {
                break;
            }
        }
        // Check if an interrupt is received
        if RUNNING_STATE.load(atomic::Ordering::SeqCst) & 1 == 1 {
            break;
        }
        print!("  {}\r", format_duration(&start.elapsed()));
        stdout().flush().unwrap();
    }
    RUNNING_STATE.store(0, atomic::Ordering::SeqCst);

    let duration = format_duration(&training_start.elapsed());

    println!("\n{}", "Training finished.".green().bold());
    println!("Trained for {i} x {checkpoint_iterations} iterations in {duration}.");

    let (best_checkpoint, best_result) = results
        .into_iter()
        .reduce(|acc, e| if e.1.value < acc.1.value { e } else { acc })
        .unwrap();
    println!("\n{}", "Best Evaluation:".bold().underline());
    println!("    {:13}{}", "Checkpoint:".bold(), best_checkpoint);
    println!("    {:13}{}", "Value:".bold(), best_result.value);
    println!("    {:13}{}", "States:".bold(), best_result.states);
}

fn run(args: ModelArgs) {
    load_model(&args.path);
    todo!()
}

impl DqnCommand {
    pub fn run(self) {
        match self {
            DqnCommand::Train(args) => train(args),
            DqnCommand::Run(args) => run(args),
        }
    }
}
