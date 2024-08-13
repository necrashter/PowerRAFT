use ndarray::Array1;

use crate::types::*;

/// Contains information about the distribution system.
#[derive(Clone)]
pub struct Graph {
    /// Adjacency list for branch connections.
    pub branches: Vec<Vec<BusIndex>>,
    /// True if a bus at given index is directly connected to energy resource.
    pub connected: Vec<bool>,
    /// Failure probabilities.
    pub pfs: Array1<Probability>,
}
