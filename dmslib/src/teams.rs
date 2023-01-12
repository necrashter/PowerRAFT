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
use crate::SolveFailure;
use crate::{Index, Time};

use itertools::Itertools;
use ndarray::{Array1, Array2};
use std::collections::VecDeque;
use std::time::Instant;

#[cfg(not(feature = "hashbrown"))]
use std::collections::HashMap;

#[cfg(feature = "hashbrown")]
use hashbrown::HashMap;

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
    pub fn to_teams_problem(
        self,
        teams: Vec<io::Team>,
        horizon: Option<usize>,
    ) -> Result<(Problem, Config), SolveFailure> {
        let team_problem = crate::io::TeamProblem {
            name: None,
            graph: self,
            teams,
            pfo: None,
            horizon,
            time_func: io::TimeFunc::default(),
        };

        team_problem.prepare()
    }
}

/// Configuration struct for teams problem.
pub struct Config {
    /// State exploration will be cancelled if its memory usage exceeds this limit.
    /// [`SolveFailure::OutOfMemory`] will be returned.
    pub max_memory: usize,
    /// Optimization horizon for policy synthesis.
    /// Use `None` to automatically determine it based on transitions.
    /// `Some(value)` allows setting the optimization horizon manually instead of determining it
    /// automatically from state space.
    pub horizon: Option<usize>,
}

impl Config {
    /// Build a new config struct with default settings.
    pub const fn new() -> Config {
        Config {
            // TODO: Make this adjustable without recompiling
            max_memory: 14_400_000_000,
            horizon: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}

fn solve_generic<'a, TT, E, AA, PS>(
    graph: &'a Graph,
    initial_teams: Vec<TeamState>,
    config: &Config,
) -> Result<Solution<TT>, SolveFailure>
where
    TT: Transition,
    E: Explorer<'a, TT>,
    AA: ActionApplier<TT>,
    PS: PolicySynthesizer<TT>,
{
    let start_time = Instant::now();

    let ExploreResult {
        bus_states,
        team_states,
        transitions,
        max_memory,
    } = E::memory_limited_explore::<AA>(graph, initial_teams, config.max_memory)?;

    let generation_time: f64 = start_time.elapsed().as_secs_f64();

    let auto_horizon = TT::determine_horizon(&transitions);
    log::info!("Automatically determined horizon: {auto_horizon}");
    let horizon = if let Some(v) = config.horizon {
        if auto_horizon > v {
            log::warn!("Given horizon ({v}) is smaller than determined ({auto_horizon})");
        }
        v
    } else {
        auto_horizon
    };
    let (values, policy) = PS::synthesize_policy(&transitions, horizon);

    let total_time: f64 = start_time.elapsed().as_secs_f64();

    Ok(Solution {
        total_time,
        generation_time,
        max_memory,
        states: bus_states,
        teams: team_states,
        transitions,
        values,
        policy,
        horizon,
    })
}

/// Stores the solution for a field teams restoration [`Problem`].
pub struct Solution<T: Transition> {
    /// Total time to generate the complete solution in seconds.
    pub total_time: f64,
    /// Total time to generate the MDP without policy synthesis in seconds.
    pub generation_time: f64,
    /// Maximum memory usage in bytes.
    pub max_memory: usize,

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

pub trait GraphRefOrVal {
    fn get_info(self) -> (Array2<f64>, Array2<Time>);
}

impl GraphRefOrVal for Graph {
    fn get_info(self) -> (Array2<f64>, Array2<Time>) {
        (self.team_nodes, self.travel_times)
    }
}

impl GraphRefOrVal for &Graph {
    fn get_info(self) -> (Array2<f64>, Array2<Time>) {
        (self.team_nodes.clone(), self.travel_times.clone())
    }
}

/// Get the minimum value of value function in the first state.
pub fn get_min_value(values: &[Vec<f64>]) -> f64 {
    *(values[0]
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap())
}

impl<T: Transition> Solution<T> {
    /// Get the minimum value of value function in the first state.
    pub fn get_min_value(&self) -> f64 {
        get_min_value(&self.values)
    }

    /// Convert the solution to the io representation together with the corresponding graph.
    ///
    /// Graph can be passed by value or reference.
    pub fn into_io<G: GraphRefOrVal>(self, graph: G) -> io::TeamSolution<T> {
        let Solution {
            total_time,
            generation_time,
            max_memory,
            states,
            teams,
            transitions,
            values,
            policy,
            horizon,
        } = self;
        let (team_nodes, travel_times) = graph.get_info();
        io::TeamSolution {
            total_time,
            generation_time,
            max_memory,
            team_nodes,
            travel_times,
            states,
            teams,
            transitions,
            values,
            policy,
            horizon,
        }
    }

    /// Get [`io::BenchmarkResult`].
    pub fn get_benchmark_result(&self) -> io::BenchmarkResult {
        io::BenchmarkResult {
            total_time: self.total_time,
            generation_time: self.generation_time,
            max_memory: self.max_memory,
            states: self.transitions.len(),
            value: self.get_min_value(),
            horizon: self.horizon,
        }
    }

    /// Convert the solution to a [`io::BenchmarkResult`].
    pub fn to_benchmark_result(self) -> io::BenchmarkResult {
        self.get_benchmark_result()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;
