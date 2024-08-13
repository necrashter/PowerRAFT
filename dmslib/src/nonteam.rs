//! Module for solving restoration problem without teams.
mod actions;
mod exploration;
mod solve_variations;
pub mod state;
pub mod transitions;

pub use actions::*;
pub use exploration::*;
pub use solve_variations::*;
use state::*;
use transitions::*;

use crate::graph::Graph;
use crate::io;
use crate::policy::*;
use crate::types::*;
use crate::SolveFailure;

use itertools::Itertools;
use ndarray::Array2;
use std::time::Instant;

#[cfg(not(feature = "hashbrown"))]
use std::collections::HashMap;

#[cfg(feature = "hashbrown")]
use hashbrown::HashMap;

/// Represents the action of a single team with the index of the destination bus.
/// For waiting teams, this is the index of the current bus.
/// For en-route teams (continue action), this must be the index of the destination bus.
pub type TeamAction = BusIndex;

use crate::teams::Config;

pub fn solve_generic<'a, TT, E, AA, PS>(
    graph: &'a Graph,
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
        transitions,
        max_memory,
    } = E::memory_limited_explore::<AA>(graph, config.max_memory)?;

    let generation_time: f64 = start_time.elapsed().as_secs_f64();

    let auto_horizon = determine_horizon(&transitions);
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
    /// Array of actions for each state, each entry containing a list of transitions
    /// This has to be triple Vec because each state has arbitrary number of actions and each
    /// action has arbitrary number of transitions.
    pub transitions: Vec<Vec<Vec<T>>>,

    /// Value function for each action.
    pub values: Vec<Vec<Value>>,
    /// Index of optimal actions in each state.
    pub policy: Vec<ActionIndex>,
    /// Given or computed Optimization horizon.
    pub horizon: usize,
}

impl<T: Transition> Solution<T> {
    /// Get the minimum value of value function in the first state.
    pub fn get_min_value(&self) -> Value {
        get_min_value(&self.values)
    }

    /// Get [`io::BenchmarkResult`].
    pub fn get_benchmark_result(&self) -> io::BenchmarkResult {
        io::BenchmarkResult {
            total_time: self.total_time,
            generation_time: self.generation_time,
            max_memory: self.max_memory,
            states: self.transitions.len(),
            transitions: get_transition_count(&self.transitions),
            value: self.get_min_value(),
            horizon: self.horizon,
        }
    }

    /// Convert the solution to a [`io::BenchmarkResult`].
    pub fn to_benchmark_result(self) -> io::BenchmarkResult {
        self.get_benchmark_result()
    }
}

use crate::io::Array2Serializer;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

impl<T: Transition> Serialize for Solution<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(8))?;
        map.serialize_entry("totalTime", &self.total_time)?;
        map.serialize_entry("generationTime", &self.generation_time)?;

        map.serialize_entry("states", &Array2Serializer(&self.states))?;
        map.serialize_entry("transitions", &self.transitions)?;

        map.serialize_entry("values", &self.values)?;
        map.serialize_entry("policy", &self.policy)?;
        map.end()
    }
}

#[cfg(test)]
mod integration_tests;
