use super::*;
use crate::utils::sorted_intersects;

#[derive(PartialEq, Debug)]
enum TeamActionState {
    OnUnknownBus,
    OnKnownBus,
    EnRoute,
}

/// Not an action iterator by itself, but holds the data required to build an iterator.
pub struct ProtoIterator {
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
    /// Set of buses in beta_1
    energizable_buses: Vec<Index>,
    /// True if the progress condition is satisfied by an en-route team.
    progress_satisfied: bool,
}

impl ProtoIterator {
    /// Construct ProtoIterator from a state and graph.
    fn from_state(state: &State, graph: &Graph) -> ProtoIterator {
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
        ProtoIterator {
            target_buses,
            minbeta,
            team_states,
            team_buses,
            energizable_buses,
            progress_satisfied,
        }
    }
}

/// Trait that represents an iterator for feasible action set.
/// A(s) in paper.
pub trait ActionIterator: Iterator<Item = Vec<TeamAction>> + Sized {
    fn setup(graph: &Graph) -> Self;
    /// Construct this iterator from ProtoIterator.
    fn from_proto(&mut self, proto: ProtoIterator, state: &State) -> &mut Self;
    /// Construct this iterator from a state and graph.
    #[inline]
    fn from_state(&mut self, state: &State, graph: &Graph) -> &mut Self {
        self.from_proto(ProtoIterator::from_state(state, graph), state)
    }
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
    fn setup(_graph: &Graph) -> Self {
        NaiveIterator {
            target_buses: Vec::default(),
            minbeta: Vec::default(),
            team_states: Vec::default(),
            team_buses: Vec::default(),
            progress_satisfied: false,
            next: None,
        }
    }

    fn from_proto(&mut self, proto: ProtoIterator, _state: &State) -> &mut Self {
        let ProtoIterator {
            target_buses,
            minbeta,
            team_states,
            team_buses,
            energizable_buses: _,
            progress_satisfied,
        } = proto;
        self.target_buses = target_buses;
        self.minbeta = minbeta;
        self.team_states = team_states;
        self.team_buses = team_buses;
        self.progress_satisfied = progress_satisfied;
        self.next = None;
        self.reset();
        self
    }
}

/// An action iterator that wraps around another action iterator and checks for "wait for moving
/// teams" condition during initialization. If the condition is met, only wait action will be
/// issued. Otherwise, the underlying iterator will be initialized and used.
pub struct WaitMovingIterator<T: ActionIterator> {
    /// Underlying iterator.
    iter: T,
    /// Whether we are in waiting state
    waiting_state: bool,
    /// The wait action for this state if the "wait for moving teams" condition is satisfied.
    wait_action: Option<Vec<TeamAction>>,
}

impl<T: ActionIterator> Iterator for WaitMovingIterator<T> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.waiting_state {
            self.wait_action.take()
        } else {
            self.iter.next()
        }
    }
}

impl<T: ActionIterator> ActionIterator for WaitMovingIterator<T> {
    fn setup(graph: &Graph) -> Self {
        Self {
            iter: T::setup(graph),
            waiting_state: false,
            wait_action: None,
        }
    }

    fn from_proto(&mut self, proto: ProtoIterator, state: &State) -> &mut Self {
        let action: Vec<TeamAction> = proto
            .team_states
            .iter()
            .filter_map(|t| match t {
                TeamActionState::EnRoute => Some(CONTINUE_ACTION),
                TeamActionState::OnUnknownBus => Some(WAIT_ACTION),
                TeamActionState::OnKnownBus => None,
            })
            .collect_vec();
        self.waiting_state = proto.progress_satisfied && action.len() == proto.team_states.len();
        if self.waiting_state {
            self.wait_action = Some(action);
        } else {
            self.iter.from_proto(proto, state);
        }
        self
    }
}

/// An action iterator that wraps around another action iterator and eliminates actions according
/// to the "components on the way" condition.
///
/// If an energizable component (i.e., in beta_1) that is on the way is skipped in an action, it
/// will be eliminated.
pub struct OnWayIterator<T: ActionIterator> {
    /// Underlying iterator.
    iter: T,
    /// For each path i to j, there's an entry for the list of components on that path in ascending
    /// order.
    on_way: Array2<Vec<Index>>,
    /// The set of buses in beta_1
    energizable_buses: Vec<Index>,
    /// Bus at which each team is located, represented as index in target_buses.
    /// usize;:MAX if en-route.
    team_buses: Vec<Index>,
}

impl<T: ActionIterator> Iterator for OnWayIterator<T> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(action) = self.iter.next() {
            let on_way: bool = self.team_buses.iter().zip(action.iter()).any(|(&i, &j)| {
                if i == usize::MAX || j == CONTINUE_ACTION || j == WAIT_ACTION {
                    false
                } else {
                    sorted_intersects(
                        self.on_way[[i, j as usize]].iter(),
                        self.energizable_buses.iter(),
                    )
                }
            });
            if on_way {
                continue;
            }
            return Some(action);
        }
        None
    }
}

impl<T: ActionIterator> ActionIterator for OnWayIterator<T> {
    fn setup(graph: &Graph) -> Self {
        let bus_count = graph.branches.len();
        let mut on_way: Array2<Vec<Index>> = Array2::default(graph.travel_times.raw_dim());
        for (((i, j), elem), &direct) in on_way.indexed_iter_mut().zip(graph.travel_times.iter()) {
            if i == j {
                continue;
            }
            for k in 0..bus_count {
                if i == k || j == k {
                    continue;
                }
                let through_k = graph.travel_times[[i, k]] + graph.travel_times[[k, j]];
                if through_k <= direct {
                    elem.push(k);
                }
            }
        }
        Self {
            iter: T::setup(graph),
            on_way,
            energizable_buses: Vec::default(),
            team_buses: Vec::default(),
        }
    }

    #[inline]
    fn from_proto(&mut self, proto: ProtoIterator, state: &State) -> &mut Self {
        self.team_buses = state
            .teams
            .iter()
            .map(|team| match team {
                TeamState::OnBus(i) => *i,
                TeamState::EnRoute(_, _, _) => usize::MAX,
            })
            .collect();
        self.energizable_buses = proto.energizable_buses.clone();
        self.iter.from_proto(proto, state);
        self
    }
}
