//! Server routes module.
use dmslib::io::fs::*;
use dmslib::GRAPHS_PATH;

use serde::{Deserialize, Serialize};
use std::path::Path;
use warp::{filters::BoxedFilter, Filter, Reply};
use warp::{http::StatusCode, reply};

/// Path to static files for the client.
pub const STATIC_PATH: &str = "../client";

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
                log::error!("Error while getting the graph list: {error}");
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
            let graph: dmslib::io::Graph = if let Some(field) = req.get("graph") {
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
            let teams: Vec<dmslib::io::Team> = if let Some(field) = req.get("teams") {
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
