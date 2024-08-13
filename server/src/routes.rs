//! Server routes module.
use dmslib::io::fs::*;
use dmslib::GRAPHS_PATH;

use std::path::Path;
use warp::{filters::BoxedFilter, Filter, Reply};
use warp::{http::StatusCode, reply};

/// Path to static files for the client.
pub const STATIC_PATH: &str = "../client";

/// Content length limit for JSON requests.
const JSON_CONTENT_LIMIT: u64 = 8 * 1024 * 1024;

/// Every route combined for a single network
pub fn api() -> BoxedFilter<(impl Reply,)> {
    let static_files = warp::any().and(warp::fs::dir(STATIC_PATH));
    let graph_files = warp::path("graphs").and(warp::fs::dir(GRAPHS_PATH));

    graph_files
        .or(static_files)
        .or(warp::path!("policy")
            .and(warp::post())
            .and(warp::body::content_length_limit(JSON_CONTENT_LIMIT))
            .and(warp::body::json())
            .map(|req: dmslib::io::TeamProblem| {
                if req.teams.is_empty() {
                    // Non-team problem
                    let (graph, config) = match req.prepare_nonteam() {
                        Ok(out) => out,
                        Err(e) => {
                            let error = format!("Error while processing problem: {e}");
                            return reply::with_status(
                                reply::json(&error),
                                StatusCode::BAD_REQUEST,
                            );
                        }
                    };
                    let solution = match dmslib::nonteam::solve_naive(&graph, &config) {
                        Ok(out) => out,
                        Err(e) => {
                            let error = format!("Error while generating a solution: {e}");
                            return reply::with_status(
                                reply::json(&error),
                                StatusCode::BAD_REQUEST,
                            );
                        }
                    };
                    reply::with_status(reply::json(&solution), StatusCode::OK)
                } else {
                    // TODO: Make optimization selection configurable from UI
                    // Use optimizations by default
                    let solution = req.solve_custom_timed(
                        // NOTE: The client cannot handle sorted teams yet.
                        "BitStackStateIndexer",
                        "FilterEnergizedOnWay<PermutationalActions>",
                        "TimedActionApplier<TimeUntilEnergization>",
                    );
                    // Naive solution:
                    // let solution = req.solve_naive();
                    let solution = match solution {
                        Ok(x) => x,
                        Err(e) => {
                            let error = format!("Error while generating a solution: {e}");
                            return reply::with_status(
                                reply::json(&error),
                                StatusCode::BAD_REQUEST,
                            );
                        }
                    };
                    reply::with_status(reply::json(&solution), StatusCode::OK)
                }
            }))
        .or(warp::path!("get-graphs").and(warp::get()).map(|| {
            match list_graphs(Path::new(GRAPHS_PATH)) {
                Ok(list) => reply::with_status(reply::json(&list), StatusCode::OK),
                Err(error) => {
                    log::error!("Error while getting the graph list: {error}");
                    let error = error.to_string();
                    reply::with_status(reply::json(&error), StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }))
        .or(warp::path!("save-problem")
            .and(warp::post())
            .and(warp::body::content_length_limit(JSON_CONTENT_LIMIT))
            .and(warp::body::json())
            .map(|mut req: serde_json::Value| {
                match req.as_object_mut() {
                    Some(map) => {
                        map.remove("benchmark");
                    }
                    None => {
                        return reply::with_status(
                            "The type of request must be a JSON object.".to_string(),
                            StatusCode::BAD_REQUEST,
                        );
                    }
                }
                match save_problem(&req) {
                    Ok(_) => reply::with_status("OK".to_string(), StatusCode::OK),
                    Err(e) => reply::with_status(
                        e.to_string(),
                        if e.kind() == std::io::ErrorKind::Other {
                            StatusCode::BAD_REQUEST
                        } else {
                            StatusCode::INTERNAL_SERVER_ERROR
                        },
                    ),
                }
            }))
        .boxed()
}
