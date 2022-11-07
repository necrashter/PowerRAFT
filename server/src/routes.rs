use log::{error, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use warp::{filters::BoxedFilter, Filter, Reply};
use warp::{http::StatusCode, reply};

/// Path to static files for the client.
const STATIC_PATH: &str = "../client";

/// Path where graphs are stored.
/// Must end with `/`, or all subdirectory names will start with `/`.
const GRAPHS_PATH: &str = "../graphs/";

/// Path where experiments are stored.
const EXPERIMENTS_PATH: &str = "../experiments/";

/// Generic response struct.
#[derive(Serialize, Deserialize, Debug)]
pub struct GenericOperationResult {
    pub successful: bool,
    pub error: Option<String>,
}

impl GenericOperationResult {
    /// Return a [`GenericOperationResult`] that denotes success.
    #[inline]
    pub fn success() -> GenericOperationResult {
        GenericOperationResult {
            successful: true,
            error: None,
        }
    }

    /// Return a [`GenericOperationResult`] with the given error.
    #[inline]
    pub fn err(e: String) -> GenericOperationResult {
        GenericOperationResult {
            successful: false,
            error: Some(e),
        }
    }
}

/// Every route combined for a single network
pub fn api() -> BoxedFilter<(impl Reply,)> {
    let get_graphs = warp::path!("get-graphs").and(warp::get()).map(|| {
        match list_graphs(Path::new(GRAPHS_PATH)) {
            Ok(list) => reply::with_status(reply::json(&list), StatusCode::OK),
            Err(error) => {
                error!("Error while getting the graph list: {error}");
                let error = error.to_string();
                reply::with_status(reply::json(&error), StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    });
    let policy = warp::path!("policy")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 1024))
        .and(warp::body::json())
        .map(|req: serde_json::Value| {
            let graph: dmslib::webclient::Graph = if let Some(field) = req.get("graph") {
                match serde_json::from_value(field.clone()) {
                    Ok(v) => v,
                    Err(e) => {
                        let error = format!("Failed to parse graph: {e}");
                        return reply::with_status(reply::json(&error), StatusCode::BAD_REQUEST);
                    }
                }
            } else {
                return reply::with_status(reply::json(&"No graph given"), StatusCode::BAD_REQUEST);
            };
            let teams: Vec<dmslib::webclient::Team> = if let Some(field) = req.get("teams") {
                match serde_json::from_value(field.clone()) {
                    Ok(v) => v,
                    Err(e) => {
                        let error = format!("Failed to parse teams: {e}");
                        return reply::with_status(reply::json(&error), StatusCode::BAD_REQUEST);
                    }
                }
            } else {
                return reply::with_status(
                    reply::json(&"No team info is given"),
                    StatusCode::BAD_REQUEST,
                );
            };
            let solution = match graph.solve_teams_problem(teams) {
                Ok(x) => x,
                Err(e) => {
                    let error = format!("Error while generating a solution: {e}");
                    return reply::with_status(reply::json(&error), StatusCode::BAD_REQUEST);
                }
            };
            reply::with_status(reply::json(&solution), StatusCode::OK)
        });
    let save_experiment = warp::path!("save-experiment")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 1024))
        .and(warp::body::json())
        .map(|mut req: serde_json::Value| {
            match req.as_object_mut() {
                Some(map) => {
                    map.remove("benchmark");
                }
                None => {
                    return reply::with_status(
                        reply::json(&GenericOperationResult::err(
                            "The type of request must be a JSON object.".to_string(),
                        )),
                        StatusCode::BAD_REQUEST,
                    );
                }
            }
            match save_experiment(&req) {
                Ok(_) => reply::with_status(
                    reply::json(&GenericOperationResult::success()),
                    StatusCode::OK,
                ),
                Err(e) => reply::with_status(
                    reply::json(&GenericOperationResult::err(e.to_string())),
                    if e.kind() == std::io::ErrorKind::Other {
                        StatusCode::BAD_REQUEST
                    } else {
                        StatusCode::INTERNAL_SERVER_ERROR
                    },
                ),
            }
        });
    let static_files = warp::any().and(warp::fs::dir(STATIC_PATH));
    let graph_files = warp::path("graphs").and(warp::fs::dir(GRAPHS_PATH));
    graph_files
        .or(static_files)
        .or(policy)
        .or(get_graphs)
        .or(save_experiment)
        .boxed()
}

#[derive(Serialize, Deserialize, Debug)]
struct View {
    lat: f32,
    lng: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct GraphEntry {
    filename: String,
    name: String,
    solution_file: Option<String>,
    view: View,
}

use std::io::prelude::*;
use std::path::Path;

// one possible implementation of walking a directory only visiting files
fn list_graphs(dir: &Path) -> std::io::Result<HashMap<String, Vec<GraphEntry>>> {
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
                                warn!("Cannot parse \"view\" member of {filename}: {e}");
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

fn save_experiment(content: &serde_json::Value) -> std::io::Result<()> {
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
    let name = name + ".json";
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
