//! Input/Output for Deep Q-Learning Module.
use std::path::{Path, PathBuf};

use crate::dqn;

use super::*;

/// A YAML file that serializes DqnModel.
#[derive(Serialize, Deserialize, Debug)]
pub struct DqnModelYaml {
    pub name: Option<String>,
    /// The field teams problem on which the model will run.
    pub problem: String,
    /// Model settings.
    pub model: dqn::ModelSettings,
    /// Trainer settings.
    pub trainer: dqn::TrainerSettings,
    /// Evaluation settings.
    #[serde(default)]
    pub evaluation: dqn::EvaluationSettings,
}

/// Contains all hyperparameter information about a DQN model.
#[derive(Debug)]
pub struct DqnModel {
    pub name: Option<String>,
    /// The field teams problem on which the model will run.
    pub problem: TeamProblem,
    /// Model settings.
    pub model: dqn::ModelSettings,
    /// Trainer settings.
    pub trainer: dqn::TrainerSettings,
    /// Evaluation settings.
    pub evaluation: dqn::EvaluationSettings,
}

impl DqnModel {
    /// Read the model information from a YAML file.
    pub fn read_yaml_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        use std::io::{Error, ErrorKind};

        // Read model YAML
        let content = std::fs::read_to_string(&path)?;
        let DqnModelYaml {
            name,
            problem: problem_relative_path,
            model,
            trainer,
            evaluation,
        } = match serde_yaml::from_str(&content) {
            Ok(yaml) => yaml,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("Failed to parse model YAML: {error}"),
                ));
            }
        };

        // Read the problem
        let mut problem_path = PathBuf::new();
        problem_path.push(path);
        problem_path.pop();
        problem_path.push(problem_relative_path);
        let problem = match TeamProblem::read_from_file(&problem_path) {
            Ok(value) => value,
            Err(error) => {
                return Err(Error::new(
                    error.kind(),
                    format!(
                        "Failed to read the problem ({}): {}",
                        problem_path.display(),
                        error
                    ),
                ));
            }
        };

        Ok(Self {
            name,
            problem,
            model,
            trainer,
            evaluation,
        })
    }
}
