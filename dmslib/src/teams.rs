mod action_iteration;
mod exploration;
mod state;

use action_iteration::*;
use exploration::*;
use state::*;

use crate::policy::*;
use crate::webclient;

use itertools::Itertools;
use ndarray::{Array1, Array2, ArrayView1};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

/// Data type for bus indices.
pub type Index = usize;
/// Data type for measuring time.
pub type Time = usize;

/// Represents the actions of a single team.
/// Wait: -1 (WAIT_ACTION constant), Continue: -2 (CONTINUE_ACTION constant), Move: index of the bus.
pub type TeamAction = isize;
pub const WAIT_ACTION: isize = -1;
pub const CONTINUE_ACTION: isize = -2;

/// Contains information about the distribution system.
pub struct Graph {
    /// Travel times between each edge.
    travel_times: Array2<Time>,
    /// Adjacency list for branch connections.
    branches: Vec<Vec<Index>>,
    /// True if a bus at given index is directly connected to energy resource.
    connected: Vec<bool>,
    /// Failure probabilities.
    pfs: Array1<f64>,
}

/// Convert webclient types to teams problem.
pub fn solve(graph: webclient::Graph, teams: Vec<webclient::Team>) -> Result<Solution, String> {
    let start_time = Instant::now();

    let mut locations: Vec<webclient::LatLng> =
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

    let teams_state: Vec<TeamState> = teams
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

    let lnodes = locations.len();
    let mut travel_times = Array2::<Time>::zeros((lnodes, lnodes));

    for (i1, l1) in locations.iter().enumerate() {
        for (i2, l2) in locations.iter().enumerate().skip(i1 + 1) {
            let time = l1.distance_to(l2).ceil() as Time;
            travel_times[(i1, i2)] = time;
            travel_times[(i2, i1)] = time;
        }
    }

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

    let graph = Graph {
        travel_times,
        branches,
        connected,
        pfs,
    };
    let generation_start_time = Instant::now();
    let (states, transitions) = NaiveExplorer::<NaiveIterator>::explore(&graph, teams_state);
    let generation_time: f64 = generation_start_time.elapsed().as_secs_f64();
    let (values, policy) = synthesize_policy(&transitions);

    let mut team_nodes = Array2::<f64>::zeros((locations.len(), 2));
    for (i, location) in locations.into_iter().enumerate() {
        team_nodes[(i, 0)] = location.0;
        team_nodes[(i, 1)] = location.1;
    }

    let (states, teams) = states.deconstruct();
    let travel_times = graph.travel_times;

    let total_time: f64 = start_time.elapsed().as_secs_f64();

    Ok(Solution {
        total_time,
        generation_time,
        team_nodes,
        travel_times,
        states,
        teams,
        transitions,
        values,
        policy,
    })
}

pub struct Solution {
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
    pub transitions: Vec<Vec<Vec<RegularTransition>>>,

    /// Value function for each action.
    pub values: Vec<Vec<f64>>,
    /// Index of optimal actions in each state.
    pub policy: Vec<usize>,
}

impl Serialize for Solution {
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

#[cfg(test)]
mod tests;
