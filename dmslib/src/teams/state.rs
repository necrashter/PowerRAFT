use super::*;
use num_derive::FromPrimitive;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

/// State of a single team. Use a `Vec` to represent multiple teams.
#[derive(PartialEq, Eq, Clone, Debug, PartialOrd, Ord)]
pub enum TeamState {
    OnBus(Index),
    EnRoute(Index, Index, Time),
}

impl Default for TeamState {
    fn default() -> Self {
        TeamState::OnBus(Index::MAX)
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
            buses: vec![BusState::Unknown; graph.connected.len()],
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
    pub fn compute_minbeta(&self, graph: &Graph) -> Vec<Index> {
        let mut minbeta: Vec<Index> = self
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
                Index::MAX
            })
            .collect();
        {
            // Determine the remaining beta values
            let mut deque: VecDeque<Index> = minbeta
                .iter()
                .enumerate()
                .filter_map(|(i, &beta)| if beta == 1 { Some(i as Index) } else { None })
                .collect();
            while let Some(i) = deque.pop_front() {
                let next_beta: Index = minbeta[i as usize] + 1;
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
            match t {
                TeamState::OnBus(i) => {
                    0.hash(hash_state);
                    i.hash(hash_state);
                }
                TeamState::EnRoute(i, j, k) => {
                    1.hash(hash_state);
                    i.hash(hash_state);
                    j.hash(hash_state);
                    k.hash(hash_state);
                }
            }
        }
    }
}

impl Serialize for TeamState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TeamState::OnBus(a) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("node", a)?;
                map.end()
            }
            TeamState::EnRoute(a, b, t) => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("node", a)?;
                map.serialize_entry("target", b)?;
                map.serialize_entry("time", t)?;
                // NOTE: An older version put travel time here.
                // Now we put it in a separate field in output.
                // See travel_times matrix in io::TeamSolution.
                map.end()
            }
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
            TeamState::OnBus(1),
            TeamState::OnBus(2),
            TeamState::OnBus(3),
            TeamState::EnRoute(1, 10, 1),
            TeamState::EnRoute(1, 10, 2),
            TeamState::EnRoute(2, 10, 1),
            TeamState::EnRoute(2, 11, 1),
            TeamState::EnRoute(3, 10, 1),
            TeamState::EnRoute(3, 11, 1),
            TeamState::EnRoute(3, 11, 2),
        ];

        let mut teams = vec![
            TeamState::EnRoute(1, 10, 2),
            TeamState::OnBus(3),
            TeamState::EnRoute(3, 11, 2),
            TeamState::EnRoute(1, 10, 1),
            TeamState::EnRoute(3, 11, 1),
            TeamState::OnBus(1),
            TeamState::EnRoute(2, 11, 1),
            TeamState::EnRoute(2, 10, 1),
            TeamState::OnBus(2),
            TeamState::EnRoute(3, 10, 1),
        ];
        teams.sort_unstable();

        assert_eq!(ordered_teams, teams);
    }

    #[test]
    fn state_ord_test() {
        use BusState::*;
        use TeamState::*;

        let ordered_states = vec![
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(1)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![OnBus(1), OnBus(1), OnBus(1)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Damaged, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![OnBus(0), EnRoute(0, 2, 1), OnBus(0)],
            },
            State {
                buses: vec![Damaged, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Energized, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
        ];

        let mut shuffled = vec![
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(1)],
            },
            State {
                buses: vec![Energized, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Damaged, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![OnBus(1), OnBus(1), OnBus(1)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Damaged, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![OnBus(0), EnRoute(0, 2, 1), OnBus(0)],
            },
        ];
        shuffled.sort_unstable();
        assert_eq!(shuffled, ordered_states);

        let mut shuffled = vec![
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Energized, Unknown, Energized],
                teams: vec![OnBus(0), EnRoute(0, 2, 1), OnBus(0)],
            },
            State {
                buses: vec![Energized, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Damaged, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(1)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Damaged, Unknown, Unknown, Unknown],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Unknown],
                teams: vec![OnBus(1), OnBus(1), OnBus(1)],
            },
        ];
        shuffled.sort();
        assert_eq!(shuffled, ordered_states);
    }
}
