use super::*;
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
        TeamState::OnBus(usize::MAX)
    }
}

/// State of a single bus.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BusState {
    Damaged = -1,
    Unknown = 0,
    Energized = 1,
}

impl Default for BusState {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Struct representing a state in MDP.
#[derive(Eq, Clone)]
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
    pub fn get_cost(&self) -> f64 {
        self.buses
            .iter()
            .filter(|&b| *b != BusState::Energized)
            .count() as f64
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
                if self.buses[j] == BusState::Energized {
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
                    if self.buses[j] == BusState::Energized {
                        return 1;
                    }
                }
                usize::MAX
            })
            .collect();
        {
            // Determine the remaining beta values
            let mut deque: VecDeque<Index> = minbeta
                .iter()
                .enumerate()
                .filter_map(|(i, &beta)| if beta == 1 { Some(i) } else { None })
                .collect();
            while let Some(i) = deque.pop_front() {
                let next_beta: Index = minbeta[i] + 1;
                for &j in graph.branches[i].iter() {
                    if next_beta < minbeta[j] {
                        minbeta[j] = next_beta;
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

/// A trait for indexing the explored states of a team-based restoration problem.
pub trait StateIndexer {
    /// New state indexer structure from graph.
    fn new(bus_count: usize, team_count: usize) -> Self;
    /// Get the number of states.
    fn get_state_count(&self) -> usize;
    /// Get the index of given state, adding it to the hasmap when necessary.
    fn index_state(&mut self, s: State) -> usize;
    /// Get the state at given index.
    fn get_state(&self, index: usize) -> State;
    /// Deconstruct the state indexer to state space.
    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>);
}

/// A naive state indexer with hashmap.
pub struct NaiveStateIndexer {
    /// Number of states.
    state_count: usize,
    /// Matrix of bus states, each state in a row.
    bus_states: Array2<BusState>,
    /// Matrix of team states, each state in a row.
    team_states: Array2<TeamState>,
    /// Reverse index
    state_to_index: HashMap<State, usize>,
}

impl StateIndexer for NaiveStateIndexer {
    fn new(bus_count: usize, team_count: usize) -> Self {
        NaiveStateIndexer {
            state_count: 0,
            bus_states: Array2::default((0, bus_count)),
            team_states: Array2::default((0, team_count)),
            state_to_index: HashMap::new(),
        }
    }

    #[inline]
    fn get_state_count(&self) -> usize {
        self.state_count
    }

    fn index_state(&mut self, s: State) -> usize {
        match self.state_to_index.get(&s) {
            Some(i) => *i,
            None => {
                let i = self.state_count;
                self.state_count += 1;
                self.bus_states
                    .push_row(ndarray::ArrayView::from(&s.buses))
                    .unwrap();
                self.team_states
                    .push_row(ndarray::ArrayView::from(&s.teams))
                    .unwrap();
                self.state_to_index.insert(s, i);
                i
            }
        }
    }

    fn get_state(&self, index: usize) -> State {
        State {
            buses: self.bus_states.row(index).to_vec(),
            teams: self.team_states.row(index).to_vec(),
        }
    }

    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>) {
        (self.bus_states, self.team_states)
    }
}

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
}
