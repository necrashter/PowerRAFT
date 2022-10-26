use super::*;

/// Generic trait for the functions that explore the actions of a given state.
pub trait ActionExplorer {
    /// Explore the actions and transitions of a state at the given index in the
    /// SolutionGenerator.
    fn explore<T: ActionIterator>(soln: &mut SolutionGenerator, index: usize);
}

/// Naive action explorer.
pub struct NaiveExplorer;

impl ActionExplorer for NaiveExplorer {
    #[inline]
    fn explore<T: ActionIterator>(soln: &mut SolutionGenerator, index: usize) {
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
            state
                .actions::<T>(&soln.graph)
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

/// Action explorer for initial state.
///
/// This requires special handling because energization is allowed to succeed in the initial
/// state without team movement. Normally, this is not the case since all energizations are
/// attempted after each transition.
pub struct InitialStateExplorer;

impl ActionExplorer for InitialStateExplorer {
    #[inline]
    fn explore<T: ActionIterator>(soln: &mut SolutionGenerator, index: usize) {
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
            state
                .actions::<T>(&soln.graph)
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
