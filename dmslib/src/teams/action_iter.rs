use super::*;

#[derive(PartialEq, Debug)]
enum TeamActionState {
    OnUnknownBus,
    OnKnownBus,
    EnRoute,
}

/// Trait that represents an iterator for feasible action set.
pub trait ActionIterator: Iterator {
    fn from_state(state: &State, graph: &Graph) -> Self;
}

/// Naive action iterator without any action-eliminating optimizations.
pub struct NaiveIterator {
    /// This vector contains the elements in the set of reachable buses with Unknown
    /// status, beta(s), in ascending order.
    target_buses: Vec<Index>,
    /// Each element of this list at position i will give the smallest j for which
    /// `target_buses[i]` is an element of beta_j(s). j=0 is there's no such j.
    minbeta: Vec<Index>,
    /// State of the teams
    team_states: Vec<TeamActionState>,
    /// Bus at which each team is located, represented as index in target_buses.
    /// usize;:MAX if en-route or not in target_buses.
    team_buses: Vec<Index>,
    /// True if the progress condition is satisfied by an en-route team.
    progress_satisfied: bool,
    /// Next action
    next: Option<Vec<TeamAction>>,
}

impl NaiveIterator {
    // Reset the iterator
    fn reset(&mut self) {
        let mut next: Option<Vec<TeamAction>> = Some(
            self.team_states
                .iter()
                .map(|team_state| match team_state {
                    TeamActionState::OnUnknownBus => WAIT_ACTION,
                    TeamActionState::OnKnownBus => 0,
                    TeamActionState::EnRoute => CONTINUE_ACTION,
                })
                .collect(),
        );
        // Ensure progress condition.
        while next.is_some() && !self.progress_condition(next.as_ref().unwrap()) {
            next = self.next_action(next.unwrap());
        }
        self.next = next;
    }

    /// Updates the `current` action field with the next actions, not necessarily feasible.
    /// Returns True if actions wrapped around.
    fn next_action(&self, mut action: Vec<TeamAction>) -> Option<Vec<TeamAction>> {
        for i in 0..action.len() {
            if action[i] == CONTINUE_ACTION {
                debug_assert_eq!(self.team_states[i], TeamActionState::EnRoute);
                continue;
            }
            action[i] += 1;
            if (action[i] as usize) == self.team_buses[i] {
                // TODO: Encode this as wait?
                action[i] += 1;
            }
            if (action[i] as usize) < self.target_buses.len() {
                return Some(action);
            } else {
                action[i] = if self.team_states[i] == TeamActionState::OnUnknownBus {
                    WAIT_ACTION
                } else if self.team_buses[i] == 0 {
                    debug_assert!(1 < self.target_buses.len());
                    1
                } else {
                    0
                };
            }
        }
        // If we reach this point every action is wait -> we wrapped around; no more actions
        None
    }

    /// Returns true if the progress condition is satisfied.
    /// Progress condition assures that at least one team is going to an energizable bus.
    fn progress_condition(&self, action: &[TeamAction]) -> bool {
        self.progress_satisfied
            || action
                .iter()
                .any(|&i| i >= 0 && self.minbeta[i as usize] == 1)
    }
}

impl Iterator for NaiveIterator {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next.take();
        if let Some(action) = current {
            let current: Vec<TeamAction> = action
                .iter()
                .map(|&i| {
                    if i == CONTINUE_ACTION || i == WAIT_ACTION {
                        i
                    } else {
                        self.target_buses[i as usize] as isize
                    }
                })
                .collect();
            let mut next = self.next_action(action);
            while next.is_some() && !self.progress_condition(next.as_ref().unwrap()) {
                next = self.next_action(next.unwrap());
            }
            self.next = next;
            Some(current)
        } else {
            None
        }
    }
}

impl ActionIterator for NaiveIterator {
    fn from_state(state: &State, graph: &Graph) -> NaiveIterator {
        let minbeta = state.minbetas(graph);
        let (target_buses, minbeta): (Vec<Index>, Vec<Index>) = minbeta
            .iter()
            .enumerate()
            .filter(|(_i, &beta)| beta != 0 && beta != usize::MAX)
            .unzip();
        let team_states: Vec<TeamActionState> = state
            .teams
            .iter()
            .map(|team| match team {
                TeamState::OnBus(i) => {
                    let i = *i;
                    if i >= state.buses.len() {
                        // The team is at a starting position, so it has to move.
                        // This is treated like a known bus.
                        TeamActionState::OnKnownBus
                    } else if state.buses[i] == BusState::Unknown {
                        TeamActionState::OnUnknownBus
                    } else {
                        TeamActionState::OnKnownBus
                    }
                }
                TeamState::EnRoute(_, _, _) => TeamActionState::EnRoute,
            })
            .collect();
        let team_buses: Vec<Index> = state
            .teams
            .iter()
            .map(|team| match team {
                TeamState::OnBus(i) => match target_buses.binary_search(i) {
                    Ok(j) => j,
                    Err(_) => usize::MAX,
                },
                TeamState::EnRoute(_, _, _) => usize::MAX,
            })
            .collect();
        let energizable_buses: Vec<Index> = target_buses
            .iter()
            .zip(minbeta.iter())
            .filter_map(|(&i, &beta)| if beta == 1 { Some(i) } else { None })
            .collect();
        let progress_satisfied = state.teams.iter().any(|team| {
            if let TeamState::EnRoute(_, b, _) = team {
                energizable_buses.binary_search(b).is_ok()
            } else {
                false
            }
        });
        let mut it = NaiveIterator {
            target_buses,
            minbeta,
            team_states,
            team_buses,
            next: None,
            progress_satisfied,
        };
        it.reset();
        it
    }
}
