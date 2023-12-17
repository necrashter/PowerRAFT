use std::path::{Path, PathBuf};

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OptimizationInfo {
    /// State indexer class
    pub indexer: String,
    /// Action set definition
    pub actions: String,
    /// Action applier
    pub transitions: String,
    /// Explorer class
    #[serde(default = "default_explorer")]
    pub explorer: String,
}

fn default_explorer() -> String {
    "NaiveExplorer".to_string()
}

pub fn serialize_benchmark_result<S>(
    result: &Result<BenchmarkResult, SolveFailure>,
    s: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match result {
        Ok(result) => {
            let mut ser = s.serialize_map(Some(1))?;
            ser.serialize_entry("success", result)?;
            ser.end()
        }
        Err(failure) => {
            let mut ser = s.serialize_map(Some(2))?;
            ser.serialize_entry("error", failure)?;
            ser.serialize_entry("description", format!("{}", failure).as_str())?;
            ser.end()
        }
    }
}

#[derive(Serialize, Debug)]
pub struct OptimizationBenchmarkResult {
    pub optimizations: OptimizationInfo,
    #[serde(serialize_with = "serialize_benchmark_result")]
    pub result: Result<BenchmarkResult, SolveFailure>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExperimentTask {
    pub problems: Vec<TeamProblem>,
    pub optimizations: Vec<OptimizationInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Experiment {
    pub name: Option<String>,
    pub tasks: Vec<ExperimentTask>,
}

pub fn read_experiment_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Experiment> {
    let content = std::fs::read_to_string(&path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;
    read_experiment_from_value(value, path)
}

pub fn read_experiment_from_value<P: AsRef<Path>>(
    value: serde_json::Value,
    path: P,
) -> std::io::Result<Experiment> {
    let path = {
        let mut p = PathBuf::new();
        p.push(path);
        p
    };
    if let serde_json::Value::Object(mut map) = value {
        let name = if let Some(serde_json::Value::String(s)) = map.get("name").take() {
            Some(s.clone())
        } else {
            None
        };
        let tasks = map.get_mut("tasks");
        if let Some(tasks) = tasks {
            if let serde_json::Value::Array(a) = tasks.take() {
                let tasks = a
                    .into_iter()
                    .map(|mut v| -> std::io::Result<ExperimentTask> {
                        fs::read_field_from_file(&mut v, "optimizations", &path)?;
                        let taskmap = if let serde_json::Value::Object(mut v) = v {
                            if let Some(serde_json::Value::Array(problems)) = v.get_mut("problems")
                            {
                                for problem in problems.iter_mut() {
                                    fs::read_field_from_file(problem, "graph", &path)?;
                                }
                            }
                            v
                        } else {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Each task must be an object",
                            ));
                        };
                        let v = serde_json::Value::Object(taskmap);
                        let e: ExperimentTask = serde_json::from_value(v)?;
                        Ok(e)
                    })
                    .collect::<std::io::Result<Vec<ExperimentTask>>>()?;
                Ok(Experiment { name, tasks })
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
            "Experiment must be a JSON object",
        ))
    }
}
