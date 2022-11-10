//! Module for solving field teams restoration problem.
mod actions;
mod exploration;
mod solve_variations;
pub mod state;
pub mod transitions;

use actions::*;
use exploration::*;
pub use solve_variations::*;
use state::*;
use transitions::*;

use crate::io;
use crate::policy::*;
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
    pub travel_times: Array2<Time>,
    /// Adjacency list for branch connections.
    pub branches: Vec<Vec<Index>>,
    /// True if a bus at given index is directly connected to energy resource.
    pub connected: Vec<bool>,
    /// Failure probabilities.
    pub pfs: Array1<f64>,
    /// The latitude and longtitude for each vertex in team graph.
    pub team_nodes: Array2<f64>,
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
    pub graph: Graph,
    pub initial_teams: Vec<TeamState>,
}

impl io::Graph {
    /// Convert this graph for solving a restoration problem with teams.
    pub fn to_teams_problem(self, teams: Vec<io::Team>) -> Result<Problem, String> {
        let team_problem = crate::io::TeamProblem {
            name: None,
            graph: self,
            teams,
            time_func: io::TimeFunc::default(),
        };

        team_problem.prepare()
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

    /// Convert the solution to the io representation together with the corresponding graph.
    pub fn to_webclient(self, graph: Graph) -> io::TeamSolution<T> {
        let Solution {
            total_time,
            generation_time,
            states,
            teams,
            transitions,
            values,
            policy,
        } = self;
        io::TeamSolution {
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

    /// Convert the solution to a [`io::BenchmarkResult`].
    pub fn to_benchmark_result(self) -> io::BenchmarkResult {
        io::BenchmarkResult {
            total_time: self.total_time,
            generation_time: self.generation_time,
            states: self.transitions.len(),
            value: self.get_min_value(),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;
