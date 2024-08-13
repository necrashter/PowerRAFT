use super::*;

#[cfg(test)]
mod tests;

/// Performs energization with the given action (list of target buses).
/// Outcomes are a list of probability and bus state pairs.
fn energize(
    graph: &Graph,
    buses: Vec<BusState>,
    action: &[TeamAction],
) -> Vec<(Probability, Vec<BusState>)> {
    // All energization outcomes with probability.
    let mut outcomes: Vec<(Probability, Vec<BusState>)> = Vec::new();
    let mut state = buses;
    for &i in action {
        state[i as usize] = BusState::Damaged;
    }
    'permutations: loop {
        let p = action.iter().fold(1.0, |acc, &i| {
            let pf = graph.pfs[i as usize];
            acc * if state[i as usize] == BusState::Damaged {
                pf
            } else {
                1.0 - pf
            }
        });
        // Discard transitions with p = 0
        if p != 0.0 {
            outcomes.push((p, state.clone()));
        }
        for i in action {
            let i = *i as usize;
            if state[i] == BusState::Damaged {
                state[i] = BusState::Energized;
                continue 'permutations;
            } else {
                state[i] = BusState::Damaged;
            }
        }
        break 'permutations;
    }
    outcomes
}

/// Trait that contains methods to apply given actions at a given state.
/// The resulting transitions will have TransitionType.
pub trait ActionApplier<TransitionType: Transition> {
    /// Apply the action at given state, returning a list of transitions and the corresponding
    /// successor states.
    fn apply(
        state: &State,
        cost: Cost,
        graph: &Graph,
        actions: &[TeamAction],
    ) -> Vec<(TransitionType, State)>;
}

/// The most basic action applier.
/// Applies the action, advances time by 1 unit, and returns `RegularTransition`s.
pub struct NaiveActionApplier;

impl ActionApplier<RegularTransition> for NaiveActionApplier {
    #[inline]
    fn apply(
        state: &State,
        cost: Cost,
        graph: &Graph,
        actions: &[TeamAction],
    ) -> Vec<(RegularTransition, State)> {
        energize(graph, state.buses.clone(), actions)
            .into_iter()
            .map(|(p, bus_state)| {
                let transition = RegularTransition {
                    successor: StateIndex::MAX,
                    p,
                    cost,
                };
                let successor_state = State { buses: bus_state };
                (transition, successor_state)
            })
            .collect()
    }
}
