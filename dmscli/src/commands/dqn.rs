use dmslib::{io::DqnModel, types::Value};

use super::*;

/// Deep Q-Learning Commands
#[derive(clap::Subcommand, Debug)]
pub enum DqnCommand {
    /// Train a DQN.
    Train(ModelArgs),
    /// Run a DQN.
    Run(ModelArgs),
}

#[derive(clap::Args, Debug)]
pub struct ModelArgs {
    /// Path to the model YAML file.
    path: PathBuf,
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
                } = load_model(args.path);

                println!("\n{}", "Starting training...".green().bold());

                let (problem, config) = match problem.prepare() {
                    Ok(x) => x,
                    Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
                };
                let mut trainer =
                    trainer.build(&problem.graph, problem.initial_teams.clone(), model, config);
                let value = trainer.evaluate();
                println!(
                    "{:31} | Value: {:20}",
                    "Evaluation before training".dimmed().bold(),
                    format!("{}", value).bold(),
                );

                let mut values = Vec::<Value>::new();
                let iterations = 500;

                RUNNING_STATE.store(2, atomic::Ordering::SeqCst);
                let mut i = 0;
                loop {
                    i += 1;
                    let loss = trainer.train(iterations);
                    let value = trainer.evaluate();
                    println!(
                        // NOTE: .18 is the maximum width instead of precision
                        // since the inputs are strings.
                        "{} Loss: {:18.18} | Value: {:18.18}",
                        format!("[{i:>4}]").green().bold(),
                        format!("{}", loss).bold(),
                        format!("{}", value).bold(),
                    );
                    values.push(value);
                    // Check if an interrupt is received
                    if RUNNING_STATE.load(atomic::Ordering::SeqCst) & 1 == 1 {
                        break;
                    }
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
