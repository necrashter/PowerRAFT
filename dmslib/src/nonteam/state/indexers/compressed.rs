use super::*;
use bitvec::prelude::*;
use num_traits::FromPrimitive;

/// Struct for compressing the states using BitVec.
pub struct StateCompressor {
    bus_count: usize,
}

impl StateCompressor {
    /// - `max_index`: Maximum `index` field in team representation.
    ///    Not necessarily equal to `bus_count - 1` because of initial location nodes.
    /// - `max_time`: Maximum possible travel time.
    pub fn new(bus_count: usize) -> Self {
        StateCompressor { bus_count }
    }

    /// Convert a single state from its slices to BitVec representation.
    pub fn slice_to_bits(&self, buses: &[BusState]) -> BitVec {
        let mut out: BitVec = BitVec::new();
        out.resize(buses.len() * 2, false);
        {
            let mut i: usize = 0;
            for &bus in buses.iter() {
                out[i..(i + 2)].store::<usize>(bus as usize);
                i += 2;
            }
        }
        out
    }

    /// Convert a single state to BitVec representation.
    pub fn state_to_bits(&self, state: State) -> BitVec {
        let State { buses } = state;
        self.slice_to_bits(&buses)
    }

    /// Obtain a single state from its BitVec representation.
    pub fn bits_to_state(&self, bits: BitVec) -> State {
        let mut buses: Vec<BusState> = Vec::new();
        let mut index: usize = 0;
        for _ in 0..self.bus_count {
            let bus = bits[index..(index + 2)].load::<usize>();
            buses.push(FromPrimitive::from_usize(bus).unwrap());
            index += 2;
        }
        State { buses }
    }

    /// Convert states given in Array2 representation to bitvecs.
    pub fn compress(&self, buses: Array2<BusState>) -> Vec<BitVec> {
        assert_eq!(buses.shape()[1], self.bus_count);

        let state_count = buses.shape()[0];
        let buses = buses.into_raw_vec();

        let mut bitvecs: Vec<BitVec> = Vec::new();
        bitvecs.reserve_exact(state_count);

        let mut bus_i: usize = 0;

        for _ in 0..state_count {
            let bitvec = self.slice_to_bits(&buses[bus_i..(bus_i + self.bus_count)]);
            bitvecs.push(bitvec);
            bus_i += self.bus_count;
        }

        bitvecs
    }

    /// Convert given bitvec representation of states to Array2 representation.
    pub fn decompress(&self, bitvecs: Vec<BitVec>) -> Array2<BusState> {
        let state_count = bitvecs.len();

        let mut bus_states: Array2<BusState> = Array2::default((state_count, self.bus_count));

        for (i, bitvec) in bitvecs.into_iter().enumerate() {
            let state = self.bits_to_state(bitvec);

            for (x, y) in bus_states
                .row_mut(i)
                .iter_mut()
                .zip(state.buses.into_iter())
            {
                *x = y;
            }
        }

        bus_states
    }
}

/// Same as StackStateIndexer but inner representation of states is smaller.
///
/// A state indexer that uses stack to keep track of states to be explored:
/// - New states are added to a stack.
/// - HashMap is used as reverse index.
/// - State `Array2`s are built by deconstructing the hashmap.
pub struct BitStackStateIndexer {
    bus_count: usize,
    compressor: StateCompressor,
    state_to_index: HashMap<BitVec, usize>,
    stack: Vec<(usize, BitVec)>,
}

impl BitStackStateIndexer {
    pub fn new(bus_count: usize) -> Self {
        BitStackStateIndexer {
            bus_count,
            compressor: StateCompressor::new(bus_count),
            state_to_index: HashMap::new(),
            stack: Vec::new(),
        }
    }
}

impl Iterator for BitStackStateIndexer {
    type Item = (usize, State);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((i, bits)) = self.stack.pop() {
            Some((i, self.compressor.bits_to_state(bits)))
        } else {
            None
        }
    }
}

impl StateIndexer for BitStackStateIndexer {
    fn new(graph: &Graph) -> Self {
        let bus_count = graph.branches.len();
        BitStackStateIndexer::new(bus_count)
    }

    fn get_state_count(&self) -> usize {
        self.state_to_index.len()
    }

    fn index_state(&mut self, s: State) -> usize {
        let bits = self.compressor.state_to_bits(s);
        match self.state_to_index.get(&bits) {
            Some(i) => *i,
            None => {
                let i = self.state_to_index.len();
                self.stack.push((i, bits.clone()));
                self.state_to_index.insert(bits, i);
                i
            }
        }
    }

    fn deconstruct(self) -> Array2<BusState> {
        let BitStackStateIndexer {
            bus_count,
            state_to_index,
            stack,
            compressor,
        } = self;
        if !stack.is_empty() {
            panic!("State stack is not empty in deconstruct");
        }
        drop(stack);

        let state_count = state_to_index.len();
        let mut bus_states = Array2::default((state_count, bus_count));
        for (bits, i) in state_to_index.into_iter() {
            let state = compressor.bits_to_state(bits);
            for (x, y) in bus_states
                .row_mut(i)
                .iter_mut()
                .zip(state.buses.into_iter())
            {
                *x = y;
            }
        }
        bus_states
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;
    use BusState::*;

    fn get_states() -> Vec<State> {
        vec![
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
        ]
    }

    #[test]
    fn state_compressor_test() {
        let bus_count = 4;
        let comp = StateCompressor::new(bus_count);

        for state in get_states() {
            let bits = comp.state_to_bits(state.clone());
            assert_eq!(state, comp.bits_to_state(bits));
        }
    }

    #[test]
    fn compress_states_test() {
        let bus_count = 4;
        let comp = StateCompressor::new(bus_count);

        let bus_states: Array2<BusState> = array![
            [Unknown, Damaged, Damaged, Damaged],
            [Unknown, Unknown, Unknown, Unknown],
            [Damaged, Damaged, Damaged, Damaged],
            [Unknown, Damaged, Energized, Damaged],
            [Energized, Energized, Unknown, Energized],
            [Energized, Energized, Energized, Energized],
        ];

        let bitvecs = comp.compress(bus_states.clone());
        let bus2 = comp.decompress(bitvecs);

        assert_eq!(bus2, bus_states);
    }

    /// Check whether the state compressor can handle the cases where the teams
    /// are located outside the bus graph, i.e., the additional nodes for the
    /// initial locations.
    #[test]
    fn compress_states_initial_node_test() {
        let bus_states: Array2<BusState> = array![
            [Unknown, Damaged, Damaged, Damaged],
            [Unknown, Unknown, Damaged, Energized],
            [Energized, Energized, Unknown, Energized],
        ];

        let bus_count = 4;
        let comp = StateCompressor::new(bus_count);

        let bitvecs = comp.compress(bus_states.clone());
        let bus2 = comp.decompress(bitvecs);

        assert_eq!(bus2, bus_states);
    }
}
