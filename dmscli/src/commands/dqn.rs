use dmslib::io::DqnModel;

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
    /// Path to the model JSON file.
    path: PathBuf,
}

/// Load model and print name information.
fn load_model(path: PathBuf) -> DqnModel {
    println!("Loading model: {}", path.display());
    match DqnModel::read_from_file(path) {
        Ok(model) => {
            println!("Model Name: {}", model.name.as_deref().unwrap_or("-"));
            println!("Problem Name: {}", model.name.as_deref().unwrap_or("-"));
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

                println!("\nStarting training...");

                let (problem, config) = match problem.prepare() {
                    Ok(x) => x,
                    Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
                };
                let mut trainer =
                    trainer.build(&problem.graph, problem.initial_teams.clone(), model, config);
                let value = trainer.evaluate();
                println!("value: {value}");
                for i in 1..=1000 {
                    let loss = trainer.train(500);
                    let value = trainer.evaluate();
                    println!("Epoch {i}\n\tloss: {loss}\n\tvalue: {value}");
                }
            }
            DqnCommand::Run(args) => {
                load_model(args.path);
                todo!()
            }
        }
    }
}
