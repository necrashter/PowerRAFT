//! Contains methods and utilities for policy synthesis.
use crate::types::*;

use ndarray::Array1;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

/// Marker trait for all structs that represent state transitions.
pub trait Transition: Serialize {
    /// Generate a self-transition for a terminal state.
    fn terminal_transition(index: StateIndex, cost: Cost) -> Self;
    /// Generate a transition with given cost, probability and time = 1.
    fn time1_transition(index: StateIndex, cost: Cost, p: Probability) -> Self;

    /// Set the index of successor state.
    fn set_successor(&mut self, index: StateIndex);
    /// Get the index of successor state.
    fn get_successor(&self) -> StateIndex;
    /// Get the probability of this transition.
    fn get_probability(&self) -> Probability;
    /// Get the probability of this transition.
    fn get_cost(&self) -> Cost;
    /// Get time required for this transition.
    fn get_time(&self) -> Time;
}

/// A regular MDP transition with probability and cost.
#[derive(Clone, PartialEq, Debug)]
pub struct RegularTransition {
    /// Index of the successor state.
    pub successor: StateIndex,
    /// Probability of this transition.
    /// The probabilities of all transitions of an action should add up to 1.
    pub p: Probability,
    /// Cost that incurs when this transition is taken.
    pub cost: Cost,
}

#[derive(PartialEq, Clone)]
enum DfsState<T> {
    New,
    Visiting,
    Done(T),
}

impl Transition for RegularTransition {
    #[inline]
    fn terminal_transition(index: StateIndex, cost: Cost) -> Self {
        Self {
            successor: index,
            p: 1.0,
            cost,
        }
    }

    #[inline]
    fn time1_transition(index: StateIndex, cost: Cost, p: Probability) -> Self {
        Self {
            successor: index,
            p,
            cost,
        }
    }

    #[inline]
    fn set_successor(&mut self, index: StateIndex) {
        self.successor = index;
    }

    #[inline]
    fn get_successor(&self) -> StateIndex {
        self.successor
    }

    #[inline]
    fn get_probability(&self) -> Probability {
        self.p
    }

    #[inline]
    fn get_cost(&self) -> Cost {
        self.cost
    }

    #[inline]
    fn get_time(&self) -> Time {
        1
    }
}

impl RegularTransition {
    /// Convert this transition to an equivalent [`TimedTransition`] with `time = 1`.
    #[inline]
    pub fn to_timed(self) -> TimedTransition {
        let RegularTransition { successor, p, cost } = self;
        TimedTransition {
            successor,
            p,
            cost,
            time: 1,
        }
    }
}

/// Given a [`RegularTransition`] space, return the equivalent [`TimedTransition`] space with all
/// transition times equal to 1.
pub fn to_timed_transitions(
    transitions: &[Vec<Vec<RegularTransition>>],
) -> Vec<Vec<Vec<TimedTransition>>> {
    transitions
        .iter()
        .map(|state| {
            state
                .iter()
                .map(|action| {
                    action
                        .iter()
                        .map(|transition| transition.clone().to_timed())
                        .collect()
                })
                .collect()
        })
        .collect()
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
#[derive(Clone, PartialEq, Debug)]
pub struct TimedTransition {
    /// Index of the successor state.
    pub successor: StateIndex,
    /// Probability of this transition.
    /// The probabilities of all transitions of an action should add up to 1.
    pub p: Probability,
    /// Cost that incurs when this transition is taken.
    pub cost: Cost,
    /// Passed time when this transition is taken.
    pub time: Time,
}

impl Transition for TimedTransition {
    #[inline]
    fn terminal_transition(index: StateIndex, cost: Cost) -> Self {
        Self {
            successor: index,
            p: 1.0,
            cost,
            time: 1,
        }
    }

    #[inline]
    fn time1_transition(index: StateIndex, cost: Cost, p: Probability) -> Self {
        Self {
            successor: index,
            p,
            cost,
            time: 1,
        }
    }

    #[inline]
    fn set_successor(&mut self, index: StateIndex) {
        self.successor = index;
    }

    #[inline]
    fn get_successor(&self) -> StateIndex {
        self.successor
    }

    #[inline]
    fn get_probability(&self) -> Probability {
        self.p
    }

    #[inline]
    fn get_cost(&self) -> Cost {
        self.cost
    }

    #[inline]
    fn get_time(&self) -> Time {
        self.time
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

/// Run depth-first search on the transition space.
fn dfs<T: Transition>(transitions: &[Vec<Vec<T>>]) -> Vec<DfsState<usize>> {
    let mut memoization = vec![DfsState::<usize>::New; transitions.len()];

    fn visit<T: Transition>(
        index: StateIndex,
        transitions: &[Vec<Vec<T>>],
        memoization: &mut [DfsState<usize>],
    ) -> usize {
        let m = &mut memoization[index as usize];
        if let DfsState::Done(v) = m {
            return *v;
        } else if *m == DfsState::Visiting {
            panic!("MDP state graph is cyclic");
        }
        *m = DfsState::Visiting;
        let mut max_depth = 0;
        for action in transitions[index as usize].iter() {
            for t in action.iter() {
                let successor = t.get_successor();
                let time = t.get_time() as usize;
                let depth: usize = if successor == index {
                    time
                } else {
                    visit(successor, transitions, memoization) + time
                };
                max_depth = std::cmp::max(max_depth, depth);
            }
        }
        memoization[index as usize] = DfsState::Done(max_depth);
        max_depth
    }
    visit(0, transitions, &mut memoization);

    memoization
}

/// Returns 1 plus the length of the longest path starting from each state to a
/// terminal state via depth-first search.
/// For terminal states, the returned value is 1.
///
/// Panics in case of unreachable states.
pub fn longest_path_lengths<T: Transition>(transitions: &[Vec<Vec<T>>]) -> Vec<usize> {
    dfs(transitions)
        .into_iter()
        .map(|dfs_state| {
            if let DfsState::Done(depth) = dfs_state {
                depth
            } else {
                panic!("DFS failed to reach all states.");
            }
        })
        .collect()
}

/// Determine the optimization horizon from transition space.
pub fn determine_horizon<T: Transition>(transitions: &[Vec<Vec<T>>]) -> usize {
    let memoization = dfs(transitions);
    let DfsState::Done(depth) = memoization[0] else {
        unreachable!()
    };
    depth
}

/// Generic policy synthesizer for the given transition type.
pub trait PolicySynthesizer<TransitionType: Transition> {
    /// Synthesize a policy, an action selection strategy that minimizes the cost.
    /// Returns a pair containing values of actions and index of the optimal action in each state.
    fn synthesize_policy(
        transitions: &[Vec<Vec<TransitionType>>],
        horizon: usize,
    ) -> (Vec<Vec<Value>>, Vec<ActionIndex>);
}

/// The most basic policy synthesizer for `RegularTransition`s.
/// Uses a bottom-up approach, computing each `V_{i}` before `V_{i+1}`.
///
/// The complexity is `O(optimization_horizon * transitions)`.
pub struct NaivePolicySynthesizer;

impl PolicySynthesizer<RegularTransition> for NaivePolicySynthesizer {
    fn synthesize_policy(
        transitions: &[Vec<Vec<RegularTransition>>],
        horizon: usize,
    ) -> (Vec<Vec<Value>>, Vec<ActionIndex>) {
        assert!(
            !transitions.is_empty(),
            "States must be non-empty during policy synthesis"
        );
        let mut values: Array1<Value> = Array1::zeros(transitions.len());
        for _ in 1..horizon {
            let prev_val = values;
            values = Array1::zeros(transitions.len());
            for (i, action) in transitions.iter().enumerate() {
                let optimal_value: Value = action
                    .iter()
                    .map(|transitions| {
                        transitions
                            .iter()
                            .map(|t| {
                                let p = t.p as Value;
                                let cost = t.cost as Value;
                                let successor = t.successor as usize;
                                p * (cost + prev_val[successor])
                            })
                            .sum()
                    })
                    .min_by(|a: &Value, b| {
                        a.partial_cmp(b)
                            .expect("Transition values must be comparable in value iteration")
                    })
                    .expect("No actions in a state");
                values[i] = optimal_value;
            }
        }

        let mut state_action_values: Vec<Vec<Value>> = Vec::new();
        state_action_values.reserve(transitions.len());
        let mut policy: Vec<ActionIndex> = vec![0; transitions.len()];

        let prev_val = values;
        for (i, action) in transitions.iter().enumerate() {
            let action_values: Vec<Value> = action
                .iter()
                .map(|transitions| {
                    transitions
                        .iter()
                        .map(|t| {
                            let p = t.p as Value;
                            let cost = t.cost as Value;
                            let successor = t.successor as usize;
                            p * (cost + prev_val[successor])
                        })
                        .sum()
                })
                .collect();
            let optimal_action = action_values
                .iter()
                .enumerate()
                .min_by(|a: &(usize, &Value), b: &(usize, &Value)| {
                    a.1.partial_cmp(b.1)
                        .expect("Transition values must be comparable in value iteration")
                })
                .expect("No actions in a state")
                .0;
            state_action_values.push(action_values);
            policy[i] = optimal_action as ActionIndex;
        }
        (state_action_values, policy)
    }
}

/// The most basic policy synthesizer for `TimedTransition`s.
/// Uses a bottom-up approach, computing each `V_{i}` before `V_{i+1}`.
/// The complexity is `O(optimization_horizon * transitions)`.
///
/// ## Transitions with `t=0`
///
/// Transitions with zero time are handled correctly, given that all states with zero-timed
/// transitions come before others. This is always the case in field-team restoration problem,
/// where zero-timed transitions may only occur at the first state, only if there's team on
/// energizable bus.
pub struct NaiveTimedPolicySynthesizer;

impl PolicySynthesizer<TimedTransition> for NaiveTimedPolicySynthesizer {
    fn synthesize_policy(
        transitions: &[Vec<Vec<TimedTransition>>],
        horizon: usize,
    ) -> (Vec<Vec<Value>>, Vec<ActionIndex>) {
        assert!(
            !transitions.is_empty(),
            "States must be non-empty during policy synthesis"
        );
        // Special handling for first iteration: figure out maximum transition time, which will be
        // used to determine how many value functions we need to remember from previous iterations.
        let (values, max_time): (Array1<Value>, usize) = {
            let mut values = Array1::zeros(transitions.len());
            let mut max_time: usize = 0;
            for (i, action) in transitions.iter().enumerate().rev() {
                let optimal_value: Value = action
                    .iter()
                    .map(|transitions| {
                        transitions
                            .iter()
                            .map(|t| {
                                max_time = std::cmp::max(max_time, t.time as usize);
                                (t.p as Value) * (t.cost as Value)
                            })
                            .sum()
                    })
                    .min_by(|a: &Value, b| {
                        a.partial_cmp(b)
                            .expect("Transition values must be comparable in value iteration")
                    })
                    .expect("No actions in a state");
                values[i] = optimal_value;
            }
            (values, max_time)
        };
        // Array of values from previous iterations.
        // `values[0]`: current iteration, `values[1]`: previous iteration, etc.
        let mut values: Vec<Array1<Value>> = vec![values; max_time + 1];
        for iteration in 2..horizon {
            values[max_time] = Array1::zeros(transitions.len());
            values.rotate_right(1);
            for (i, action) in transitions.iter().enumerate().rev() {
                let optimal_value: Value = action
                    .iter()
                    .map(|transitions| {
                        transitions
                            .iter()
                            .map(|t| {
                                let time = t.time as usize;
                                let successor = t.successor as usize;
                                let cost =
                                    (t.cost as Value) * (std::cmp::min(time, iteration) as Value);
                                t.p * (cost + values[time][successor])
                            })
                            .sum()
                    })
                    .min_by(|a: &Value, b| {
                        a.partial_cmp(b)
                            .expect("Transition values must be comparable in value iteration")
                    })
                    .expect("No actions in a state");
                values[0][i] = optimal_value;
            }
        }

        let mut state_action_values: Vec<Vec<Value>> = vec![Vec::new(); transitions.len()];
        state_action_values.reserve(transitions.len());
        let mut policy: Vec<ActionIndex> = vec![0; transitions.len()];

        values[max_time] = Array1::zeros(transitions.len());
        values.rotate_right(1);
        for (i, action) in transitions.iter().enumerate().rev() {
            let action_values: Vec<Value> = action
                .iter()
                .map(|transitions| {
                    transitions
                        .iter()
                        .map(|t| {
                            let time = t.time as usize;
                            let successor = t.successor as usize;
                            let cost = (t.cost as Value) * (std::cmp::min(time, horizon) as Value);
                            t.p * (cost + values[time][successor])
                        })
                        .sum()
                })
                .collect();
            let (optimal_action, optimal_value) = action_values
                .iter()
                .enumerate()
                .min_by(|a: &(usize, &Value), b: &(usize, &Value)| {
                    a.1.partial_cmp(b.1)
                        .expect("Transition values must be comparable in value iteration")
                })
                .expect("No actions in a state");
            // This might be required for zero-timed transitions.
            values[0][i] = *optimal_value;
            state_action_values[i] = action_values;
            policy[i] = optimal_action as ActionIndex;
        }
        (state_action_values, policy)
    }
}

/// Get the minimum value of value function in the first state.
pub fn get_min_value(values: &[Vec<Value>]) -> Value {
    *(values[0]
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap())
}

/// Get the total number of transitions.
pub fn get_transition_count<T>(transitions: &[Vec<Vec<T>>]) -> usize {
    transitions
        .iter()
        .map(|actions| {
            actions
                .iter()
                .map(|transitions| transitions.len())
                .sum::<usize>()
        })
        .sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_traits_test() {
        macro_rules! test_trait_funcs {
            ($a:ty) => {{
                let transition = <$a>::terminal_transition(2, 6 as Cost);
                assert_eq!(transition.cost, 6 as Cost);
                assert_eq!(transition.p, 1.0);
                assert_eq!(transition.successor, 2);
                let mut transition = <$a>::time1_transition(2, 6 as Cost, 0.5);
                assert_eq!(transition.cost, 6 as Cost);
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
            cost: 6 as Cost,
        };
        let ser = serde_json::to_string(&t).unwrap();

        assert!(ser == "[2,0.5,6.0,1]" || ser == "[2,0.5,6,1]");

        let t = TimedTransition {
            successor: 2,
            p: 0.5,
            cost: 6 as Cost,
            time: 12,
        };
        let ser = serde_json::to_string(&t).unwrap();

        assert!(ser == "[2,0.5,6.0,12]" || ser == "[2,0.5,6,12]");
    }

    #[test]
    fn naive_policy_test() {
        let transitions: Vec<Vec<Vec<RegularTransition>>> = vec![
            vec![
                vec![RegularTransition {
                    successor: 1,
                    cost: 4 as Cost,
                    p: 1.0,
                }],
                vec![RegularTransition {
                    successor: 1,
                    cost: 1 as Cost,
                    p: 1.0,
                }],
            ],
            vec![vec![RegularTransition {
                successor: 1,
                cost: 2 as Cost,
                p: 1.0,
            }]],
        ];
        let (values, actions) = NaivePolicySynthesizer::synthesize_policy(&transitions, 10);
        assert_eq!(
            values,
            vec![vec![22 as Value, 19 as Value], vec![20 as Value]]
        );
        assert_eq!(actions, vec![1, 0]);
        // Equivalence to TimedTransition with time = 1
        let (values, actions) =
            NaiveTimedPolicySynthesizer::synthesize_policy(&to_timed_transitions(&transitions), 10);
        assert_eq!(
            values,
            vec![vec![22 as Value, 19 as Value], vec![20 as Value]]
        );
        assert_eq!(actions, vec![1, 0]);

        let transitions: Vec<Vec<Vec<RegularTransition>>> = vec![
            vec![vec![
                RegularTransition {
                    successor: 1,
                    cost: 2 as Cost,
                    p: 0.5,
                },
                RegularTransition {
                    successor: 1,
                    cost: 0 as Cost,
                    p: 0.5,
                },
            ]],
            vec![vec![RegularTransition {
                successor: 1,
                cost: 1 as Cost,
                p: 1.0,
            }]],
        ];
        let (values, actions) = NaivePolicySynthesizer::synthesize_policy(&transitions, 10);
        assert_eq!(values, vec![vec![10 as Value], vec![10 as Value]]);
        assert_eq!(actions, vec![0, 0]);
        // Equivalence to TimedTransition with time = 1
        let (values, actions) =
            NaiveTimedPolicySynthesizer::synthesize_policy(&to_timed_transitions(&transitions), 10);
        assert_eq!(values, vec![vec![10 as Value], vec![10 as Value]]);
        assert_eq!(actions, vec![0, 0]);
    }

    #[test]
    fn timed_policy_test() {
        let transitions: Vec<Vec<Vec<TimedTransition>>> = vec![
            vec![
                vec![TimedTransition {
                    successor: 1,
                    cost: 1 as Cost,
                    p: 1.0,
                    time: 5,
                }],
                vec![TimedTransition {
                    successor: 1,
                    cost: 2 as Cost,
                    p: 1.0,
                    time: 1,
                }],
            ],
            vec![vec![TimedTransition {
                successor: 1,
                cost: 2 as Cost,
                p: 1.0,
                time: 1,
            }]],
        ];
        assert_eq!(get_transition_count(&transitions), 3);
        let (values, actions) = NaiveTimedPolicySynthesizer::synthesize_policy(&transitions, 10);
        assert_eq!(
            values,
            vec![vec![15 as Value, 20 as Value], vec![20 as Value]]
        );
        assert_eq!(actions, vec![0, 0]);
    }

    /// Test with zero-timed transitions at the start.
    #[test]
    fn zero_timed_policy_test() {
        let transitions: Vec<Vec<Vec<TimedTransition>>> = vec![
            vec![vec![
                TimedTransition {
                    successor: 1,
                    cost: 0 as Cost,
                    p: 0.5,
                    time: 0,
                },
                TimedTransition {
                    successor: 2,
                    cost: 0 as Cost,
                    p: 0.5,
                    time: 0,
                },
            ]],
            vec![vec![TimedTransition {
                successor: 1,
                cost: 1 as Cost,
                p: 1.0,
                time: 1,
            }]],
            vec![vec![TimedTransition {
                successor: 2,
                cost: 2 as Cost,
                p: 1.0,
                time: 1,
            }]],
        ];
        assert_eq!(get_transition_count(&transitions), 4);
        let (values, actions) = NaiveTimedPolicySynthesizer::synthesize_policy(&transitions, 10);
        assert_eq!(
            values,
            vec![vec![15 as Value], vec![10 as Value], vec![20 as Value]]
        );
        assert_eq!(actions, vec![0, 0, 0]);
    }

    #[test]
    fn longest_path_lengths_simple_test() {
        let transitions: Vec<Vec<Vec<RegularTransition>>> = vec![
            vec![vec![
                RegularTransition {
                    successor: 1,
                    cost: 0 as Cost,
                    p: 0.5,
                },
                RegularTransition {
                    successor: 2,
                    cost: 0 as Cost,
                    p: 0.5,
                },
            ]],
            vec![vec![RegularTransition {
                successor: 1,
                cost: 1 as Cost,
                p: 1.0,
            }]],
            vec![vec![RegularTransition {
                successor: 2,
                cost: 2 as Cost,
                p: 1.0,
            }]],
        ];
        let depths = longest_path_lengths(&transitions);
        assert_eq!(depths, vec![2, 1, 1]);
    }

    #[test]
    #[should_panic]
    fn longest_path_lengths_unreachable_test() {
        let transitions: Vec<Vec<Vec<RegularTransition>>> = vec![
            vec![vec![
                RegularTransition {
                    successor: 1,
                    cost: 0 as Cost,
                    p: 0.5,
                },
                RegularTransition {
                    successor: 2,
                    cost: 0 as Cost,
                    p: 0.5,
                },
            ]],
            vec![vec![RegularTransition {
                successor: 1,
                cost: 1 as Cost,
                p: 1.0,
            }]],
            vec![vec![RegularTransition {
                successor: 2,
                cost: 2 as Cost,
                p: 1.0,
            }]],
            vec![vec![RegularTransition {
                successor: 3,
                cost: 2 as Cost,
                p: 1.0,
            }]],
        ];
        longest_path_lengths(&transitions);
    }
}
