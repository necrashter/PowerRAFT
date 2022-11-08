use std::path::PathBuf;

use dmslib::io::TeamProblem;

use clap::Parser;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the experiment JSON file
    path: PathBuf,

    /// Action set class to use
    action_set: String,

    /// Action applier class to use. When omitted (default), NaiveActionApplier will be used.
    action_applier: Option<String>,
}

fn main() {
    let args = Args::parse();

    println!("Solving team problem: {}", args.path.to_str().unwrap());
    let problem = match TeamProblem::read_from_file(args.path) {
        Ok(x) => x,
        Err(err) => {
            eprintln!("Cannot read team problem: {}", err);
            return;
        }
    };

    let action_set = args.action_set;

    if let Some(action_applier) = args.action_applier {
        let solution = match problem.solve_custom_timed(&action_set, &action_applier) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("Cannot solve team problem: {}", err);
                return;
            }
        };
        println!("Number of states: {}", solution.transitions.len());
        println!("Generation time: {}", solution.generation_time);
        println!("Total time: {}", solution.total_time);
        println!(
            "MinValue: {}",
            solution.values[0]
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
        );
    } else {
        let solution = match problem.solve_custom_regular(&action_set) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("Cannot solve team problem: {}", err);
                return;
            }
        };
        println!("Number of states: {}", solution.transitions.len());
        println!("Generation time: {}", solution.generation_time);
        println!("Total time: {}", solution.total_time);
        println!(
            "MinValue: {}",
            solution.values[0]
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
        );
    }
}
