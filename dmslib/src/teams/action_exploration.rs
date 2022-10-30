use super::*;

/// Generic trait for the functions that explore the actions of a given state.
pub trait ActionExplorer<'a, T: ActionIterator<'a>> {
    /// Construct an action explorer from SolutionGenerator.
    fn setup(graph: &'a Graph) -> Self;
    /// Explore the actions and transitions of a state at the given index in the
    /// SolutionGenerator.
    fn explore(&mut self, soln: &mut SolutionGenerator, graph: &Graph, index: usize);
    /// Explore the actions and transitions of the initial state.
    ///
    /// This requires special handling because energization is allowed to succeed in the initial
    /// state without team movement. Normally, this is not the case since all energizations are
    /// attempted after each transition.
    fn explore_initial(&mut self, soln: &mut SolutionGenerator, graph: &Graph, index: usize);
}

/// Naive action explorer.
pub struct NaiveExplorer<'a, T: ActionIterator<'a>> {
    /// Action iterator.
    iterator: T,
    /// This struct semantically stores a reference with `'a` lifetime due to wrapped
    /// ActionIterator.
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T: ActionIterator<'a>> ActionExplorer<'a, T> for NaiveExplorer<'a, T> {
    fn setup(graph: &'a Graph) -> Self {
        NaiveExplorer {
            iterator: T::setup(graph),
            _phantom: std::marker::PhantomData,
        }
    }

    #[inline]
    fn explore(&mut self, soln: &mut SolutionGenerator, graph: &Graph, index: usize) {
        let state = soln.get_state(index);
        let cost = state.get_cost();
        debug_assert_eq!(
            state.energize(graph),
            None,
            "Energization succeeded at the start of a non-initial state"
        );
        let action_transitions: Vec<Vec<Transition>> = if state.is_terminal(graph) {
            vec![vec![Transition {
                successor: index,
                p: 1.0,
                cost,
            }]]
        } else {
            self.iterator
                .from_state(&state, graph)
                .map(|action| {
                    let (team_outcome, bus_outcomes) = state.apply_action(graph, &action);
                    bus_outcomes
                        .into_iter()
                        .map(|(p, bus_state)| {
                            let successor_state = State {
                                teams: team_outcome.clone(),
                                buses: bus_state,
                            };
                            let successor_index = soln.index_state(&successor_state);
                            Transition {
                                successor: successor_index,
                                p,
                                cost,
                            }
                        })
                        .collect()
                })
                .collect()
        };
        soln.transitions[index] = action_transitions;
    }

    #[inline]
    fn explore_initial(&mut self, soln: &mut SolutionGenerator, graph: &Graph, index: usize) {
        let state = soln.get_state(index);
        let cost = state.get_cost();
        let action_transitions: Vec<Vec<Transition>> = if state.is_terminal(graph) {
            vec![vec![Transition {
                successor: index,
                p: 1.0,
                cost,
            }]]
        } else if let Some(bus_outcomes) = state.energize(graph) {
            vec![bus_outcomes
                .into_iter()
                .map(|(p, bus_state)| {
                    let successor_state = State {
                        teams: state.teams.clone(),
                        buses: bus_state,
                    };
                    let successor_index = soln.index_state(&successor_state);
                    Transition {
                        successor: successor_index,
                        p,
                        cost: 0.0,
                    }
                })
                .collect()]
        } else {
            self.iterator
                .from_state(&state, graph)
                .map(|action| {
                    let (team_outcome, bus_outcomes) = state.apply_action(graph, &action);
                    bus_outcomes
                        .into_iter()
                        .map(|(p, bus_state)| {
                            let successor_state = State {
                                teams: team_outcome.clone(),
                                buses: bus_state,
                            };
                            let successor_index = soln.index_state(&successor_state);
                            Transition {
                                successor: successor_index,
                                p,
                                cost,
                            }
                        })
                        .collect()
                })
                .collect()
        };
        soln.transitions[index] = action_transitions;
    }
}
