use std::{
    io::{stdout, Write},
    time::Instant,
};

use dmslib::{dqn::EvaluationResult, io::DqnModel, types::Value};

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
fn load_model(path: PathBuf) -> DqnModel {
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

impl DqnCommand {
    pub fn run(self) {
        match self {
            DqnCommand::Train(args) => {
                let DqnModel {
                    name: _,
                    problem,
                    model,
                    trainer,
                } = load_model(args.model.path);

                println!("\nInitializing...");

                let (problem, config) = match problem.prepare() {
                    Ok(x) => x,
                    Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
                };
                let mut trainer =
                    trainer.build(&problem.graph, problem.initial_teams.clone(), model, config);
                let EvaluationResult {
                    value,
                    avg_q,
                    states,
                } = trainer.evaluate();
                println!(
                    "\n{:23} || Value: {} | Avg. Q: {} | States: {}",
                    "Pre-training Evaluation".dimmed().bold(),
                    format!("{:>8.2}", value).bold(),
                    format!("{:>8.2}", avg_q).bold(),
                    format!("{:>8}", states).bold(),
                );

                println!("{}", "Starting training...".green());

                let mut values = Vec::<Value>::new();
                let iterations = 500;

                RUNNING_STATE.store(2, atomic::Ordering::SeqCst);
                let mut i = 0;
                loop {
                    i += 1;
                    let start = Instant::now();
                    let loss = trainer.train(iterations);
                    let EvaluationResult {
                        value,
                        avg_q,
                        states,
                    } = trainer.evaluate();
                    println!(
                        "{} Loss: {} || Value: {} | Avg. Q: {} | States: {}",
                        format!("[{i:>4}]").green().bold(),
                        format!("{:>10.4}", loss).bold(),
                        format!("{:>8.2}", value).bold(),
                        format!("{:>8.2}", avg_q).bold(),
                        format!("{:>8}", states).bold(),
                    );
                    values.push(value);
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

                println!("\n{}", "Training finished.".green().bold());
                println!("Trained for {i} x {iterations} iterations.");
                println!(
                    "{:18}{}",
                    "Minimum Value:".bold(),
                    values.into_iter().reduce(Value::min).unwrap()
                );
            }
            DqnCommand::Run(args) => {
                load_model(args.path);
                todo!()
            }
        }
    }
}
