use super::*;

mod run;

mod list;
pub use list::*;

mod simulation;

mod dqn;
pub use dqn::DqnCommand;

/// All CLI commands available in this binary.
#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Run an experiment.
    #[command(alias = "r")]
    Run(Run),

    /// Solve a problem with custom optimizations.
    #[command(alias = "s")]
    Solve(Solve),

    /// Print the travel time matrix for a field-teams problem.
    #[command(alias = "tt")]
    TravelTimes(TravelTimes),

    /// Print the direct distance matrix for a field-teams problem.
    #[command(alias = "d")]
    Distances(Distances),

    /// Print the list of all possible optimizations.
    ListAllOpt,

    /// Load the solution and exit (check integrity).
    Load(Load),

    /// Subcommand for Deep Q-Learning
    #[command(subcommand)]
    Dqn(DqnCommand),
}

#[derive(clap::Args, Debug)]
pub struct Run {
    /// Path to the experiment JSON file.
    path: PathBuf,
    /// Don't save solutions alongside results JSON file.
    #[arg(long, default_value_t = false)]
    no_save: bool,
    /// Don't simulate the restoration process.
    #[arg(long, default_value_t = false)]
    no_sim: bool,
}

#[derive(clap::Args, Debug)]
pub struct Solve {
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
    /// Explorer class.
    #[arg(short, long, default_value = "NaiveExplorer")]
    explorer: String,
    /// Print the results as JSON (Hint: redirect stdout)
    #[arg(short, long, default_value_t = false)]
    json: bool,
}

#[derive(clap::Args, Debug)]
pub struct TravelTimes {
    /// Path to the JSON file containing the problem.
    path: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct Distances {
    /// Path to the JSON file containing the problem.
    path: PathBuf,
    /// Number of decimal places in output.
    #[arg(short, long, default_value_t = 3)]
    precision: usize,
}

#[derive(clap::Args, Debug)]
pub struct Load {
    /// Path to the binary file containing the solution.
    path: PathBuf,
}

impl Command {
    pub fn run(self) {
        match self {
            Command::Run(args) => args.run(),
            Command::Solve(args) => args.run(),
            Command::TravelTimes(args) => args.run(),
            Command::Distances(args) => args.run(),
            Command::ListAllOpt => list_all_opt(),
            Command::Load(args) => args.run(),
            Command::Dqn(sub) => sub.run(),
        }
    }
}
