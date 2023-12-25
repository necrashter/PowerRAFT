//! Input/Output for Deep Q-Learning Module.
use std::path::{Path, PathBuf};

use crate::dqn;

use super::*;

/// Contains all hyperparameter information about a DQN model.
#[derive(Serialize, Deserialize, Debug)]
pub struct DqnModel {
    pub name: Option<String>,
    /// The field teams problem on which the model will run.
    pub problem: TeamProblem,
    /// Model settings.
    pub model: dqn::ModelSettings,
    /// Trainer settings.
    pub trainer: dqn::TrainerSettings,
}

impl DqnModel {
    pub fn read_from_value<P: AsRef<Path>>(
        mut value: serde_json::Value,
        path: P,
    ) -> std::io::Result<Self> {
        if let Some(value) = value.get_mut("problem") {
            if let serde_json::Value::String(s) = value {
                let mut problem_path = PathBuf::new();
                problem_path.push(path);
                problem_path.pop();
                problem_path.push(s);
                *value = TeamProblem::read_value_from_file(problem_path)?;
            } else {
                TeamProblem::process_serde_value(value, path)?;
            }
        }
        let out: Self = serde_json::from_value(value)?;
        Ok(out)
    }

    pub fn read_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let value: serde_json::Value = serde_json::from_str(&content)?;
        Self::read_from_value(value, path)
    }
}
