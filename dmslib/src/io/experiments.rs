use std::path::{Path, PathBuf};

use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct OptimizationInfo {
    /// Action set definition
    pub actions: String,
    /// Action applier
    pub transitions: String,
}

#[derive(Serialize, Debug)]
pub struct OptimizationBenchmarkResult {
    pub optimizations: OptimizationInfo,
    pub result: BenchmarkResult,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ExperimentTask {
    Benchmark {
        problem: TeamProblem,
        optimizations: OptimizationInfo,
    },
    BenchmarkAll {
        problem: TeamProblem,
        optimizations: Vec<OptimizationInfo>,
    },
}

fn read_experiments<P: AsRef<Path>>(
    v: Vec<serde_json::Value>,
    path: P,
) -> std::io::Result<Vec<ExperimentTask>> {
    v.into_iter()
        .map(|mut v| -> std::io::Result<ExperimentTask> {
            if let Some(problem) = v.get_mut("problem") {
                fs::read_field_from_file(problem, "graph", &path)?;
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Problem is missing graph field",
                ));
            }
            let e: ExperimentTask = serde_json::from_value(v)?;
            Ok(e)
        })
        .collect::<std::io::Result<Vec<ExperimentTask>>>()
}

pub fn read_experiments_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<ExperimentTask>> {
    let content = std::fs::read_to_string(&path)?;
    let path = {
        let mut p = PathBuf::new();
        p.push(path);
        p
    };
    let value: serde_json::Value = serde_json::from_str(&content)?;
    if let serde_json::Value::Array(a) = value {
        read_experiments(a, path)
    } else if let serde_json::Value::Object(mut map) = value {
        let tasks = map.get_mut("tasks");
        if let Some(tasks) = tasks {
            if let serde_json::Value::Array(a) = tasks.take() {
                read_experiments(a, path)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "tasks field is not array",
                ))
            }
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Missing field: tasks",
            ))
        }
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Cannot recognize data structure",
        ))
    }
}
