use super::*;

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

/// State indexer that sorts the team states to eliminate permutations of equivalent team states.
pub struct SortedStateIndexer(NaiveStateIndexer);

impl StateIndexer for SortedStateIndexer {
    #[inline]
    fn new(bus_count: usize, team_count: usize) -> Self {
        Self(NaiveStateIndexer::new(bus_count, team_count))
    }

    #[inline]
    fn get_state_count(&self) -> usize {
        self.0.get_state_count()
    }

    #[inline]
    fn index_state(&mut self, mut s: State) -> usize {
        s.teams.sort_unstable();
        self.0.index_state(s)
    }

    #[inline]
    fn get_state(&self, index: usize) -> State {
        self.0.get_state(index)
    }

    #[inline]
    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>) {
        self.0.deconstruct()
    }
}
