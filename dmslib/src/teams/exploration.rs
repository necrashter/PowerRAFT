use crate::ALLOCATOR;

use super::*;

pub struct ExploreResult<TT: Transition> {
    pub bus_states: Array2<BusState>,
    pub team_states: Array2<TeamState>,
    pub transitions: Vec<Vec<Vec<TT>>>,
    pub max_memory: usize,
}

/// Generic trait for the functions that explore the actions of a given state.
pub trait Explorer<'a, TT: Transition> {
    /// Explore the possible states starting from the given team state.
    fn explore<AA: ActionApplier<TT>>(
        graph: &'a Graph,
        teams: Vec<TeamState>,
    ) -> ExploreResult<TT> {
        Self::memory_limited_explore::<AA>(graph, teams, usize::MAX).unwrap()
    }

    /// Explore the possible states starting from the given team state.
    ///
    /// When the memory usage reported by global allocator exceeds the limit,
    /// [`SolveFailure::OutOfMemory`] will be returned;
    fn memory_limited_explore<AA: ActionApplier<TT>>(
        graph: &'a Graph,
        teams: Vec<TeamState>,
        memory_limit: usize,
    ) -> Result<ExploreResult<TT>, SolveFailure>;
}

/// Naive action explorer.
pub struct NaiveExplorer<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> {
    /// Action iterator.
    iterator: AI,
    /// Reference to a graph.
    graph: &'a Graph,
    /// State indexer.
    states: SI,
    /// 3D vector of transitions:
    /// - `transitions[i]`: Actions of state i
    /// - `transitions[i][j]`: Transitions of action j in state i
    transitions: Vec<Vec<Vec<TT>>>,
}

impl<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> NaiveExplorer<'a, TT, AI, SI> {
    /// Explore the actions and transitions of a state at the given index in the state indexer.
    #[inline]
    fn explore_state<AA: ActionApplier<TT>>(&mut self, input: (usize, State)) {
        let (index, state) = input;
        let cost = state.get_cost();
        debug_assert_eq!(
            state.energize(self.graph),
            None,
            "Energization succeeded at the start of a non-initial state"
        );
        let action_transitions: Vec<Vec<TT>> = if state.is_terminal(self.graph) {
            vec![vec![TT::terminal_transition(index as StateIndex, cost)]]
        } else {
            let state = state.to_action_state(self.graph);
            self.iterator
                .prepare(&state)
                .map(|action: Vec<TeamAction>| -> Vec<TT> {
                    AA::apply(&state, cost, self.graph, &action)
                        .into_iter()
                        .map(|(mut transition, successor_state)| {
                            // Index the successor states
                            let successor_index = self.states.index_state(successor_state);
                            transition.set_successor(successor_index as StateIndex);
                            transition
                        })
                        .collect()
                })
                .collect()
        };
        if self.transitions.len() <= index {
            self.transitions.resize_with(index + 1, Default::default);
        }
        self.transitions[index] = action_transitions;
    }

    /// Explore the actions and transitions of the initial state.
    ///
    /// This requires special handling because energization is allowed to succeed in the initial
    /// state without team movement. Normally, this is not the case since all energizations are
    /// attempted after each transition.
    #[inline]
    fn explore_initial<AA: ActionApplier<TT>>(&mut self, input: (usize, State)) {
        let (index, state) = input;
        let cost = state.get_cost();
        let action_transitions: Vec<Vec<TT>> = if state.is_terminal(self.graph) {
            vec![vec![TT::terminal_transition(index as StateIndex, cost)]]
        } else if let Some(bus_outcomes) = state.energize(self.graph) {
            vec![bus_outcomes
                .into_iter()
                .map(|(p, bus_state)| {
                    let successor_state = State {
                        teams: state.teams.clone(),
                        buses: bus_state,
                    };
                    let successor_index = self.states.index_state(successor_state);
                    TT::costless_transition(successor_index as StateIndex, p)
                })
                .collect()]
        } else {
            let state = state.to_action_state(self.graph);
            self.iterator
                .prepare(&state)
                .map(|action| {
                    AA::apply(&state, cost, self.graph, &action)
                        .into_iter()
                        .map(|(mut transition, successor_state)| {
                            // Index the successor states
                            let successor_index = self.states.index_state(successor_state);
                            transition.set_successor(successor_index as StateIndex);
                            transition
                        })
                        .collect()
                })
                .collect()
        };
        if self.transitions.len() <= index {
            self.transitions.resize_with(index + 1, Default::default);
        }
        self.transitions[index] = action_transitions;
    }
}

impl<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> Explorer<'a, TT>
    for NaiveExplorer<'a, TT, AI, SI>
{
    fn memory_limited_explore<AA: ActionApplier<TT>>(
        graph: &'a Graph,
        teams: Vec<TeamState>,
        memory_limit: usize,
    ) -> Result<ExploreResult<TT>, SolveFailure> {
        const MEMORY_SAMPLE_PERIOD: usize = 2_usize.pow(15);
        // NOTE: Previously, initail memory usage was subtracted from the currently allocated.
        // However, in some cases it caused underflow due to memory usage approximation errors.
        let mut max_memory: usize = 0;

        let mut explorer = NaiveExplorer {
            iterator: AI::setup(graph),
            graph,
            states: SI::new(graph, &teams),
            transitions: Vec::new(),
        };
        explorer
            .states
            .index_state(State::start_state(graph, teams));

        {
            let initial = explorer.states.next();
            explorer.explore_initial::<AA>(
                initial.expect("No initial exploration state in StateIndexer"),
            );
        }
        let mut index = 1; // First one indexed
        while let Some(i) = explorer.states.next() {
            explorer.explore_state::<AA>(i);

            index += 1;
            if index % MEMORY_SAMPLE_PERIOD == 0 {
                let allocated = ALLOCATOR.allocated();
                max_memory = std::cmp::max(max_memory, allocated);
                if allocated > memory_limit {
                    return Err(SolveFailure::OutOfMemory {
                        used: max_memory,
                        limit: memory_limit,
                    });
                }
            }
        }

        let allocated = ALLOCATOR.allocated();
        max_memory = std::cmp::max(max_memory, allocated);

        let (bus_states, team_states) = explorer.states.deconstruct();
        let transitions = explorer.transitions;
        Ok(ExploreResult {
            bus_states,
            team_states,
            transitions,
            max_memory,
        })
    }
}
