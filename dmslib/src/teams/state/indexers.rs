use super::*;

/// A trait for indexing the states of a team-based restoration problem.
///
/// Each StateIndexer also implements an Iterator, which yields the next state to be explored.
/// If the exploration is done, then the iterator will end.
pub trait StateIndexer: Iterator<Item = (usize, State)> {
    /// New state indexer structure from graph.
    fn new(bus_count: usize, team_count: usize) -> Self;
    /// Get the number of states.
    fn get_state_count(&self) -> usize;
    /// Get the index of given state, adding it to the hasmap when necessary.
    fn index_state(&mut self, s: State) -> usize;
    /// Deconstruct the state indexer to state space.
    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>);
}

/// A naive state indexer:
/// - New states are added to `Array2`s as indexed.
/// - HashMap is used as reverse index.
pub struct NaiveStateIndexer {
    /// Number of states.
    state_count: usize,
    /// States before this one are explored.
    /// In other words, index of the next state to be explored.
    explored_count: usize,
    /// Matrix of bus states, each state in a row.
    bus_states: Array2<BusState>,
    /// Matrix of team states, each state in a row.
    team_states: Array2<TeamState>,
    /// Reverse index
    state_to_index: HashMap<State, usize>,
}

impl Iterator for NaiveStateIndexer {
    type Item = (usize, State);

    fn next(&mut self) -> Option<Self::Item> {
        if self.explored_count >= self.state_count {
            None
        } else {
            let index = self.explored_count;
            self.explored_count += 1;
            let state = State {
                buses: self.bus_states.row(index).to_vec(),
                teams: self.team_states.row(index).to_vec(),
            };
            Some((index, state))
        }
    }
}

impl StateIndexer for NaiveStateIndexer {
    fn new(bus_count: usize, team_count: usize) -> Self {
        NaiveStateIndexer {
            state_count: 0,
            explored_count: 0,
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

    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>) {
        (self.bus_states, self.team_states)
    }
}

/// A state indexer that uses stack to keep track of states to be explored:
/// - New states are added to a stack.
/// - HashMap is used as reverse index.
/// - State `Array2`s are built by deconstructing the hashmap.
pub struct StackStateIndexer {
    bus_count: usize,
    team_count: usize,
    state_to_index: HashMap<State, usize>,
    stack: Vec<(usize, State)>,
}

impl Iterator for StackStateIndexer {
    type Item = (usize, State);

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop()
    }
}

impl StateIndexer for StackStateIndexer {
    fn new(bus_count: usize, team_count: usize) -> Self {
        StackStateIndexer {
            bus_count,
            team_count,
            state_to_index: HashMap::new(),
            stack: Vec::new(),
        }
    }

    fn get_state_count(&self) -> usize {
        self.state_to_index.len()
    }

    fn index_state(&mut self, s: State) -> usize {
        match self.state_to_index.get(&s) {
            Some(i) => *i,
            None => {
                let i = self.state_to_index.len();
                self.stack.push((i, s.clone()));
                self.state_to_index.insert(s, i);
                i
            }
        }
    }

    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>) {
        let StackStateIndexer {
            bus_count,
            team_count,
            state_to_index,
            stack,
        } = self;
        if !stack.is_empty() {
            panic!("State stack is not empty in deconstruct");
        }
        drop(stack);
        let state_count = state_to_index.len();
        let mut bus_states = Array2::default((state_count, bus_count));
        let mut team_states = Array2::default((state_count, team_count));
        for (state, i) in state_to_index.into_iter() {
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

impl<T: StateIndexer> Iterator for SortedStateIndexer<T> {
    type Item = (usize, State);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

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
    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>) {
        self.0.deconstruct()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btree_indexer_test() {
        use BusState::*;
        use TeamState::*;

        let mut indexer = StackStateIndexer::new(3, 1);

        let state0 = State {
            buses: vec![Unknown, Unknown, Unknown],
            teams: vec![OnBus(0)],
        };

        assert_eq!(indexer.index_state(state0.clone()), 0);
        assert_eq!(indexer.index_state(state0.clone()), 0);
        assert_eq!(indexer.index_state(state0.clone()), 0);

        let (i, s) = indexer.next().unwrap();
        assert_eq!(i, 0);
        assert_eq!(s, state0);

        let state1 = State {
            buses: vec![Unknown, Unknown, Damaged],
            teams: vec![OnBus(0)],
        };
        let state2 = State {
            buses: vec![Unknown, Unknown, Unknown],
            teams: vec![OnBus(1)],
        };

        assert_eq!(indexer.index_state(state1.clone()), 1);
        assert_eq!(indexer.index_state(state2.clone()), 2);
        assert_eq!(indexer.index_state(state1.clone()), 1);
        assert_eq!(indexer.index_state(state2.clone()), 2);

        let (i, s) = indexer.next().unwrap();
        assert_eq!(i, 2);
        assert_eq!(s, state2);

        let (i, s) = indexer.next().unwrap();
        assert_eq!(i, 1);
        assert_eq!(s, state1);

        assert_eq!(indexer.index_state(state0.clone()), 0);

        assert_eq!(indexer.next(), None);

        let (bus_states, team_states) = indexer.deconstruct();
        assert_eq!(
            bus_states,
            ndarray::array![
                [Unknown, Unknown, Unknown],
                [Unknown, Unknown, Damaged],
                [Unknown, Unknown, Unknown],
            ]
        );
        assert_eq!(
            team_states,
            ndarray::array![[OnBus(0)], [OnBus(0)], [OnBus(1)],]
        );
    }
}
