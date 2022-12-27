use super::*;

#[cfg(test)]
mod tests;

/// Get the minimum amount of time until a team arrives when the teams are ordered with the given
/// action.
#[inline]
fn min_time_until_arrival(
    graph: &Graph,
    teams: &[TeamState],
    actions: &[TeamAction],
) -> Option<Time> {
    teams
        .iter()
        .zip(actions.iter())
        .filter_map(|(team, &action)| match team {
            TeamState::OnBus(source) => {
                if action == *source {
                    None
                } else {
                    let dest = action as usize;
                    let travel_time = graph.travel_times[(*source, dest)];
                    Some(travel_time)
                }
            }
            TeamState::EnRoute(source, dest, t) => {
                debug_assert!(action == *dest);
                let travel_time = graph.travel_times[(*source, *dest)];
                Some(travel_time - t)
            }
        })
        .min()
}

/// Trait for functions that determine the amount of time to be passed when an action is applied.
pub trait DetermineActionTime {
    /// Get the amount of time to be passed when the given action is applied.
    fn get_time(graph: &Graph, action_state: &ActionState, actions: &[TeamAction]) -> Time;

    /// Get the amount of time to be passed when the given action is applied at the given state.
    ///
    /// Syntactic sugar for [`DetermineActionTime::get_time`].
    fn get_time_state(graph: &Graph, state: State, actions: &[TeamAction]) -> Time {
        Self::get_time(graph, &state.to_action_state(graph), actions)
    }
}

/// Dummy [`DetermineActionTime`] implementation that always returns 1.
/// This essentially mimics [`RegularTransition`] with [`TimedTransition`].
/// Used to test their equivalence when all transitions have time = 1.
pub struct ConstantTime;
impl DetermineActionTime for ConstantTime {
    #[inline]
    fn get_time(_graph: &Graph, _action_state: &ActionState, _actions: &[TeamAction]) -> Time {
        1
    }
}

/// Get the minimum amount of time until a team arrives when the teams are ordered with the given
/// action.
pub struct TimeUntilArrival;
impl DetermineActionTime for TimeUntilArrival {
    #[inline]
    fn get_time(graph: &Graph, action_state: &ActionState, actions: &[TeamAction]) -> Time {
        min_time_until_arrival(graph, &action_state.state.teams, actions)
            // NOTE: if there's no minimum time, it means that all teams are waiting,
            // which shouldn't happen.
            .expect("No minimum time in TimeUntilArrival (all waiting)")
    }
}

/// Get the minimum amount of time until an energization attempt happens when the teams are
/// ordered with the given action.
///
/// This yields the longest time by which we can advance without potential loss of optimality.
pub struct TimeUntilEnergization;
impl DetermineActionTime for TimeUntilEnergization {
    #[inline]
    fn get_time(graph: &Graph, action_state: &ActionState, actions: &[TeamAction]) -> Time {
        action_state
            .state
            .teams
            .iter()
            .zip(actions.iter())
            .filter_map(|(team, &action)| {
                // Only consider buses that are energizable.
                let beta = action_state.minbeta[action];
                if beta != 1 {
                    return None;
                }
                match team {
                    TeamState::OnBus(source) => {
                        debug_assert_ne!(
                            action, *source,
                            "A team cannot reach & wait on a bus without energizing it."
                        );
                        let travel_time = graph.travel_times[(*source, action)];
                        Some(travel_time)
                    }
                    TeamState::EnRoute(source, dest, t) => {
                        debug_assert!(action == *dest);
                        let travel_time = graph.travel_times[(*source, *dest)];
                        Some(travel_time - t)
                    }
                }
            })
            .min()
            .expect("Cannot get time until energization: progress condition is not satisfied")
    }
}

/// Advance time for the teams when the given action is ordered.
#[inline]
fn advance_time_for_teams(
    graph: &Graph,
    teams: &[TeamState],
    actions: &[TeamAction],
    time: usize,
) -> Vec<TeamState> {
    teams
        .iter()
        .zip(actions.iter())
        .map(|(team, &action)| {
            let team = team.clone();
            match team {
                TeamState::OnBus(source) => {
                    let dest = action as usize;
                    let travel_time = graph.travel_times[(source, dest)];
                    if time >= travel_time {
                        TeamState::OnBus(dest)
                    } else {
                        TeamState::EnRoute(source, dest, time)
                    }
                }
                TeamState::EnRoute(source, dest, t) => {
                    debug_assert!(action == dest);
                    let travel_time = graph.travel_times[(source, dest)];
                    if time + t >= travel_time {
                        TeamState::OnBus(dest)
                    } else {
                        TeamState::EnRoute(source, dest, t + time)
                    }
                }
            }
        })
        .collect()
}

/// Performs recursive energization with given team and bus state on the given graph.
/// Returns a pair of bool and outcomes.
/// The bool determines whether at least one energization happened.
/// Outcomes are a list of probability and bus state pairs.
fn recursive_energization(
    graph: &Graph,
    teams: &[TeamState],
    buses: Vec<BusState>,
) -> (bool, Vec<(f64, Vec<BusState>)>) {
    // Buses on which a team is present
    let team_buses: Vec<usize> = teams
        .iter()
        .filter_map(|team| match team {
            TeamState::OnBus(i) => {
                let i = *i;
                if i < buses.len() {
                    Some(i)
                } else {
                    None
                }
            }
            TeamState::EnRoute(_, _, _) => None,
        })
        .unique()
        .collect();
    // All energization outcomes with probability.
    let mut outcomes: Vec<(f64, Vec<BusState>)> = Vec::new();
    // Recursive energization process
    let mut queue: Vec<(f64, Vec<BusState>)> = Vec::new();

    /// Compute alpha as defined in paper
    macro_rules! get_alpha {
        ($state:expr) => {{
            team_buses
                .clone()
                .into_iter()
                .filter(|i| {
                    let i = *i;
                    $state[i] == BusState::Unknown && {
                        graph.connected[i]
                            || graph.branches[i]
                                .iter()
                                .any(|j| $state[*j] == BusState::Energized)
                    }
                })
                .collect()
        }};
    }
    /// Add permutations to the queue
    macro_rules! add_permutations {
        ($p:expr, $state:expr, $alpha:expr) => {{
            for &i in &$alpha {
                $state[i] = BusState::Damaged;
            }
            'permutations: loop {
                let p = $alpha.iter().fold($p, |acc, &i| {
                    let pf = graph.pfs[i];
                    acc * if $state[i] == BusState::Damaged {
                        pf
                    } else {
                        1.0 - pf
                    }
                });
                queue.push((p, $state.clone()));
                for &i in &$alpha {
                    if $state[i] == BusState::Damaged {
                        $state[i] = BusState::Energized;
                        continue 'permutations;
                    } else {
                        $state[i] = BusState::Damaged;
                    }
                }
                break 'permutations;
            }
        }};
    }

    // Handle initial state
    {
        let mut state = buses;
        let alpha: Vec<usize> = get_alpha!(state);
        if alpha.is_empty() {
            outcomes.push((1.0, state));
            return (false, outcomes);
        }

        add_permutations!(1.0, state, alpha);
    }
    // Handle states in queue
    while let Some(next) = queue.pop() {
        let (p, mut state) = next;
        let alpha: Vec<usize> = get_alpha!(state);
        if alpha.is_empty() {
            // Discard transitions with p = 0
            if p != 0.0 {
                outcomes.push((p, state));
            }
            continue;
        }

        add_permutations!(p, state, alpha);
    }
    (true, outcomes)
}

impl State {
    /// Attempt to energize without moving the teams.
    pub fn energize(&self, graph: &Graph) -> Option<Vec<(f64, Vec<BusState>)>> {
        let (success, outcomes) = recursive_energization(graph, &self.teams, self.buses.clone());
        if success {
            Some(outcomes)
        } else {
            // No energizations happened
            debug_assert_eq!(outcomes[0].0, 1.0);
            debug_assert_eq!(outcomes[0].1, self.buses);
            None
        }
    }
}

/// Trait that contains methods to apply given actions at a given state.
/// The resulting transitions will have TransitionType.
pub trait ActionApplier<TransitionType: Transition> {
    /// Apply the action at given state, returning a list of transitions and the corresponding
    /// successor states.
    fn apply(
        action_state: &ActionState,
        cost: f64,
        graph: &Graph,
        actions: &[TeamAction],
    ) -> Vec<(TransitionType, State)>;

    /// Apply the action at given state, returning a list of transitions and the corresponding
    /// successor states.
    ///
    /// Syntactic sugar for [`ActionApplier::apply`]
    #[inline]
    fn apply_state(
        state: &State,
        cost: f64,
        graph: &Graph,
        actions: &[TeamAction],
    ) -> Vec<(TransitionType, State)> {
        Self::apply(&state.clone().to_action_state(graph), cost, graph, actions)
    }
}

/// The most basic action applier.
/// Applies the action, advances time by 1 unit, and returns `RegularTransition`s.
pub struct NaiveActionApplier;

impl ActionApplier<RegularTransition> for NaiveActionApplier {
    #[inline]
    fn apply(
        action_state: &ActionState,
        cost: f64,
        graph: &Graph,
        actions: &[TeamAction],
    ) -> Vec<(RegularTransition, State)> {
        debug_assert_eq!(actions.len(), action_state.state.teams.len());
        let teams = advance_time_for_teams(graph, &action_state.state.teams, actions, 1);
        recursive_energization(graph, &teams, action_state.state.buses.clone())
            .1
            .into_iter()
            .map(|(p, bus_state)| {
                let transition = RegularTransition {
                    successor: usize::MAX,
                    p,
                    cost,
                };
                let successor_state = State {
                    teams: teams.clone(),
                    buses: bus_state,
                };
                (transition, successor_state)
            })
            .collect()
    }
}

/// Simple action applier that takes time into consideration.
/// Advances time according to the amount returned by [`DetermineActionTime`] generic.
/// Returns [`TimedTransition`]s.
///
/// Never construct this struct. Use static methods only.
pub struct TimedActionApplier<F: DetermineActionTime> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: DetermineActionTime> ActionApplier<TimedTransition> for TimedActionApplier<F> {
    #[inline]
    fn apply(
        action_state: &ActionState,
        cost: f64,
        graph: &Graph,
        actions: &[TeamAction],
    ) -> Vec<(TimedTransition, State)> {
        debug_assert_eq!(actions.len(), action_state.state.teams.len());
        // Get minimum time until a team reaches its destination.
        let time: Time = F::get_time(graph, action_state, actions);
        let teams = advance_time_for_teams(graph, &action_state.state.teams, actions, time);
        recursive_energization(graph, &teams, action_state.state.buses.clone())
            .1
            .into_iter()
            .map(|(p, bus_state)| {
                let transition = TimedTransition {
                    successor: usize::MAX,
                    p,
                    cost,
                    time,
                };
                let successor_state = State {
                    teams: teams.clone(),
                    buses: bus_state,
                };
                (transition, successor_state)
            })
            .collect()
    }
}
