mod action_exploration;
mod action_iteration;
mod state;

use action_exploration::*;
use action_iteration::*;
use state::*;

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
    let mut solgen = SolutionGenerator::new(graph.branches.len());
    let generation_start_time = Instant::now();
    solgen.explore(&graph, teams_state);
    let generation_time: f64 = generation_start_time.elapsed().as_secs_f64();
    let (values, policy) = solgen.synthesize_policy();

    let mut team_nodes = Array2::<f64>::zeros((locations.len(), 2));
    for (i, location) in locations.into_iter().enumerate() {
        team_nodes[(i, 0)] = location.0;
        team_nodes[(i, 1)] = location.1;
    }

    let states: Array2<BusState> = solgen.bus_states;
    let teams: Array2<TeamState> = solgen.team_states;
    let transitions = solgen.transitions;
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

/// A struct that contains the solution to a team-based restoration problem.
/// First run `explore` and then `synthesize_policy`.
pub struct SolutionGenerator {
    /// Distribution system topology.
    // graph: Graph,
    /// Matrix of bus states, each state in a row.
    bus_states: Array2<BusState>,
    /// Matrix of team states, each state in a row.
    team_states: Array2<TeamState>,
    /// Reverse index
    state_to_index: HashMap<State, usize>,
    /// 3D vector of transitions:
    /// - `transitions[i]`: Actions of state i
    /// - `transitions[i][j]`: Transitions of action j in state i
    transitions: Vec<Vec<Vec<Transition>>>,
}

impl SolutionGenerator {
    /// New solution structure from graph.
    pub fn new(bus_count: usize) -> SolutionGenerator {
        SolutionGenerator {
            bus_states: Array2::default((0, bus_count)),
            team_states: Array2::default((0, 0)),
            state_to_index: HashMap::new(),
            transitions: Vec::new(),
        }
    }

    /// Explore the possible states starting from the given team state.
    fn explore(&mut self, graph: &Graph, teams: Vec<TeamState>) {
        self.team_states = Array2::default((0, teams.len()));
        let mut index = self.index_state(&State::start_state(graph, teams));
        let mut explorer =
            NaiveExplorer::<WaitMovingIterator<OnWayIterator<NaiveIterator>>>::setup(graph);
        explorer.explore_initial(self, graph, index);
        index += 1;
        while index < self.transitions.len() {
            explorer.explore(self, graph, index);
            index += 1;
        }
    }

    /// Get the index of given state, adding it to the hasmap when necessary.
    fn index_state(&mut self, s: &State) -> usize {
        match self.state_to_index.get(s) {
            Some(i) => *i,
            None => {
                let i = self.transitions.len();
                self.bus_states
                    .push_row(ndarray::ArrayView::from(&s.buses))
                    .unwrap();
                self.team_states
                    .push_row(ndarray::ArrayView::from(&s.teams))
                    .unwrap();
                self.transitions.push(Vec::default());
                self.state_to_index.insert(s.clone(), i);
                i
            }
        }
    }

    /// Get the state at given index.
    fn get_state(&self, index: usize) -> State {
        State {
            buses: self.bus_states.row(index).to_vec(),
            teams: self.team_states.row(index).to_vec(),
        }
    }

    /// Synthesize a policy, an array containing the index of optimal actions in each state.
    /// Must be run after running `explore`, i.e., state space must not be empty.
    ///
    /// Returns a pair containing action values and index of optimal action in each state.
    fn synthesize_policy(&mut self) -> (Vec<Vec<f64>>, Vec<usize>) {
        assert!(
            !self.transitions.is_empty(),
            "States must be non-empty during policy synthesis"
        );
        let mut values: Array1<f64> = Array1::zeros(self.transitions.len());
        const OPTIMIZATION_HORIZON: usize = 30;
        for _ in 1..OPTIMIZATION_HORIZON {
            let prev_val = values;
            values = Array1::zeros(self.transitions.len());
            for (i, action) in self.transitions.iter().enumerate() {
                let optimal_value: f64 = action
                    .iter()
                    .map(|transitions| {
                        transitions
                            .iter()
                            .map(|t| t.p * (t.cost + prev_val[t.successor]))
                            .sum()
                    })
                    .min_by(|a: &f64, b| {
                        a.partial_cmp(b)
                            .expect("Transition values must be comparable in value iteration")
                    })
                    .expect("No actions in a state");
                values[i] = optimal_value;
            }
        }

        let mut state_action_values: Vec<Vec<f64>> = Vec::new();
        state_action_values.reserve(self.transitions.len());
        let mut policy: Vec<usize> = vec![0; self.transitions.len()];

        let prev_val = values;
        for (i, action) in self.transitions.iter().enumerate() {
            let action_values: Vec<f64> = action
                .iter()
                .map(|transitions| {
                    transitions
                        .iter()
                        .map(|t| t.p * (t.cost + prev_val[t.successor]))
                        .sum()
                })
                .collect();
            let optimal_action = action_values
                .iter()
                .enumerate()
                .min_by(|a: &(usize, &f64), b: &(usize, &f64)| {
                    a.1.partial_cmp(b.1)
                        .expect("Transition values must be comparable in value iteration")
                })
                .expect("No actions in a state")
                .0;
            state_action_values.push(action_values);
            policy[i] = optimal_action;
        }
        (state_action_values, policy)
    }
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
    pub transitions: Vec<Vec<Vec<Transition>>>,

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
