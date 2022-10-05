//! # Web Client
//!
//! This module contains structs to serialize and deserialize web client representation of graphs.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BranchNodes(pub usize, pub usize);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LatLng(pub f64, pub f64);

impl LatLng {
    /// Given 2 latitude and longitude values, returns the distance in kilometers.
    /// Results in inaccuracies up to 0.5%
    ///
    /// [Source](https://stackoverflow.com/questions/19412462/getting-distance-between-two-points-based-on-latitude-longitude/)
    pub fn distance_to(&self, other: &LatLng) -> f64 {
        // approximate radius of earth in km
        const EARTH_RADIUS: f64 = 6373.0;

        let lat1 = self.0.to_radians();
        let lon1 = self.1.to_radians();
        let lat2 = other.0.to_radians();
        let lon2 = other.1.to_radians();
        let dlon = lon2 - lon1;
        let dlat = lat2 - lat1;
        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        EARTH_RADIUS * c
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Branch {
    pub nodes: BranchNodes,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExtBranch {
    pub node: usize,
    pub source: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    pub pf: f64,
    pub latlng: LatLng,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Resource {
    pub latlng: LatLng,
    /// "type" is a keyword...
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Graph {
    pub name: String,
    pub branches: Vec<Branch>,
    #[serde(rename = "externalBranches")]
    pub external: Vec<ExtBranch>,
    pub nodes: Vec<Node>,
    pub resources: Vec<Resource>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Team {
    pub index: Option<usize>,
    pub latlng: Option<LatLng>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let data = r#"
        {
            "name": "John Doe",
            "branches": [
                { "nodes": [0, 1] },
                { "nodes": [1, 2] },
                { "nodes": [2, 3] }
            ],
            "externalBranches": [
                {
                    "source": 0,
                    "node": 0,
                    "status": 1
                }
            ],
            "nodes": [
                {
                    "latlng": [ 41.01225622702989, 29.065575599670414 ],
                    "pf": 0.5,
                    "name": "Küçükçamlıca #3",
                    "status": 0
                },
                {
                    "latlng": [ 41.01225622702989, 29.065575599670414 ],
                    "pf": 0.5
                },
                {
                    "latlng": [ 41.01225622702989, 29.065575599670414 ],
                    "pf": 0.5
                }
            ],
            "resources": [
                {
                    "latlng": [
                        41.01559155019519,
                        29.092054367065433
                    ],
                    "type": null
                }
            ],
            "view": {
                "lat": 41.01303340479826,
                "lng": 29.079051017761234
            },
            "zoom": 15
        }"#;

        // Parse the string of data into serde_json::Value.
        let v: Graph = serde_json::from_str(data).unwrap();
        assert_eq!(v.name, "John Doe");

        assert_eq!(v.branches.len(), 3);

        assert_eq!(v.branches[0].nodes.0, 0);
        assert_eq!(v.branches[0].nodes.1, 1);

        assert_eq!(v.branches[1].nodes.0, 1);
        assert_eq!(v.branches[1].nodes.1, 2);

        assert_eq!(v.branches[2].nodes.0, 2);
        assert_eq!(v.branches[2].nodes.1, 3);

        assert_eq!(v.external.len(), 1);

        assert_eq!(v.nodes.len(), 3);
        assert_eq!(v.nodes[0].pf, 0.5);
        assert_eq!(v.nodes[1].pf, 0.5);
        assert_eq!(v.nodes[2].pf, 0.5);

        assert_eq!(v.nodes[0].latlng.0, v.nodes[1].latlng.0);
        assert_eq!(v.nodes[0].latlng.1, v.nodes[1].latlng.1);
    }
}
