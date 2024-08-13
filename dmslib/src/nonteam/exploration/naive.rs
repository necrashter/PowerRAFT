use super::*;

/// Naive action explorer.
pub struct NaiveExplorer<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> {
    /// Reference to a graph.
    graph: &'a Graph,
    /// State indexer.
    states: SI,
    /// 3D vector of transitions:
    /// - `transitions[i]`: Actions of state i
    /// - `transitions[i][j]`: Transitions of action j in state i
    transitions: Vec<Vec<Vec<TT>>>,
    _phantom: std::marker::PhantomData<AI>,
}

impl<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> NaiveExplorer<'a, TT, AI, SI> {
    /// Explore the actions and transitions of a state at the given index in the state indexer.
    #[inline]
    fn explore_state<AA: ActionApplier<TT>>(&mut self, input: (usize, State)) {
        let (index, state) = input;
        let cost = state.get_cost();
        let action_transitions: Vec<Vec<TT>> = if state.is_terminal(self.graph) {
            vec![vec![TT::terminal_transition(index as StateIndex, cost)]]
        } else {
            AI::get_actions(&state, self.graph)
                .into_iter()
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
}

impl<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> Explorer<'a, TT>
    for NaiveExplorer<'a, TT, AI, SI>
{
    fn memory_limited_explore<AA: ActionApplier<TT>>(
        graph: &'a Graph,
        memory_limit: usize,
    ) -> Result<ExploreResult<TT>, SolveFailure> {
        const MEMORY_SAMPLE_PERIOD: usize = 2_usize.pow(15);
        // NOTE: Previously, initial memory usage was subtracted from the currently allocated.
        // However, in some cases it caused underflow due to memory usage approximation errors.
        let mut max_memory: usize = 0;

        let mut explorer = NaiveExplorer {
            graph,
            states: SI::new(graph),
            transitions: Vec::new(),
            _phantom: std::marker::PhantomData::<AI>,
        };
        explorer.states.index_state(State::start_state(graph));

        let mut index = 0;
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

        let bus_states = explorer.states.deconstruct();
        let transitions = explorer.transitions;
        Ok(ExploreResult {
            bus_states,
            transitions,
            max_memory,
        })
    }
}
