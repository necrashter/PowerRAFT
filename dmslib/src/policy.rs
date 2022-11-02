//! Contains methods and utilities for policy synthesis.
use crate::Time;

use ndarray::Array1;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

/// Marker trait for all structs that represent state transitions.
pub trait Transition: Serialize {
    /// Generate a self-transition for a terminal state.
    fn terminal_transition(index: usize, cost: f64) -> Self;
    /// Generate a transition without cost with probability.
    /// In teams, this is used for the case when a bus is energizable immediately at the start.
    fn costless_transition(index: usize, p: f64) -> Self;
    /// Set the index of successor state.
    fn set_successor(&mut self, index: usize);
}

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

impl Transition for RegularTransition {
    #[inline]
    fn terminal_transition(index: usize, cost: f64) -> Self {
        Self {
            successor: index,
            p: 1.0,
            cost,
        }
    }

    #[inline]
    fn costless_transition(index: usize, p: f64) -> Self {
        Self {
            successor: index,
            p,
            cost: 0.0,
        }
    }

    #[inline]
    fn set_successor(&mut self, index: usize) {
        self.successor = index;
    }
}

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

/// A regular MDP transition with probability and cost.
pub struct TimedTransition {
    /// Index of the successor state.
    pub successor: usize,
    /// Probability of this transition.
    /// The probabilities of all transitions of an action should add up to 1.
    pub p: f64,
    /// Cost that incurs when this transition is taken.
    pub cost: f64,
    /// Passed time when this transition is taken.
    pub time: Time,
}

impl Transition for TimedTransition {
    #[inline]
    fn terminal_transition(index: usize, cost: f64) -> Self {
        Self {
            successor: index,
            p: 1.0,
            cost,
            time: 1,
        }
    }

    #[inline]
    fn costless_transition(index: usize, p: f64) -> Self {
        Self {
            successor: index,
            p,
            cost: 0.0,
            // Not sure about this one, it might be also 0.
            // TODO: Look into this after implementing timed policy synthesis.
            time: 1,
        }
    }

    #[inline]
    fn set_successor(&mut self, index: usize) {
        self.successor = index;
    }
}

impl Serialize for TimedTransition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(4))?;
        seq.serialize_element(&self.successor)?;
        seq.serialize_element(&self.p)?;
        seq.serialize_element(&self.cost)?;
        seq.serialize_element(&self.time)?;
        seq.end()
    }
}

/// Generic policy synthesizer for the given transition type.
pub trait PolicySynthesizer<TransitionType: Transition> {
    /// Synthesize a policy, an action selection strategy that minimizes the cost.
    /// Returns a pair containing values of actions and index of the optimal action in each state.
    fn synthesize_policy(
        transitions: &[Vec<Vec<TransitionType>>],
        optimization_horizon: usize,
    ) -> (Vec<Vec<f64>>, Vec<usize>);
}

/// The most basic policy synthesizer for `RegularTransition`s.
/// Uses a bottom-up approach, computing each `V_{i}` before `V_{i+1}`.
///
/// The complexity is `O(optimization_horizon * transitions)`.
pub struct NaivePolicySynthesizer;

impl PolicySynthesizer<RegularTransition> for NaivePolicySynthesizer {
    fn synthesize_policy(
        transitions: &[Vec<Vec<RegularTransition>>],
        optimization_horizon: usize,
    ) -> (Vec<Vec<f64>>, Vec<usize>) {
        assert!(
            !transitions.is_empty(),
            "States must be non-empty during policy synthesis"
        );
        let mut values: Array1<f64> = Array1::zeros(transitions.len());
        for _ in 1..optimization_horizon {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_traits_test() {
        macro_rules! test_trait_funcs {
            ($a:ty) => {{
                let transition = <$a>::terminal_transition(2, 6.0);
                assert_eq!(transition.cost, 6.0);
                assert_eq!(transition.p, 1.0);
                assert_eq!(transition.successor, 2);
                let mut transition = <$a>::costless_transition(2, 0.5);
                assert_eq!(transition.cost, 0.0);
                assert_eq!(transition.p, 0.5);
                assert_eq!(transition.successor, 2);
                transition.set_successor(20);
                assert_eq!(transition.successor, 20);
            }};
        }
        test_trait_funcs!(RegularTransition);
        test_trait_funcs!(TimedTransition);
    }

    #[test]
    fn transition_serialization() {
        let t = RegularTransition {
            successor: 2,
            p: 0.5,
            cost: 6.0,
        };

        assert_eq!(serde_json::to_string(&t).unwrap(), "[2,0.5,6.0,1]");

        let t = TimedTransition {
            successor: 2,
            p: 0.5,
            cost: 6.0,
            time: 12,
        };

        assert_eq!(serde_json::to_string(&t).unwrap(), "[2,0.5,6.0,12]");
    }

    #[test]
    fn naive_policy_test() {
        let (values, actions) = NaivePolicySynthesizer::synthesize_policy(
            &vec![
                vec![
                    vec![RegularTransition {
                        successor: 1,
                        cost: 2.0,
                        p: 1.0,
                    }],
                    vec![RegularTransition {
                        successor: 1,
                        cost: 0.5,
                        p: 1.0,
                    }],
                ],
                vec![vec![RegularTransition {
                    successor: 1,
                    cost: 1.0,
                    p: 1.0,
                }]],
            ],
            10,
        );
        assert_eq!(values, vec![vec![11.0, 9.5], vec![10.0]]);
        assert_eq!(actions, vec![1, 0]);

        let (values, actions) = NaivePolicySynthesizer::synthesize_policy(
            &vec![
                vec![vec![
                    RegularTransition {
                        successor: 1,
                        cost: 2.0,
                        p: 0.5,
                    },
                    RegularTransition {
                        successor: 1,
                        cost: 0.0,
                        p: 0.5,
                    },
                ]],
                vec![vec![RegularTransition {
                    successor: 1,
                    cost: 1.0,
                    p: 1.0,
                }]],
            ],
            10,
        );
        assert_eq!(values, vec![vec![10.0], vec![10.0]]);
        assert_eq!(actions, vec![0, 0]);
    }
}
