//! Contains methods and utilities for policy synthesis.
use ndarray::Array1;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

/// Marker trait for all structs that represent state transitions.
pub trait Transition: Serialize {}

/// A regular MDP transition with probability and cost.
pub struct RegularTransition {
    /// Index of the successor state.
    pub successor: usize,
    /// Probability of this transition.
    /// The probabilities of all transitions of an action should add up to 1.
    pub p: f64,
    /// Cost that incurs when this transition is taken.
    pub cost: f64,
}

impl Transition for RegularTransition {}

impl Serialize for RegularTransition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(4))?;
        seq.serialize_element(&self.successor)?;
        seq.serialize_element(&self.p)?;
        seq.serialize_element(&self.cost)?;
        seq.serialize_element(&1)?;
        seq.end()
    }
}

/// Synthesize a policy, an array containing the index of optimal actions in each state.
/// Must be run after running `explore`, i.e., state space must not be empty.
///
/// Returns a pair containing action values and index of optimal action in each state.
pub fn synthesize_policy(
    transitions: &Vec<Vec<Vec<RegularTransition>>>,
) -> (Vec<Vec<f64>>, Vec<usize>) {
    assert!(
        !transitions.is_empty(),
        "States must be non-empty during policy synthesis"
    );
    let mut values: Array1<f64> = Array1::zeros(transitions.len());
    const OPTIMIZATION_HORIZON: usize = 30;
    for _ in 1..OPTIMIZATION_HORIZON {
        let prev_val = values;
        values = Array1::zeros(transitions.len());
        for (i, action) in transitions.iter().enumerate() {
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
    state_action_values.reserve(transitions.len());
    let mut policy: Vec<usize> = vec![0; transitions.len()];

    let prev_val = values;
    for (i, action) in transitions.iter().enumerate() {
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
