use super::*;
pub use crate::teams::state::BusState;

/// Struct representing a state in MDP.
#[derive(Eq, Clone, Debug)]
pub struct State {
    /// The state of each bus.
    pub buses: Vec<BusState>,
}

impl State {
    /// Creates the starting state from given team configuration.
    pub fn start_state(graph: &Graph) -> State {
        State {
            buses: graph
                .pfs
                .iter()
                .map(|&pf| {
                    if pf == 1.0 {
                        BusState::Damaged
                    } else {
                        BusState::Unknown
                    }
                })
                .collect_vec(),
        }
    }

    /// Cost function: the count of unenergized (damaged or unknown) buses.
    pub fn get_cost(&self) -> Cost {
        self.buses
            .iter()
            .filter(|&b| *b != BusState::Energized)
            .count() as Cost
    }

    /// Compute the transition probability from this state to another based on given
    /// failure probabilities.
    pub fn get_probability(&self, other: &State, pfs: &[Probability]) -> Probability {
        let mut p: Probability = 1.0;
        for (i, (&a, &b)) in self.buses.iter().zip(other.buses.iter()).enumerate() {
            if a != b {
                debug_assert_eq!(a, BusState::Unknown);
                debug_assert_ne!(b, BusState::Unknown);
                let pf = pfs[i];
                p *= if b == BusState::Damaged { pf } else { 1.0 - pf };
            }
        }
        p
    }

    pub fn is_terminal(&self, graph: &Graph) -> bool {
        !self.buses.iter().enumerate().any(|(i, bus)| {
            if *bus != BusState::Unknown {
                return false;
            }
            if graph.connected[i] {
                return true;
            }
            for &j in graph.branches[i].iter() {
                if self.buses[j as usize] == BusState::Energized {
                    return true;
                }
            }
            false
        })
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        let buses_len = self.buses.len();
        assert_eq!(
            buses_len,
            other.buses.len(),
            "Equality is undefined for states of different systems."
        );
        for i in 0..buses_len {
            if self.buses[i] != other.buses[i] {
                return false;
            }
        }
        true
    }
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let buses_len = self.buses.len();
        assert_eq!(
            buses_len,
            other.buses.len(),
            "Ordering is undefined for states of different systems."
        );
        for i in 0..buses_len {
            if self.buses[i] != other.buses[i] {
                return self.buses[i].cmp(&other.buses[i]);
            }
        }
        std::cmp::Ordering::Equal
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Hash is implemented for index lookup for a given state.
impl std::hash::Hash for State {
    fn hash<H: std::hash::Hasher>(&self, hash_state: &mut H) {
        // We don't hash bus/team size because it will be the same in a given HashMap
        for bus in self.buses.iter() {
            let i = match bus {
                BusState::Damaged => -1,
                BusState::Unknown => 0,
                BusState::Energized => 1,
            };
            i.hash(hash_state);
        }
    }
}

mod indexers;
pub use indexers::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_ord_test() {
        use BusState::*;

        let ordered_states = vec![
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Energized],
            },
            State {
                buses: vec![Unknown, Damaged, Unknown, Energized],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
            },
            State {
                buses: vec![Damaged, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Energized, Unknown, Unknown, Unknown],
            },
        ];

        let mut shuffled = vec![
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Energized, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Damaged, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Energized],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
            },
            State {
                buses: vec![Unknown, Damaged, Unknown, Energized],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
            },
        ];
        shuffled.sort_unstable();
        assert_eq!(shuffled, ordered_states);

        let mut shuffled = vec![
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
            },
            State {
                buses: vec![Energized, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Unknown, Damaged, Unknown, Energized],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Energized],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Damaged, Unknown, Unknown, Unknown],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
            },
        ];
        shuffled.sort();
        assert_eq!(shuffled, ordered_states);
    }

    #[test]
    fn start_state_test() {
        let mut graph = Graph {
            branches: vec![vec![1], vec![0, 2], vec![1], vec![4], vec![3, 5], vec![4]],
            connected: vec![true, false, false, true, false, false],
            pfs: ndarray::arr1(&[0.5, 0.5, 0.25, 0.25, 0.25, 0.25]),
        };
        assert_eq!(
            State::start_state(&graph),
            State {
                buses: vec![
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Unknown,
                ],
            },
        );
        // If pf is 1, it should start as damaged
        graph.pfs = ndarray::arr1(&[0.5, 0.5, 1.0, 0.25, 1.0, 0.25]);
        assert_eq!(
            State::start_state(&graph),
            State {
                buses: vec![
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Damaged,
                    BusState::Unknown,
                    BusState::Damaged,
                    BusState::Unknown,
                ],
            },
        );
    }
}
