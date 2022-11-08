//! A module responsible for the DMS file system operations.
use super::{GraphEntry, TeamProblem, View};
use crate::EXPERIMENTS_PATH;

use itertools::Itertools;

use std::collections::HashMap;
use std::io::prelude::*;
use std::path::Path;

/// Yields a list of the graph `.json` files for the root directory and each subdirectory,
/// Root directory will have an empty string key in the HashMap, and others will have their
/// relative path as their key.
pub fn list_graphs(dir: &Path) -> std::io::Result<HashMap<String, Vec<GraphEntry>>> {
    if !dir.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Path {} is not a directory.", dir.to_string_lossy()),
        ));
    }
    let mut q = vec![dir.to_path_buf()];
    let mut all_graphs: HashMap<String, Vec<GraphEntry>> = HashMap::new();
    let rootdirstr = dir.to_path_buf().into_os_string().into_string().unwrap();
    let rootdirstrlen = rootdirstr.len();
    while let Some(dir) = q.pop() {
        let mut entries: Vec<GraphEntry> = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                q.push(path);
            } else {
                let filename = String::from(
                    &path.clone().into_os_string().into_string().unwrap()[rootdirstrlen..],
                );
                if !filename.ends_with(".soln.json") && filename.ends_with(".json") {
                    let data = std::fs::read_to_string(&path)?;
                    let data: serde_json::Value = serde_json::from_str(&data)?;
                    let name = if let Some(serde_json::Value::String(name)) = data.get("name") {
                        String::from(name)
                    } else {
                        String::from(path.file_stem().unwrap().to_str().unwrap())
                    };
                    let solnpath = path.with_extension(".soln.json");
                    let solution_file = if solnpath.exists() {
                        Some(String::from(
                            &solnpath.into_os_string().into_string().unwrap()[rootdirstrlen..],
                        ))
                    } else {
                        None
                    };
                    let view: View = if let Some(view) = data.get("view") {
                        match serde_json::from_value(view.clone()) {
                            Ok(v) => v,
                            Err(e) => {
                                log::warn!("Cannot parse \"view\" member of {filename}: {e}");
                                continue;
                            }
                        }
                    } else {
                        // Ignore files without view silently.
                        continue;
                    };
                    let entry = GraphEntry {
                        filename,
                        name,
                        solution_file,
                        view,
                    };
                    entries.push(entry);
                }
            }
        }
        let dirname = String::from(&dir.into_os_string().into_string().unwrap()[rootdirstrlen..]);
        all_graphs.insert(dirname, entries);
    }
    Ok(all_graphs)
}

/// Convert a string to sanitized JSON filename.
pub fn name_to_json(name: &str) -> String {
    let name = name.split_whitespace().join("-");
    let name = name + ".json";
    sanitize_filename::sanitize(name)
}

/// Given a `serde_json::Value`, save it to the [`EXPERIMENTS_PATH`] as a human-readable (pretty)
/// JSON file.
pub fn save_experiment(content: &serde_json::Value) -> std::io::Result<()> {
    let name: String = match content.get("name") {
        Some(name) => match name.as_str() {
            Some(s) => s.to_owned(),
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Experiment has no name".to_string(),
                ));
            }
        },
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Experiment has no name".to_string(),
            ));
        }
    };
    let name = name_to_json(&name);
    let path = Path::new(EXPERIMENTS_PATH).join(name);
    let path = path.as_path();
    let mut file = std::fs::File::options()
        .read(false)
        .write(true)
        .create_new(true)
        .open(path)?;
    let content = match serde_json::to_string_pretty(content) {
        Ok(s) => s,
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    };
    file.write_all(content.as_bytes())?;
    log::info!("Saved experiment: {}", path.display());
    Ok(())
}

impl TeamProblem {
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<TeamProblem> {
        let content = std::fs::read_to_string::<P>(path)?;
        let team_problem: TeamProblem = serde_json::from_str(&content)?;
        Ok(team_problem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_to_json() {
        assert_eq!(
            name_to_json("WSCC 9-bus System Test 1"),
            "WSCC-9-bus-System-Test-1.json"
        );
        assert_eq!(
            name_to_json("/WSCC    9-bus System Test 1"),
            "WSCC-9-bus-System-Test-1.json"
        );
        assert_eq!(
            name_to_json("\\/?WSCC    9-?bus System    Test 1"),
            "WSCC-9-bus-System-Test-1.json"
        );
    }
}
