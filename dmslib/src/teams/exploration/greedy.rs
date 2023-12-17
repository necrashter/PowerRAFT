use ordered_float::NotNan;

use super::*;

/// Greedy action explorer.
pub struct GreedyExplorer<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> {
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

/// A heuristic function that will be used to select an action in each state.
/// The action with lowest heuristic will be selected.
fn heuristic(graph: &Graph, state: &State, actions: &[TeamAction]) -> f64 {
    // Local horizon for this heuristic.
    const HORIZON: f64 = 100.0;
    // Cost incurred by each bus.
    // Energized/Damaged buses are ignored since they don't affect the future cost.
    let mut costs: Vec<f64> = state
        .buses
        .iter()
        .map(|bus| {
            if *bus == BusState::Unknown {
                HORIZON
            } else {
                0.0
            }
        })
        .collect();
    for (team, &action) in state.teams.iter().zip(actions) {
        let time = if team.index == action {
            // En route case
            team.time
        } else {
            graph.travel_times[(team.index as usize, action as usize)]
        };
        let pf = graph.pfs[action as usize] as f64;
        // Incur cost until the team reaches. Then, incur cost with probability pf.
        let cost = time as f64 + ((HORIZON - time as f64) * pf);
        costs[action as usize] = costs[action as usize].min(cost);
    }
    costs.into_iter().sum()
}

impl<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> GreedyExplorer<'a, TT, AI, SI> {
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
            let action = self
                .iterator
                .prepare(&state)
                .min_by_key(|action| {
                    NotNan::new(heuristic(self.graph, &state.state, action)).unwrap()
                })
                .expect("No actions in a non-terminal state");
            vec![AA::apply(&state, cost, self.graph, &action)
                .into_iter()
                .map(|(mut transition, successor_state)| {
                    // Index the successor states
                    let successor_index = self.states.index_state(successor_state);
                    transition.set_successor(successor_index as StateIndex);
                    transition
                })
                .collect()]
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
                    TT::time1_transition(successor_index as StateIndex, cost, p)
                })
                .collect()]
        } else {
            let state = state.to_action_state(self.graph);
            let action = self
                .iterator
                .prepare(&state)
                .min_by_key(|action| {
                    NotNan::new(heuristic(self.graph, &state.state, action)).unwrap()
                })
                .expect("No actions in a non-terminal state");
            vec![AA::apply(&state, cost, self.graph, &action)
                .into_iter()
                .map(|(mut transition, successor_state)| {
                    // Index the successor states
                    let successor_index = self.states.index_state(successor_state);
                    transition.set_successor(successor_index as StateIndex);
                    transition
                })
                .collect()]
        };
        if self.transitions.len() <= index {
            self.transitions.resize_with(index + 1, Default::default);
        }
        self.transitions[index] = action_transitions;
    }
}

impl<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> Explorer<'a, TT>
    for GreedyExplorer<'a, TT, AI, SI>
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

        let mut explorer = GreedyExplorer {
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
