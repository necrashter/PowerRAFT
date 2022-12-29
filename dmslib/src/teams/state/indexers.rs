use super::*;
use std::collections::BTreeMap;

/// A trait for indexing the explored states of a team-based restoration problem.
pub trait StateIndexer {
    /// New state indexer structure from graph.
    fn new(bus_count: usize, team_count: usize) -> Self;
    /// Get the number of states.
    fn get_state_count(&self) -> usize;
    /// Get the index of given state, adding it to the hasmap when necessary.
    fn index_state(&mut self, s: State) -> usize;
    /// Get the state at given index.
    fn get_state(&mut self, index: usize) -> State;
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

    fn get_state(&mut self, index: usize) -> State {
        State {
            buses: self.bus_states.row(index).to_vec(),
            teams: self.team_states.row(index).to_vec(),
        }
    }

    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>) {
        (self.bus_states, self.team_states)
    }
}

/// A state indexer based on BTreeMap.
pub struct BTreeStateIndexer {
    bus_count: usize,
    team_count: usize,
    btree: BTreeMap<State, usize>,
    stack: Vec<State>,
}

impl StateIndexer for BTreeStateIndexer {
    fn new(bus_count: usize, team_count: usize) -> Self {
        BTreeStateIndexer {
            bus_count,
            team_count,
            btree: BTreeMap::new(),
            stack: Vec::new(),
        }
    }

    fn get_state_count(&self) -> usize {
        self.btree.len()
    }

    fn index_state(&mut self, s: State) -> usize {
        match self.btree.get(&s) {
            Some(i) => *i,
            None => {
                let i = self.btree.len();
                self.stack.push(s.clone());
                self.btree.insert(s, i);
                i
            }
        }
    }

    /// NOTE: INDEX IS IGNORED!
    /// A stack is maintained in which newly indexed states are added. This returns states from
    /// that stack.
    fn get_state(&mut self, _index: usize) -> State {
        self.stack.pop().expect("State stack is empty in get_state")
    }

    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>) {
        let BTreeStateIndexer {
            bus_count,
            team_count,
            btree,
            stack,
        } = self;
        if stack.len() != 0 {
            panic!("State stack is not empty in deconstruct");
        }
        drop(stack);
        let state_count = btree.len();
        let mut bus_states = Array2::default((state_count, bus_count));
        let mut team_states = Array2::default((state_count, team_count));
        for (state, i) in btree.into_iter() {
            for (x, y) in bus_states
                .row_mut(i)
                .iter_mut()
                .zip(state.buses.into_iter())
            {
                *x = y;
            }
            for (x, y) in team_states
                .row_mut(i)
                .iter_mut()
                .zip(state.teams.into_iter())
            {
                *x = y;
            }
        }
        (bus_states, team_states)
    }
}

/// State indexer that sorts the team states to eliminate permutations of equivalent team states.
pub struct SortedStateIndexer<T: StateIndexer>(T);

impl<T: StateIndexer> StateIndexer for SortedStateIndexer<T> {
    #[inline]
    fn new(bus_count: usize, team_count: usize) -> Self {
        Self(T::new(bus_count, team_count))
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
    fn get_state(&mut self, index: usize) -> State {
        self.0.get_state(index)
    }

    #[inline]
    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>) {
        self.0.deconstruct()
    }
}
