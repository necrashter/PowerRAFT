//! Input output module.
//!
//! Contains structs to serialize and deserialize various representation of graphs.
use crate::policy::*;
use crate::teams::state::{BusState, TeamState};
use crate::Time;

use ndarray::{Array1, Array2, ArrayView1};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize, Serializer};

mod experiments;
pub mod fs;
pub use experiments::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct BranchNodes(pub usize, pub usize);

/// Holds latitude and longitude values as a tuple.
/// Serialized to JSON as an array of length 2.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LatLng(pub f64, pub f64);

/// Holds latitude and longtitude values of `view` field in graphs.
/// Unlike [`LatLng`], this one serializes to a JSON Object with `lat` and `lng` fields.
#[derive(Serialize, Deserialize, Debug)]
pub struct View {
    lat: f32,
    lng: f32,
}

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

/// JSON representation of a distribution system graph.
#[derive(Serialize, Deserialize, Debug)]
pub struct Graph {
    pub name: String,
    pub branches: Vec<Branch>,
    #[serde(rename = "externalBranches")]
    pub external: Vec<ExtBranch>,
    pub nodes: Vec<Node>,
    pub resources: Vec<Resource>,
}

/// Summarized information about a distribution system [`Graph`].
#[derive(Serialize, Deserialize, Debug)]
pub struct GraphEntry {
    pub filename: String,
    pub name: String,
    #[serde(rename = "solutionFile")]
    pub solution_file: Option<String>,
    pub view: View,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Team {
    pub index: Option<usize>,
    pub latlng: Option<LatLng>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum TimeFunc {
    /// Calculate "as the crow flies" distance between two points, multiply and/or divide it with
    /// the given factor(s), and round it up (to avoid 0) to find travel times.
    DirectDistance {
        multiplier: Option<f64>,
        divider: Option<f64>,
    },
    /// Use a constant value to build travel time matrix (except for diagonal entries).
    Constant { constant: Time },
}

impl TimeFunc {
    /// Get distance between two points according to this function.
    pub fn get_distance(&self, a: &LatLng, b: &LatLng) -> Time {
        match self {
            TimeFunc::DirectDistance {
                multiplier,
                divider,
            } => {
                let mut mul = multiplier.unwrap_or(1.0);
                if let Some(divider) = divider {
                    mul /= divider;
                }
                (a.distance_to(b) * mul).ceil() as Time
            }
            TimeFunc::Constant { constant } => *constant,
        }
    }

    /// Get the travel time matrix for the given locations according to this function.
    pub fn get_travel_times(&self, locations: &Vec<LatLng>) -> Array2<Time> {
        let lnodes = locations.len();
        let mut travel_times = Array2::<Time>::zeros((lnodes, lnodes));

        match self {
            TimeFunc::DirectDistance {
                multiplier,
                divider,
            } => {
                let mut mul = multiplier.unwrap_or(1.0);
                if let Some(divider) = divider {
                    mul /= divider;
                }
                for (i1, l1) in locations.iter().enumerate() {
                    for (i2, l2) in locations.iter().enumerate().skip(i1 + 1) {
                        let time = (l1.distance_to(l2) * mul).ceil() as Time;
                        travel_times[(i1, i2)] = time;
                        travel_times[(i2, i1)] = time;
                    }
                }
            }
            TimeFunc::Constant { constant } => {
                let mut travel_times = Array2::<Time>::from_elem((lnodes, lnodes), *constant);

                for i in 0..lnodes {
                    travel_times[(i, i)] = 0;
                }
            }
        };

        travel_times
    }
}

impl Default for TimeFunc {
    fn default() -> Self {
        Self::DirectDistance {
            multiplier: None,
            divider: None,
        }
    }
}

/// Represents a field teams restoration problem.
#[derive(Serialize, Deserialize, Debug)]
pub struct TeamProblem {
    pub name: Option<String>,
    pub graph: Graph,
    pub teams: Vec<Team>,
    /// Optimization horizon for policy synthesis.
    /// Use `None` to automatically determine it based on transitions.
    pub horizon: Option<usize>,
    /// Travel time function.
    #[serde(default, rename = "timeFunction")]
    pub time_func: TimeFunc,
}

impl TeamProblem {
    /// Get the distance matrix for the system components + any additional starting positions for
    /// the teams.
    pub fn get_distances(&self) -> Result<Array2<f64>, String> {
        let mut locations: Vec<LatLng> = self
            .graph
            .nodes
            .iter()
            .map(|node| node.latlng.clone())
            .collect();

        for (i, team) in self.teams.iter().enumerate() {
            if let Some(latlng) = &team.latlng {
                locations.push(latlng.clone());
            } else if team.index.is_none() {
                return Err(format!("Team {i} has neither index nor latlng!"));
            }
        }

        let lnodes = locations.len();
        let mut distances = Array2::<f64>::zeros((lnodes, lnodes));

        for (i1, l1) in locations.iter().enumerate() {
            for (i2, l2) in locations.iter().enumerate().skip(i1 + 1) {
                let distance = l1.distance_to(l2);
                distances[(i1, i2)] = distance;
                distances[(i2, i1)] = distance;
            }
        }

        Ok(distances)
    }

    /// Prepare this problem before solving.
    /// - Add nodes for initial team positions.
    /// - Compute travel times matrix.
    /// - ...and so on.
    pub fn prepare(self) -> Result<crate::teams::Problem, String> {
        let TeamProblem {
            name: _,
            graph,
            teams,
            horizon,
            time_func,
        } = self;

        let mut locations: Vec<LatLng> =
            graph.nodes.iter().map(|node| node.latlng.clone()).collect();
        let pfs: Array1<f64> = graph.nodes.iter().map(|node| node.pf).collect();

        for (i, team) in teams.iter().enumerate() {
            if team.index.is_none() && team.latlng.is_none() {
                return Err(format!("Team {i} has neither index nor latlng!"));
            }
        }

        for res in graph.resources.iter() {
            if res.kind.is_some() {
                return Err(String::from(
                    "Only transmission grid is supported for teams!",
                ));
            }
        }

        let initial_teams: Vec<TeamState> = teams
            .into_iter()
            .map(|t| {
                if let Some(i) = t.index {
                    TeamState::OnBus(i)
                } else {
                    let i = locations.len();
                    // We did error checking above
                    locations.push(t.latlng.as_ref().unwrap().clone());
                    TeamState::OnBus(i)
                }
            })
            .collect();

        let travel_times = time_func.get_travel_times(&locations);

        let mut branches = vec![Vec::new(); graph.nodes.len()];

        for branch in graph.branches.iter() {
            let a = branch.nodes.0;
            let b = branch.nodes.1;
            // TODO: throw error on duplicate branch?
            branches[a].push(b);
            branches[b].push(a);
        }

        let mut connected: Vec<bool> = vec![false; graph.nodes.len()];

        for x in graph.external.iter() {
            connected[x.node] = true;
        }

        let mut team_nodes = Array2::<f64>::zeros((locations.len(), 2));
        for (i, location) in locations.into_iter().enumerate() {
            team_nodes[(i, 0)] = location.0;
            team_nodes[(i, 1)] = location.1;
        }

        let graph = crate::teams::Graph {
            travel_times,
            branches,
            connected,
            pfs,
            team_nodes,
        };

        Ok(crate::teams::Problem {
            graph,
            initial_teams,
            horizon,
        })
    }

    /// Solve this field teams restoration problem without any optimizations and return a
    /// [`TeamSolution`] on success.
    pub fn solve_naive(self) -> Result<TeamSolution<RegularTransition>, String> {
        let problem = self.prepare()?;
        let solution =
            crate::teams::solve_naive(&problem.graph, problem.initial_teams, problem.horizon);
        Ok(solution.to_webclient(problem.graph))
    }

    /// Solve the field-teams restoration problem with [`RegularTransition`]s (classic MDP
    /// transitions without time) and the given action set class.
    ///
    /// Returns a [`TeamSolution`] on success.
    pub fn solve_custom_regular(
        self,
        action_set: &str,
    ) -> Result<TeamSolution<RegularTransition>, String> {
        let problem = self.prepare()?;
        let solution = crate::teams::solve_custom_regular(
            &problem.graph,
            problem.initial_teams,
            problem.horizon,
            action_set,
        )?;
        Ok(solution.to_webclient(problem.graph))
    }

    /// Solve the field-teams restoration problem with [`TimedTransition`]s and the given:
    /// - action applier class (variations of `TimedActionApplier<T>` where `T` determines time)
    /// - action set class
    ///
    /// Returns a [`TeamSolution`] on success.
    pub fn solve_custom_timed(
        self,
        action_set: &str,
        action_applier: &str,
    ) -> Result<TeamSolution<TimedTransition>, String> {
        let problem = self.prepare()?;
        let solution = crate::teams::solve_custom_timed(
            &problem.graph,
            problem.initial_teams,
            problem.horizon,
            action_set,
            action_applier,
        )?;
        Ok(solution.to_webclient(problem.graph))
    }

    /// Solve the field-teams restoration problem with the given:
    /// - action applier class
    /// - action set class
    ///
    /// Returns a [`BenchmarkResult`] on success.
    pub fn benchmark_custom(
        self,
        action_set: &str,
        action_applier: &str,
    ) -> Result<BenchmarkResult, String> {
        let problem = self.prepare()?;
        let solution = crate::teams::benchmark_custom(
            &problem.graph,
            problem.initial_teams,
            problem.horizon,
            action_set,
            action_applier,
        )?;
        Ok(solution)
    }

    /// Run all optimization combination possibilities on this field-teams restoration problem.
    pub fn benchmark_all(self) -> Result<Vec<OptimizationBenchmarkResult>, String> {
        let problem = self.prepare()?;
        Ok(crate::teams::benchmark_all(
            &problem.graph,
            problem.initial_teams,
            problem.horizon,
        ))
    }
}

/// Parses a field-teams distribution system restoration problem from JSON.
/// Takes input by reference and clones the fields.
pub fn parse_teams_problem(req: &serde_json::Value) -> Result<(Graph, Vec<Team>), String> {
    let graph: Graph = if let Some(field) = req.get("graph") {
        match serde_json::from_value(field.clone()) {
            Ok(v) => v,
            Err(e) => {
                return Err(format!("Failed to parse graph: {e}"));
            }
        }
    } else {
        return Err("No graph is given".to_string());
    };
    let teams: Vec<Team> = if let Some(field) = req.get("teams") {
        match serde_json::from_value(field.clone()) {
            Ok(v) => v,
            Err(e) => {
                return Err(format!("Failed to parse teams: {e}"));
            }
        }
    } else {
        return Err("No team info is given".to_string());
    };
    Ok((graph, teams))
}

/// This struct will be the response to a client's request to solve a field teams restoration
/// problem.
pub struct TeamSolution<T: Transition> {
    /// Total time to generate the complete solution in seconds.
    pub total_time: f64,
    /// Total time to generate the MDP without policy synthesis in seconds.
    pub generation_time: f64,

    /// Latitude and longtitude values of vertices in team graph.
    pub team_nodes: Array2<f64>,
    /// Travel time between each node
    pub travel_times: Array2<Time>,

    /// Array of bus states.
    pub states: Array2<BusState>,
    /// Array of team states.
    pub teams: Array2<TeamState>,
    /// Array of actions for each state, each entry containing a list of transitions
    /// This has to be triple Vec because each state has arbitrary number of actions and each
    /// action has arbitrary number of transitions.
    pub transitions: Vec<Vec<Vec<T>>>,

    /// Value function for each action.
    pub values: Vec<Vec<f64>>,
    /// Index of optimal actions in each state.
    pub policy: Vec<usize>,
    /// Given or computed Optimization horizon.
    pub horizon: usize,
}

impl<T: Transition> Serialize for TeamSolution<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(8))?;
        map.serialize_entry("totalTime", &self.total_time)?;
        map.serialize_entry("generationTime", &self.generation_time)?;

        map.serialize_entry("teamNodes", &Array2Serializer(&self.team_nodes))?;
        map.serialize_entry("travelTimes", &Array2Serializer(&self.travel_times))?;

        map.serialize_entry("states", &Array2Serializer(&self.states))?;
        map.serialize_entry("teams", &Array2Serializer(&self.teams))?;
        map.serialize_entry("transitions", &self.transitions)?;

        map.serialize_entry("values", &self.values)?;
        map.serialize_entry("policy", &self.policy)?;
        map.end()
    }
}

/// Private helper for 2D array serialization.
/// Array is serialized as list of lists.
struct Array2Serializer<'a, T>(&'a Array2<T>);

/// Private helper for 2D array serialization.
/// This is a row in array.
struct ArrayRowSerializer<'a, T>(ArrayView1<'a, T>);

impl<'a, T: Serialize> Serialize for Array2Serializer<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.shape()[0]))?;
        for row in self.0.rows() {
            seq.serialize_element(&ArrayRowSerializer(row))?;
        }
        seq.end()
    }
}

impl<'a, T: Serialize> Serialize for ArrayRowSerializer<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for i in self.0.iter() {
            seq.serialize_element(i)?;
        }
        seq.end()
    }
}

/// Simplified solution struct for storing benchmark-related data.
#[derive(Serialize, Debug)]
pub struct BenchmarkResult {
    /// Total time to generate the complete solution in seconds.
    pub total_time: f64,
    /// Total time to generate the MDP without policy synthesis in seconds.
    pub generation_time: f64,
    /// Number of states.
    pub states: usize,
    /// Minimum value in the initial state.
    pub value: f64,
    /// Given or computed Optimization horizon.
    pub horizon: usize,
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
