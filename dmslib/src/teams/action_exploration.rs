use super::*;

/// Generic trait for the functions that explore the actions of a given state.
pub trait ActionExplorer<T: for<'a> ActionIterator<'a>> {
    /// Construct an action explorer from SolutionGenerator.
    fn setup(graph: &Graph) -> Self;
    /// Explore the actions and transitions of a state at the given index in the
    /// SolutionGenerator.
    fn explore(&mut self, soln: &mut SolutionGenerator, index: usize);
    /// Explore the actions and transitions of the initial state.
    ///
    /// This requires special handling because energization is allowed to succeed in the initial
    /// state without team movement. Normally, this is not the case since all energizations are
    /// attempted after each transition.
    fn explore_initial(&mut self, soln: &mut SolutionGenerator, index: usize);
}

/// Naive action explorer.
pub struct NaiveExplorer<T: for<'a> ActionIterator<'a>> {
    iterator: T,
}

impl<T: for<'a> ActionIterator<'a>> ActionExplorer<T> for NaiveExplorer<T> {
    fn setup(graph: &Graph) -> Self {
        NaiveExplorer {
            iterator: T::setup(graph),
        }
    }

    #[inline]
    fn explore(&mut self, soln: &mut SolutionGenerator, index: usize) {
        let state = soln.get_state(index);
        let cost = state.get_cost();
        debug_assert_eq!(
            state.energize(&soln.graph),
            None,
            "Energization succeeded at the start of a non-initial state"
        );
        let action_transitions: Vec<Vec<Transition>> = if state.is_terminal(&soln.graph) {
            vec![vec![Transition {
                successor: index,
                p: 1.0,
                cost,
            }]]
        } else {
            self.iterator
                .from_state(&state, &soln.graph)
                .map(|action| {
                    let (team_outcome, bus_outcomes) = state.apply_action(&soln.graph, &action);
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
    fn explore_initial(&mut self, soln: &mut SolutionGenerator, index: usize) {
        let state = soln.get_state(index);
        let cost = state.get_cost();
        let action_transitions: Vec<Vec<Transition>> = if state.is_terminal(&soln.graph) {
            vec![vec![Transition {
                successor: index,
                p: 1.0,
                cost,
            }]]
        } else if let Some(bus_outcomes) = state.energize(&soln.graph) {
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
                .from_state(&state, &soln.graph)
                .map(|action| {
                    let (team_outcome, bus_outcomes) = state.apply_action(&soln.graph, &action);
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
