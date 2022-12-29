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

/// An array-based state indexer to maximize memory efficiency:
/// - Arrays of bus/teams are maintained for index to state lookup.
/// - Sorted array of bus/team/index is used for reverse indexing.
///     - Binary search is used for addition and search.
///
/// Search complexity is O(logn), but insertion complexity is unfortunately O(n) due to moving
/// elements.
pub struct ArrayStateIndexer {
    /// Number of states.
    state_count: usize,
    /// States before this one are explored.
    /// In other words, index of the next state to be explored.
    explored_count: usize,
    bus_count: usize,
    team_count: usize,

    /// Matrix of bus states, each state in a row.
    bus_states: Array2<BusState>,
    /// Matrix of team states, each state in a row.
    team_states: Array2<TeamState>,

    /// Part of reverse-index array.
    reverse_buses: Vec<BusState>,
    /// Part of reverse-index array.
    reverse_teams: Vec<TeamState>,
    /// Part of reverse-index array.
    reverse_indices: Vec<usize>,
}

use std::cmp::Ordering;

impl ArrayStateIndexer {
    /// Compare the stored state at given index in the reverse lookup array with the given state.
    fn compare(&self, index: usize, state: &State) -> Ordering {
        let buses = self.reverse_buses.iter().skip(index * self.bus_count);
        for (x, y) in buses.zip(state.buses.iter()) {
            match x.cmp(y) {
                Ordering::Equal => continue,
                x => return x,
            }
        }
        let teams = self.reverse_teams.iter().skip(index * self.team_count);
        for (x, y) in teams.zip(state.teams.iter()) {
            match x.cmp(y) {
                Ordering::Equal => continue,
                x => return x,
            }
        }
        Ordering::Equal
    }

    /// Reverse index lookup.
    /// Ok(i) -> index
    /// Err(i) -> where to insert
    fn reverse_lookup(&self, state: &State) -> Result<usize, usize> {
        let mut first = 0;
        let mut count = self.state_count;
        while count > 0 {
            let step: usize = count / 2;
            let index = first + step;
            match self.compare(index, state) {
                Ordering::Less => {
                    first += step + 1;
                    count -= step + 1;
                }
                Ordering::Equal => {
                    return Ok(self.reverse_indices[index]);
                }
                Ordering::Greater => {
                    count = step;
                }
            }
        }
        Err(first)
    }

    /// Insert a new state to reverse lookup.
    fn reverse_index(&mut self, state: State, index: usize, reverse_index: usize) {
        let State { buses, teams } = state;
        let i = index * self.bus_count;
        self.reverse_buses.splice(i..i, buses.into_iter());
        let i = index * self.team_count;
        self.reverse_teams.splice(i..i, teams.into_iter());
        self.reverse_indices.insert(index, reverse_index);
    }
}

impl Iterator for ArrayStateIndexer {
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

impl StateIndexer for ArrayStateIndexer {
    fn new(bus_count: usize, team_count: usize) -> Self {
        ArrayStateIndexer {
            state_count: 0,
            explored_count: 0,
            bus_count,
            team_count,
            bus_states: Array2::default((0, bus_count)),
            team_states: Array2::default((0, team_count)),
            reverse_buses: Vec::new(),
            reverse_teams: Vec::new(),
            reverse_indices: Vec::new(),
        }
    }

    #[inline]
    fn get_state_count(&self) -> usize {
        self.state_count
    }

    fn index_state(&mut self, s: State) -> usize {
        match self.reverse_lookup(&s) {
            Ok(i) => i,
            Err(insertion) => {
                let i = self.state_count;
                self.state_count += 1;
                self.bus_states
                    .push_row(ndarray::ArrayView::from(&s.buses))
                    .unwrap();
                self.team_states
                    .push_row(ndarray::ArrayView::from(&s.teams))
                    .unwrap();
                self.reverse_index(s, insertion, i);
                i
            }
        }
    }

    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>) {
        (self.bus_states, self.team_states)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generic_indexer_test<T: StateIndexer>(mut indexer: T, stack_based: bool) {
        use BusState::*;
        use TeamState::*;

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

        if stack_based {
            let (i, s) = indexer.next().unwrap();
            assert_eq!(i, 2);
            assert_eq!(s, state2);

            let (i, s) = indexer.next().unwrap();
            assert_eq!(i, 1);
            assert_eq!(s, state1);
        } else {
            let (i, s) = indexer.next().unwrap();
            assert_eq!(i, 1);
            assert_eq!(s, state1);

            let (i, s) = indexer.next().unwrap();
            assert_eq!(i, 2);
            assert_eq!(s, state2);
        }

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

    #[test]
    fn stack_indexer_test() {
        let indexer = StackStateIndexer::new(3, 1);
        generic_indexer_test(indexer, true);
    }

    #[test]
    fn array_indexer_test() {
        let indexer = ArrayStateIndexer::new(3, 1);
        generic_indexer_test(indexer, false);
    }
}
