use super::*;
use bitvec::{macros::internal::funty::Integral, prelude::*};
use num_traits::FromPrimitive;

/// Number of bits required to represent the given number.
fn get_bits_required_for(mut number: usize) -> usize {
    let mut i = 0;
    while number != 0 {
        number >>= 1;
        i += 1;
    }
    i
}

#[inline]
fn push_bits<T: Integral>(bv: &mut BitVec, value: T, bits: usize) {
    let start = bv.len();
    bv.resize(start + bits, false);
    bv[start..(start + bits)].store::<T>(value);
}

/// Struct for compressing the states using BitVec.
pub struct StateCompressor {
    bus_count: usize,
    team_count: usize,
    bus_bits: usize,
    time_bits: usize,
}

impl StateCompressor {
    pub fn new(bus_count: usize, team_count: usize, max_time: usize) -> Self {
        StateCompressor {
            bus_count,
            team_count,
            bus_bits: get_bits_required_for(bus_count - 1),
            time_bits: get_bits_required_for(max_time),
        }
    }

    /// Convert a single state from its slices to BitVec representation.
    pub fn slice_to_bits(&self, buses: &[BusState], teams: &[TeamState]) -> BitVec {
        let mut out: BitVec = BitVec::new();
        out.resize(buses.len() * 2, false);
        {
            let mut i: usize = 0;
            for &bus in buses.iter() {
                out[i..(i + 2)].store::<usize>(bus as usize);
                i += 2;
            }
        }
        for team in teams.iter() {
            push_bits(&mut out, team.time, self.time_bits);
            push_bits(&mut out, team.index, self.bus_bits);
        }
        out
    }

    /// Convert a single state to BitVec representation.
    pub fn state_to_bits(&self, state: State) -> BitVec {
        let State { buses, teams } = state;
        self.slice_to_bits(&buses, &teams)
    }

    /// Obtain a single state from its BitVec representation.
    pub fn bits_to_state(&self, bits: BitVec) -> State {
        let mut buses: Vec<BusState> = Vec::new();
        let mut teams: Vec<TeamState> = Vec::new();
        let mut index: usize = 0;
        for _ in 0..self.bus_count {
            let bus = bits[index..(index + 2)].load::<usize>();
            buses.push(FromPrimitive::from_usize(bus).unwrap());
            index += 2;
        }
        for _ in 0..self.team_count {
            let time = bits[index..(index + self.time_bits)].load::<Time>();
            index += self.time_bits;
            let bus_index = bits[index..(index + self.bus_bits)].load::<BusIndex>();
            index += self.bus_bits;
            teams.push(TeamState {
                time,
                index: bus_index,
            });
        }
        State { buses, teams }
    }

    /// Convert states given in Array2 representation to bitvecs.
    pub fn compress(&self, buses: Array2<BusState>, teams: Array2<TeamState>) -> Vec<BitVec> {
        assert_eq!(buses.shape()[1], self.bus_count);
        assert_eq!(teams.shape()[1], self.team_count);
        assert_eq!(buses.shape()[0], teams.shape()[0]);

        let state_count = buses.shape()[0];
        let buses = buses.into_raw_vec();
        let teams = teams.into_raw_vec();

        let mut bitvecs: Vec<BitVec> = Vec::new();
        bitvecs.reserve_exact(state_count);

        let mut bus_i: usize = 0;
        let mut team_i: usize = 0;

        for _ in 0..state_count {
            let bitvec = self.slice_to_bits(
                &buses[bus_i..(bus_i + self.bus_count)],
                &teams[team_i..(team_i + self.team_count)],
            );
            bitvecs.push(bitvec);
            bus_i += self.bus_count;
            team_i += self.team_count;
        }

        bitvecs
    }

    /// Convert given bitvec representation of states to Array2 representation.
    pub fn decompress(&self, bitvecs: Vec<BitVec>) -> (Array2<BusState>, Array2<TeamState>) {
        let state_count = bitvecs.len();

        let mut bus_states: Array2<BusState> = Array2::default((state_count, self.bus_count));
        let mut team_states: Array2<TeamState> = Array2::default((state_count, self.team_count));

        for (i, bitvec) in bitvecs.into_iter().enumerate() {
            let state = self.bits_to_state(bitvec);

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

/// Same as StackStateIndexer but inner representation of states is smaller.
///
/// A state indexer that uses stack to keep track of states to be explored:
/// - New states are added to a stack.
/// - HashMap is used as reverse index.
/// - State `Array2`s are built by deconstructing the hashmap.
pub struct BitStackStateIndexer {
    bus_count: usize,
    team_count: usize,
    compressor: StateCompressor,
    state_to_index: HashMap<BitVec, usize>,
    stack: Vec<(usize, BitVec)>,
}

impl BitStackStateIndexer {
    pub fn new(bus_count: usize, team_count: usize, max_time: usize) -> Self {
        BitStackStateIndexer {
            bus_count,
            team_count,
            compressor: StateCompressor::new(bus_count, team_count, max_time),
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
    fn new(graph: &Graph, teams: &[TeamState]) -> Self {
        let bus_count = graph.branches.len();
        let team_count = teams.len();
        let max_time = graph
            .travel_times
            .iter()
            .max()
            .expect("Cannot get max travel time");
        BitStackStateIndexer::new(bus_count, team_count, *max_time as usize)
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

    fn deconstruct(self) -> (Array2<BusState>, Array2<TeamState>) {
        let BitStackStateIndexer {
            bus_count,
            team_count,
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
        let mut team_states = Array2::default((state_count, team_count));
        for (bits, i) in state_to_index.into_iter() {
            let state = compressor.bits_to_state(bits);
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

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;
    use BusState::*;

    fn get_states() -> Vec<State> {
        vec![
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
                    TeamState { index: 2, time: 3 },
                    TeamState { time: 0, index: 1 },
                    TeamState { time: 0, index: 1 },
                ],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
                teams: vec![
                    TeamState { time: 0, index: 0 },
                    TeamState { index: 2, time: 2 },
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
                    TeamState { index: 1, time: 3 },
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
        ]
    }

    #[test]
    fn state_compressor_test() {
        let comp = StateCompressor::new(4, 3, 3);

        for state in get_states() {
            let bits = comp.state_to_bits(state.clone());
            assert_eq!(state, comp.bits_to_state(bits));
        }
    }

    #[test]
    fn compress_states_test() {
        let comp = StateCompressor::new(4, 3, 3);

        let bus_states: Array2<BusState> = array![
            [Unknown, Damaged, Damaged, Damaged],
            [Unknown, Unknown, Unknown, Unknown],
            [Damaged, Damaged, Damaged, Damaged],
            [Unknown, Damaged, Energized, Damaged],
            [Energized, Energized, Unknown, Energized],
            [Energized, Energized, Energized, Energized],
        ];

        let team_states: Array2<TeamState> = array![
            [
                TeamState { time: 0, index: 2 },
                TeamState { time: 0, index: 0 },
                TeamState { index: 1, time: 3 }
            ],
            [
                TeamState { time: 0, index: 0 },
                TeamState { index: 2, time: 1 },
                TeamState { time: 0, index: 0 }
            ],
            [
                TeamState { index: 2, time: 3 },
                TeamState { time: 0, index: 1 },
                TeamState { time: 0, index: 1 }
            ],
            [
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 }
            ],
            [
                TeamState { time: 0, index: 0 },
                TeamState { index: 2, time: 1 },
                TeamState { index: 2, time: 3 }
            ],
            [
                TeamState { index: 2, time: 3 },
                TeamState { index: 2, time: 1 },
                TeamState { time: 0, index: 1 }
            ],
        ];

        let bitvecs = comp.compress(bus_states.clone(), team_states.clone());
        let (bus2, team2) = comp.decompress(bitvecs);

        assert_eq!(bus2, bus_states);
        assert_eq!(team2, team_states);
    }
}
