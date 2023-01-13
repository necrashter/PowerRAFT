use super::*;

mod run;
pub use run::*;

mod list;
pub use list::*;

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
}

#[derive(clap::Args, Debug)]
pub struct Run {
    /// Path to the experiment JSON file.
    path: PathBuf,
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

impl Command {
    pub fn run(self) {
        match self {
            Command::Run(args) => args.run(),
            Command::Solve(args) => args.run(),
            Command::TravelTimes(args) => args.run(),
            Command::Distances(args) => args.run(),
            Command::ListAllOpt => list_all_opt(),
        }
    }
}
