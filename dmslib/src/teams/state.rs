use super::*;
use num_derive::FromPrimitive;
use serde::{Serialize, Serializer};

/// State of a single team. Use a `Vec` to represent multiple teams.
#[derive(PartialEq, Eq, Clone, Debug, PartialOrd, Ord, Serialize)]
pub struct TeamState {
    /// Remaining time
    pub time: Time,
    /// Bus index
    pub index: BusIndex,
}

impl Default for TeamState {
    fn default() -> Self {
        TeamState {
            time: 0,
            index: BusIndex::MAX,
        }
    }
}

/// State of a single bus.
#[derive(PartialEq, Eq, Clone, Debug, PartialOrd, Ord, Copy, FromPrimitive)]
pub enum BusState {
    Unknown = 0,
    Damaged = 1,
    Energized = 2,
}

impl BusState {
    /// Get the set of bus states this state can transition to as bitmask.
    #[inline]
    fn get_transition_mask(&self) -> u8 {
        match self {
            BusState::Unknown => 7,
            BusState::Damaged => 2,
            BusState::Energized => 4,
        }
    }

    /// Check if bus state is in the given set.
    #[inline]
    fn check_mask(&self, mask: u8) -> bool {
        (1 << *self as u8) & mask != 0
    }
}

impl Default for BusState {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Struct representing a state in MDP.
#[derive(Eq, Clone, Debug)]
pub struct State {
    /// The state of each bus.
    pub buses: Vec<BusState>,
    /// The state of each team.
    pub teams: Vec<TeamState>,
}

impl State {
    /// Creates the starting state from given team configuration.
    pub fn start_state(graph: &Graph, teams: Vec<TeamState>) -> State {
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
            teams,
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

    /// Returns a vector such that the value at index i contains:
    /// 1. If the status of bus at index i is unknown,
    ///    a. the smallest j value such that bus at index i is in beta_j(s)
    ///    b. usize::MAX if there's no such j
    /// 2. 0 if the status of bus at index i is energized or damaged.
    ///
    /// For each bus, minbeta array holds the number of energizations required
    /// to energize that bus. By traversing the graph starting from immediately
    /// energizable buses, we determine minbeta values and hence unreachable buses,
    /// for which minbeta = infinity.
    #[inline]
    pub fn compute_minbeta(&self, graph: &Graph) -> Vec<BusIndex> {
        let mut minbeta: Vec<BusIndex> = self
            .buses
            .iter()
            .enumerate()
            .map(|(i, bus)| {
                if bus != &BusState::Unknown {
                    return 0;
                }
                if graph.connected[i] {
                    return 1;
                }
                for &j in graph.branches[i].iter() {
                    if self.buses[j as usize] == BusState::Energized {
                        return 1;
                    }
                }
                BusIndex::MAX
            })
            .collect();
        {
            // Determine the remaining beta values
            let mut deque: VecDeque<BusIndex> = minbeta
                .iter()
                .enumerate()
                .filter_map(|(i, &beta)| if beta == 1 { Some(i as BusIndex) } else { None })
                .collect();
            while let Some(i) = deque.pop_front() {
                let next_beta: BusIndex = minbeta[i as usize] + 1;
                for &j in graph.branches[i as usize].iter() {
                    if next_beta < minbeta[j as usize] {
                        minbeta[j as usize] = next_beta;
                        deque.push_back(j);
                    }
                }
            }
        }
        minbeta
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        let buses_len = self.buses.len();
        let teams_len = self.teams.len();
        assert_eq!(
            buses_len,
            other.buses.len(),
            "Equality is undefined for states of different systems."
        );
        assert_eq!(
            teams_len,
            other.teams.len(),
            "Equality is undefined for states of different systems."
        );
        for i in 0..buses_len {
            if self.buses[i] != other.buses[i] {
                return false;
            }
        }
        for i in 0..teams_len {
            if self.teams[i] != other.teams[i] {
                return false;
            }
        }
        true
    }
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let buses_len = self.buses.len();
        let teams_len = self.teams.len();
        assert_eq!(
            buses_len,
            other.buses.len(),
            "Ordering is undefined for states of different systems."
        );
        assert_eq!(
            teams_len,
            other.teams.len(),
            "Ordering is undefined for states of different systems."
        );
        for i in 0..buses_len {
            if self.buses[i] != other.buses[i] {
                return self.buses[i].cmp(&other.buses[i]);
            }
        }
        for i in 0..teams_len {
            if self.teams[i] != other.teams[i] {
                return self.teams[i].cmp(&other.teams[i]);
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
        for t in self.teams.iter() {
            t.time.hash(hash_state);
            t.index.hash(hash_state);
        }
    }
}

impl Serialize for BusState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BusState::Damaged => serializer.serialize_str("D"),
            BusState::Unknown => serializer.serialize_str("U"),
            BusState::Energized => serializer.serialize_str("TG"),
        }
    }
}

mod indexers;
pub use indexers::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_state_ord_test() {
        let ordered_teams = vec![
            TeamState { time: 0, index: 1 },
            TeamState { time: 0, index: 2 },
            TeamState { time: 0, index: 3 },
            TeamState { index: 10, time: 1 },
            TeamState { index: 10, time: 1 },
            TeamState { index: 10, time: 1 },
            TeamState { index: 11, time: 1 },
            TeamState { index: 11, time: 1 },
            TeamState { index: 10, time: 2 },
            TeamState { index: 11, time: 2 },
        ];

        let mut teams = vec![
            TeamState { index: 10, time: 2 },
            TeamState { time: 0, index: 3 },
            TeamState { index: 11, time: 2 },
            TeamState { index: 10, time: 1 },
            TeamState { index: 11, time: 1 },
            TeamState { time: 0, index: 1 },
            TeamState { index: 11, time: 1 },
            TeamState { index: 10, time: 1 },
            TeamState { time: 0, index: 2 },
            TeamState { index: 10, time: 1 },
        ];
        teams.sort_unstable();

        assert_eq!(ordered_teams, teams);
    }

    #[test]
    fn state_ord_test() {
        use BusState::*;

        let ordered_states = vec![
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 1 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 1 },
                    TeamState { time: 0, index: 1 },
                    TeamState { time: 0, index: 1 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Damaged, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { index: 2, time: 1 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Damaged, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Energized, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
        ];

        let mut shuffled = vec![
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 1 },
                ],
            },
            State {
                buses: vec![Energized, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Damaged, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 1 },
                    TeamState { time: 0, index: 1 },
                    TeamState { time: 0, index: 1 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Damaged, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { index: 2, time: 1 },
                    TeamState { time: 0, index: 0 },
                ],
            },
        ];
        shuffled.sort_unstable();
        assert_eq!(shuffled, ordered_states);

        let mut shuffled = vec![
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { index: 2, time: 1 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Energized, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Damaged, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 1 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Energized],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Damaged, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                    TeamState { time: 0, index: 0 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![
                    TeamState { time: 0, index: 1 },
                    TeamState { time: 0, index: 1 },
                    TeamState { time: 0, index: 1 },
                ],
            },
        ];
        shuffled.sort();
        assert_eq!(shuffled, ordered_states);
    }

    #[test]
    fn start_state_test() {
        let mut graph = Graph {
            travel_times: ndarray::arr2(&[
                [0, 1, 2, 1, 2, 2],
                [1, 0, 1, 2, 2, 2],
                [2, 1, 0, 2, 2, 1],
                [1, 2, 2, 0, 1, 2],
                [2, 2, 2, 1, 0, 1],
                [2, 2, 1, 2, 1, 0],
            ]),
            branches: vec![vec![1], vec![0, 2], vec![1], vec![4], vec![3, 5], vec![4]],
            connected: vec![true, false, false, true, false, false],
            pfs: ndarray::arr1(&[0.5, 0.5, 0.25, 0.25, 0.25, 0.25]),
            team_nodes: Array2::default((0, 0)),
        };
        assert_eq!(
            State::start_state(&graph, vec![]),
            State {
                buses: vec![
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Unknown,
                ],
                teams: vec![],
            },
        );
        // If pf is 1, it should start as damaged
        graph.pfs = ndarray::arr1(&[0.5, 0.5, 1.0, 0.25, 1.0, 0.25]);
        assert_eq!(
            State::start_state(&graph, vec![]),
            State {
                buses: vec![
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Damaged,
                    BusState::Unknown,
                    BusState::Damaged,
                    BusState::Unknown,
                ],
                teams: vec![],
            },
        );
    }
}
