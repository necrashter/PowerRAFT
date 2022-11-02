//! # Disaster Management System Library
//!
//! Common functionality for DMS server and command line interface.

/// Data type for bus indices.
pub type Index = usize;
/// Data type for measuring time.
pub type Time = usize;

pub mod policy;
pub mod teams;
pub mod utils;
pub mod webclient;
