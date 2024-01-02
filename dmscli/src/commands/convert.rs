use super::*;

#[derive(clap::Args, Debug)]
pub struct Convert {
    /// Path to the binary file containing the solution.
    binary: PathBuf,
    /// Path to the JSON file that will be created.
    json: PathBuf,
}

impl Convert {
    pub fn run(self) {
        // let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        let Convert {
            binary: binary_path,
            json: json_path,
        } = self;

        if json_path.exists() {
            fatal_error!(1, "Output file already exists!");
        }

        let save_file = match dmslib::io::fs::load_solution(binary_path) {
            Ok(s) => s,
            Err(e) => fatal_error!(1, "Error while loading the solution: {}", e),
        };

        let json = match save_file.solution {
            GenericTeamSolution::Timed(solution) => serde_json::to_string(&solution),
            GenericTeamSolution::Regular(solution) => serde_json::to_string(&solution),
        };

        let json = match json {
            Ok(json) => json,
            Err(e) => fatal_error!(1, "Error while converting to JSON: {}", e),
        };

        let mut file = match std::fs::File::options()
            .read(false)
            .write(true)
            .create_new(true)
            .open(&json_path)
        {
            Ok(file) => file,
            Err(e) => fatal_error!(1, "Error while opening the JSON file: {}", e),
        };

        if let Err(e) = file.write_all(json.as_bytes()) {
            fatal_error!(1, "Error while writing the JSON file: {}", e);
        }

        drop(file);

        println!(
            "{} Saved the JSON file: {}",
            "SUCCESS!".bold().green(),
            json_path.display()
        );
    }
}
