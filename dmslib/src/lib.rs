//! # Disaster Management System Library
//!
//! Common functionality for DMS server and command line interface.

#[global_allocator]
static ALLOCATOR: cap::Cap<std::alloc::System> = cap::Cap::new(std::alloc::System, usize::MAX);

/// Data type for bus indices.
pub type Index = usize;
/// Data type for measuring time.
pub type Time = usize;

/// Path where graphs are stored.
/// Must end with `/`, or all subdirectory names will start with `/`.
pub const GRAPHS_PATH: &str = "../graphs/";

/// Path where the problems and experiments are stored.
pub const EXPERIMENTS_PATH: &str = "../experiments/";

pub mod io;
pub mod policy;
pub mod teams;
pub mod utils;
