use super::*;
use bitvec::prelude::*;
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
fn push_bits(bv: &mut BitVec, value: usize, bits: usize) {
    let start = bv.len();
    bv.resize(start + bits, false);
    bv[start..(start + bits)].store::<usize>(value);
}

struct StateCompressor {
    bus_count: usize,
    team_count: usize,
    bus_bits: usize,
    time_bits: usize,
}

impl StateCompressor {
    fn new(bus_count: usize, team_count: usize, max_time: usize) -> Self {
        StateCompressor {
            bus_count,
            team_count,
            bus_bits: get_bits_required_for(bus_count - 1),
            time_bits: get_bits_required_for(max_time),
        }
    }

    fn state_to_bits(&self, state: State) -> BitVec {
        let mut out: BitVec = BitVec::new();
        let State { buses, teams } = state;
        {
            let mut i = 0;
            let mut current: usize = 0;
            for bus in buses.into_iter() {
                let position = (i % 32) * 2;
                current |= (bus as usize) << position;
                if i != 0 && i % 32 == 0 {
                    push_bits(&mut out, current, 64);
                }
                i += 1;
            }
            if i % 32 != 0 {
                push_bits(&mut out, current, i * 2);
            }
        }
        for team in teams.into_iter() {
            match team {
                TeamState::OnBus(i) => {
                    out.push(false);
                    push_bits(&mut out, i, self.bus_bits);
                }
                TeamState::EnRoute(i, j, k) => {
                    out.push(true);
                    push_bits(&mut out, i, self.bus_bits);
                    push_bits(&mut out, j, self.bus_bits);
                    push_bits(&mut out, k, self.time_bits);
                }
            }
        }
        out
    }

    fn bits_to_state(&self, bits: BitVec) -> State {
        let mut buses: Vec<BusState> = Vec::new();
        let mut teams: Vec<TeamState> = Vec::new();
        let mut index: usize = 0;
        for _ in 0..self.bus_count {
            let bus = bits[index..(index + 2)].load::<usize>();
            buses.push(FromPrimitive::from_usize(bus).unwrap());
            index += 2;
        }
        for _ in 0..self.team_count {
            if bits[index] {
                index += 1;
                let i = bits[index..(index + self.bus_bits)].load::<usize>();
                index += self.bus_bits;
                let j = bits[index..(index + self.bus_bits)].load::<usize>();
                index += self.bus_bits;
                let k = bits[index..(index + self.time_bits)].load::<usize>();
                index += self.time_bits;
                teams.push(TeamState::EnRoute(i, j, k));
            } else {
                index += 1;
                let i = bits[index..(index + self.bus_bits)].load::<usize>();
                index += self.bus_bits;
                teams.push(TeamState::OnBus(i));
            }
        }
        State { buses, teams }
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
        BitStackStateIndexer::new(bus_count, team_count, *max_time)
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
        const IGNORE_OUTPUT: bool = false;
        if IGNORE_OUTPUT {
            let bus_states = Array2::default((1, bus_count));
            let team_states = Array2::default((1, team_count));
            (bus_states, team_states)
        } else {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use BusState::*;
    use TeamState::*;

    #[test]
    fn state_compressor_test() {
        let comp = StateCompressor::new(4, 3, 3);
        let states = vec![
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
                teams: vec![EnRoute(2, 2, 3), OnBus(1), OnBus(1)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Damaged],
                teams: vec![OnBus(0), EnRoute(1, 2, 2), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Unknown, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), OnBus(0)],
            },
            State {
                buses: vec![Unknown, Damaged, Unknown, Energized],
                teams: vec![OnBus(0), OnBus(0), EnRoute(2, 1, 3)],
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

        for state in states {
            let bits = comp.state_to_bits(state.clone());
            assert_eq!(state, comp.bits_to_state(bits));
        }
    }
}
