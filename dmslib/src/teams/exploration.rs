use super::*;

/// Generic trait for the functions that explore the actions of a given state.
pub trait Explorer<TT: Transition> {
    /// Explore the possible states starting from the given team state.
    fn explore<AA: ActionApplier<TT>>(
        graph: &Graph,
        teams: Vec<TeamState>,
    ) -> (Array2<BusState>, Array2<TeamState>, Vec<Vec<Vec<TT>>>);
}

/// Naive action explorer.
pub struct NaiveExplorer<'a, TT: Transition, AI: ActionIterator<'a>> {
    /// Action iterator.
    iterator: AI,
    /// Reference to a graph.
    graph: &'a Graph,
    /// States
    states: StateIndexer,
    /// 3D vector of transitions:
    /// - `transitions[i]`: Actions of state i
    /// - `transitions[i][j]`: Transitions of action j in state i
    transitions: Vec<Vec<Vec<TT>>>,
}

impl<'a, TT: Transition, AI: ActionIterator<'a>> NaiveExplorer<'a, TT, AI> {
    /// Explore the actions and transitions of a state at the given index in the
    /// StateIndexer.
    #[inline]
    fn explore_state<AA: ActionApplier<TT>>(&mut self, index: usize) {
        let state = self.states.get_state(index);
        let cost = state.get_cost();
        debug_assert_eq!(
            state.energize(self.graph),
            None,
            "Energization succeeded at the start of a non-initial state"
        );
        let action_transitions: Vec<Vec<TT>> = if state.is_terminal(self.graph) {
            vec![vec![TT::terminal_transition(index, cost)]]
        } else {
            self.iterator
                .prepare_from_state(&state, self.graph)
                .map(|action| {
                    AA::apply(&state, cost, self.graph, &action)
                        .into_iter()
                        .map(|(mut transition, successor_state)| {
                            // Index the successor states
                            let successor_index = self.states.index_state(&successor_state);
                            transition.set_successor(successor_index);
                            transition
                        })
                        .collect()
                })
                .collect()
        };
        // It is guaranteed that the pushed element has the given index.
        // Because states are visited in sequential order.
        self.transitions.push(action_transitions);
    }

    /// Explore the actions and transitions of the initial state.
    ///
    /// This requires special handling because energization is allowed to succeed in the initial
    /// state without team movement. Normally, this is not the case since all energizations are
    /// attempted after each transition.
    #[inline]
    fn explore_initial<AA: ActionApplier<TT>>(&mut self, index: usize) {
        let state = self.states.get_state(index);
        let cost = state.get_cost();
        let action_transitions: Vec<Vec<TT>> = if state.is_terminal(self.graph) {
            vec![vec![TT::terminal_transition(index, cost)]]
        } else if let Some(bus_outcomes) = state.energize(self.graph) {
            vec![bus_outcomes
                .into_iter()
                .map(|(p, bus_state)| {
                    let successor_state = State {
                        teams: state.teams.clone(),
                        buses: bus_state,
                    };
                    let successor_index = self.states.index_state(&successor_state);
                    TT::costless_transition(successor_index, p)
                })
                .collect()]
        } else {
            self.iterator
                .prepare_from_state(&state, self.graph)
                .map(|action| {
                    AA::apply(&state, cost, self.graph, &action)
                        .into_iter()
                        .map(|(mut transition, successor_state)| {
                            // Index the successor states
                            let successor_index = self.states.index_state(&successor_state);
                            transition.set_successor(successor_index);
                            transition
                        })
                        .collect()
                })
                .collect()
        };
        // It is guaranteed that the pushed element has the given index.
        // Because states are visited in sequential order.
        self.transitions.push(action_transitions);
    }
}

impl<TT: Transition, AI: for<'a> ActionIterator<'a>> Explorer<TT> for NaiveExplorer<'_, TT, AI> {
    fn explore<AA: ActionApplier<TT>>(
        graph: &Graph,
        teams: Vec<TeamState>,
    ) -> (Array2<BusState>, Array2<TeamState>, Vec<Vec<Vec<TT>>>) {
        let mut explorer = NaiveExplorer {
            iterator: AI::setup(graph),
            graph,
            states: StateIndexer::new(graph.branches.len(), teams.len()),
            transitions: Vec::new(),
        };
        let mut index = explorer
            .states
            .index_state(&State::start_state(graph, teams));
        explorer.explore_initial::<AA>(index);
        index += 1;
        while index < explorer.states.state_count {
            explorer.explore_state::<AA>(index);
            index += 1;
        }
        let (bus_states, team_states) = explorer.states.deconstruct();
        let transitions = explorer.transitions;
        (bus_states, team_states, transitions)
    }
}
