use super::*;

/// Module containing compressed state indexers.
mod compressed;
pub use compressed::*;

/// A trait for indexing the states of a team-based restoration problem.
///
/// Each StateIndexer also implements an Iterator, which yields the next state to be explored.
/// If the exploration is done, then the iterator will end.
pub trait StateIndexer: Iterator<Item = (usize, State)> {
    /// New state indexer structure from graph.
    fn new(graph: &Graph) -> Self;
    /// Get the number of states.
    fn get_state_count(&self) -> usize;
    /// Get the index of given state, adding it to the hasmap when necessary.
    fn index_state(&mut self, s: State) -> usize;
    /// Deconstruct the state indexer to state space.
    fn deconstruct(self) -> Array2<BusState>;
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
            };
            Some((index, state))
        }
    }
}

impl StateIndexer for NaiveStateIndexer {
    fn new(graph: &Graph) -> Self {
        let bus_count = graph.branches.len();
        NaiveStateIndexer {
            state_count: 0,
            explored_count: 0,
            bus_states: Array2::default((0, bus_count)),
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
                self.state_to_index.insert(s, i);
                i
            }
        }
    }

    fn deconstruct(self) -> Array2<BusState> {
        self.bus_states
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use BusState::*;

    fn generic_indexer_test<T: StateIndexer>(mut indexer: T, stack_based: bool) {
        assert_eq!(indexer.get_state_count(), 0);

        let state0 = State {
            buses: vec![Unknown, Unknown, Unknown, Unknown],
        };

        assert_eq!(indexer.index_state(state0.clone()), 0);
        assert_eq!(indexer.index_state(state0.clone()), 0);
        assert_eq!(indexer.index_state(state0.clone()), 0);
        assert_eq!(indexer.get_state_count(), 1);

        let (i, s) = indexer.next().unwrap();
        assert_eq!(i, 0);
        assert_eq!(s, state0);

        assert_eq!(indexer.get_state_count(), 1);

        let state1 = State {
            buses: vec![Unknown, Unknown, Damaged, Unknown],
        };
        let state2 = State {
            buses: vec![Energized, Energized, Unknown, Unknown],
        };

        assert_eq!(indexer.index_state(state1.clone()), 1);
        assert_eq!(indexer.get_state_count(), 2);
        assert_eq!(indexer.index_state(state1.clone()), 1);
        assert_eq!(indexer.get_state_count(), 2);
        assert_eq!(indexer.index_state(state2.clone()), 2);
        assert_eq!(indexer.get_state_count(), 3);
        assert_eq!(indexer.index_state(state1.clone()), 1);
        assert_eq!(indexer.get_state_count(), 3);
        assert_eq!(indexer.index_state(state2.clone()), 2);
        assert_eq!(indexer.get_state_count(), 3);

        if stack_based {
            let (i, s) = indexer.next().unwrap();
            assert_eq!(i, 2);
            assert_eq!(s, state2);
            assert_eq!(indexer.get_state_count(), 3);

            let (i, s) = indexer.next().unwrap();
            assert_eq!(i, 1);
            assert_eq!(s, state1);
        } else {
            let (i, s) = indexer.next().unwrap();
            assert_eq!(i, 1);
            assert_eq!(s, state1);
            assert_eq!(indexer.get_state_count(), 3);

            let (i, s) = indexer.next().unwrap();
            assert_eq!(i, 2);
            assert_eq!(s, state2);
        }
        assert_eq!(indexer.get_state_count(), 3);

        assert_eq!(indexer.index_state(state0.clone()), 0);

        assert_eq!(indexer.next(), None);
        assert_eq!(indexer.get_state_count(), 3);

        let bus_states = indexer.deconstruct();
        assert_eq!(
            bus_states,
            ndarray::array![
                [Unknown, Unknown, Unknown, Unknown],
                [Unknown, Unknown, Damaged, Unknown],
                [Energized, Energized, Unknown, Unknown],
            ]
        );
    }

    #[test]
    fn bit_stack_indexer_test() {
        let bus_count = 4;
        let indexer = BitStackStateIndexer::new(bus_count);
        generic_indexer_test(indexer, true);
    }
}
