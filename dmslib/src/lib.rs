//! # Disaster Management System Library
//!
//! Common functionality for DMS server and command line interface.

use serde::{Deserialize, Serialize};

pub mod io;
pub mod policy;
pub mod teams;
pub mod types;
pub mod utils;

#[global_allocator]
static ALLOCATOR: cap::Cap<std::alloc::System> = cap::Cap::new(std::alloc::System, usize::MAX);

/// Path where graphs are stored.
/// Must end with `/`, or all subdirectory names will start with `/`.
pub const GRAPHS_PATH: &str = "../graphs/";

/// Path where the problems and experiments are stored.
pub const EXPERIMENTS_PATH: &str = "../experiments/";

/// Represents the reasons why a solution attempt might fail.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "content")] // content will be used for BadInput(String)
pub enum SolveFailure {
    BadInput(String),
    OutOfMemory { used: usize, limit: usize },
}

impl std::error::Error for SolveFailure {}

impl std::fmt::Display for SolveFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SolveFailure::BadInput(reason) => write!(f, "Bad input: {}", reason),
            SolveFailure::OutOfMemory { used, limit } => {
                write!(f, "Out of memory! Used {} of {}.", used, limit)
            }
        }
    }
}
