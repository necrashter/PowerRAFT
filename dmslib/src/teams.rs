//! Module for solving field teams restoration problem.
mod actions;
mod exploration;
pub mod state;
pub mod transitions;

use actions::*;
use exploration::*;
use state::*;
use transitions::*;

use crate::policy::*;
use crate::webclient;
use crate::{Index, Time};

use itertools::Itertools;
use ndarray::{Array1, Array2};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Represents the action of a single team with the index of the destination bus.
/// For waiting teams, this is the index of the current bus.
/// For en-route teams (continue action), this must be the index of the destination bus.
pub type TeamAction = usize;

/// Contains information about the distribution system.
#[derive(Clone)]
pub struct Graph {
    /// Travel times between each edge.
    ///
    /// All diagonal entries must be zero, i.e., distance of each edge to itself is 0.
    ///
    /// Triangle inequality is assumed by some [`ActionSet`]s.
    travel_times: Array2<Time>,
    /// Adjacency list for branch connections.
    branches: Vec<Vec<Index>>,
    /// True if a bus at given index is directly connected to energy resource.
    connected: Vec<bool>,
    /// Failure probabilities.
    pfs: Array1<f64>,
    /// The latitude and longtitude for each vertex in team graph.
    team_nodes: Array2<f64>,
}

impl Graph {
    /// Create a matrix that maps each path (i, j) in this graph to a list of buses on that path,
    /// sorted in ascending order.
    ///
    /// A bus k is on path (i, j) if w(i, k) + w(k, j) is smaller or equal to w(i, j) where w is
    /// the travel time function.
    pub fn get_components_on_way(&self) -> Array2<Vec<Index>> {
        let bus_count = self.branches.len();
        let mut on_way: Array2<Vec<Index>> = Array2::default(self.travel_times.raw_dim());
        for (((i, j), elem), &direct) in on_way.indexed_iter_mut().zip(self.travel_times.iter()) {
            if i == j {
                continue;
            }
            for k in 0..bus_count {
                if i == k || j == k {
                    continue;
                }
                let through_k = self.travel_times[[i, k]] + self.travel_times[[k, j]];
                if through_k <= direct {
                    elem.push(k);
                }
            }
        }
        on_way
    }
}

/// Represents a field teams restoration problem.
#[derive(Clone)]
pub struct Problem {
    graph: Graph,
    initial_teams: Vec<TeamState>,
}

impl webclient::Graph {
    /// Convert this graph for solving a restoration problem with teams.
    pub fn to_teams_problem(self, teams: Vec<webclient::Team>) -> Result<Problem, String> {
        let mut locations: Vec<webclient::LatLng> =
            self.nodes.iter().map(|node| node.latlng.clone()).collect();
        let pfs: Array1<f64> = self.nodes.iter().map(|node| node.pf).collect();

        for (i, team) in teams.iter().enumerate() {
            if team.index.is_none() && team.latlng.is_none() {
                return Err(format!("Team {i} has neither index nor latlng!"));
            }
        }

        for res in self.resources.iter() {
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

        let lnodes = locations.len();
        let mut travel_times = Array2::<Time>::zeros((lnodes, lnodes));

        for (i1, l1) in locations.iter().enumerate() {
            for (i2, l2) in locations.iter().enumerate().skip(i1 + 1) {
                let time = l1.distance_to(l2).ceil() as Time;
                travel_times[(i1, i2)] = time;
                travel_times[(i2, i1)] = time;
            }
        }

        let mut branches = vec![Vec::new(); self.nodes.len()];

        for branch in self.branches.iter() {
            let a = branch.nodes.0;
            let b = branch.nodes.1;
            // TODO: throw error on duplicate branch?
            branches[a].push(b);
            branches[b].push(a);
        }

        let mut connected: Vec<bool> = vec![false; self.nodes.len()];

        for x in self.external.iter() {
            connected[x.node] = true;
        }

        let mut team_nodes = Array2::<f64>::zeros((locations.len(), 2));
        for (i, location) in locations.into_iter().enumerate() {
            team_nodes[(i, 0)] = location.0;
            team_nodes[(i, 1)] = location.1;
        }

        let graph = Graph {
            travel_times,
            branches,
            connected,
            pfs,
            team_nodes,
        };

        Ok(Problem {
            graph,
            initial_teams,
        })
    }

    /// Solve a field teams restoration problem on this graph.
    pub fn solve_teams_problem(
        self,
        teams: Vec<webclient::Team>,
    ) -> Result<webclient::TeamSolution<RegularTransition>, String> {
        let problem = self.to_teams_problem(teams)?;
        let solution = solve_generic::<
            RegularTransition,
            NaiveExplorer<RegularTransition, FilterOnWay<NaiveActions>>,
            NaiveActionApplier,
            NaivePolicySynthesizer,
        >(&problem.graph, problem.initial_teams);
        Ok(solution.to_webclient(problem.graph))
    }
}

fn solve_generic<'a, TT, E, AA, PS>(graph: &'a Graph, initial_teams: Vec<TeamState>) -> Solution<TT>
where
    TT: Transition,
    E: Explorer<'a, TT>,
    AA: ActionApplier<TT>,
    PS: PolicySynthesizer<TT>,
{
    let start_time = Instant::now();
    let (states, teams, transitions) = E::explore::<AA>(graph, initial_teams);
    let generation_time: f64 = start_time.elapsed().as_secs_f64();
    let (values, policy) = PS::synthesize_policy(&transitions, 30);
    let total_time: f64 = start_time.elapsed().as_secs_f64();

    Solution {
        total_time,
        generation_time,
        states,
        teams,
        transitions,
        values,
        policy,
    }
}

/// Stores the solution for a field teams restoration [`Problem`].
pub struct Solution<T: Transition> {
    /// Total time to generate the complete solution in seconds.
    pub total_time: f64,
    /// Total time to generate the MDP without policy synthesis in seconds.
    pub generation_time: f64,

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
}

impl<T: Transition> Solution<T> {
    /// Get the minimum value of value function in the first state.
    pub fn get_min_value(&self) -> f64 {
        *self.values[0]
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    /// Convert the solution to the webclient representation together with the corresponding graph.
    pub fn to_webclient(self, graph: Graph) -> webclient::TeamSolution<T> {
        let Solution {
            total_time,
            generation_time,
            states,
            teams,
            transitions,
            values,
            policy,
        } = self;
        webclient::TeamSolution {
            total_time,
            generation_time,
            team_nodes: graph.team_nodes,
            travel_times: graph.travel_times,
            states,
            teams,
            transitions,
            values,
            policy,
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;
