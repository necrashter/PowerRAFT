//! # Disaster Management System Library
//!
//! Common functionality for DMS server and command line interface.

use std::cell::RefCell;

use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};

pub mod dqn;
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

// Some settings are stored in thread local variables.
thread_local! {
    /// Optional random seed used by some components.
    pub static RANDOM_SEED: RefCell<Option<u64>> = RefCell::new(None);
}

/// Create a new StdRng with the seed given in [`RANDOM_SEED`] thread-local variable.
pub fn create_rng() -> StdRng {
    RANDOM_SEED.with_borrow(|seed| {
        if let Some(seed) = seed {
            StdRng::seed_from_u64(*seed)
        } else {
            StdRng::from_entropy()
        }
    })
}
