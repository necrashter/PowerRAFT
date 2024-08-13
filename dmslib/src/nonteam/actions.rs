use super::*;
use itertools::Itertools;

/// Trait that represents an iterator for feasible action set.
/// A(s) in paper.
pub trait ActionSet<'a> {
    /// Return all actions in a state as a `Vec`.
    fn get_actions(state: &State, graph: &Graph) -> Vec<Vec<TeamAction>>;
}

/// Naive action set definition without any action-eliminating optimizations.
///
/// Missing:
/// 1. a -> b and b -> a in possible energizations (not possible currently anyway)
/// 2. Duplicates (edge case in dense connections)
/// 3. Resources energize ALL connected immediately.
pub struct NaiveActions;

impl ActionSet<'_> for NaiveActions {
    fn get_actions(state: &State, graph: &Graph) -> Vec<Vec<TeamAction>> {
        // source bus -> list of target buses
        let (_source_buses, bus_targets): (Vec<BusIndex>, Vec<Vec<BusIndex>>) = state
            .buses
            .iter()
            .enumerate()
            .filter_map(|(i, bus)| {
                if bus != &BusState::Energized {
                    return None;
                }
                let targets: Vec<BusIndex> = graph.branches[i]
                    .iter()
                    .cloned()
                    .filter(|j| state.buses[*j as usize] == BusState::Unknown)
                    .collect();
                if targets.is_empty() {
                    None
                } else {
                    Some((i as BusIndex, targets))
                }
            })
            .unzip();

        // Unknown buses connected to resources
        // We energize these immediately for now
        let connected = graph
            .connected
            .iter()
            .enumerate()
            .filter(|&(bus_index, &conn)| conn && state.buses[bus_index] == BusState::Unknown)
            .map(|(bus_index, _)| bus_index as TeamAction)
            .collect_vec();

        if bus_targets.is_empty() {
            vec![connected]
        } else {
            let mut prod = bus_targets
                .into_iter()
                .multi_cartesian_product()
                .map(|targets| targets.into_iter().unique().collect_vec())
                .collect_vec();
            // We need to get rid of actions in which multiple source buses target the same bus for energization.
            // Get target bus count for each combination.
            let prod_counts = prod.iter().map(|targets| targets.len()).collect_vec();
            let max_prod_count = *prod_counts.iter().max().unwrap();
            // Eliminate if target bus count is not max.
            let mut retain_condition = prod_counts.into_iter().map(|count| count == max_prod_count);
            prod.retain(|_| retain_condition.next().unwrap());

            if !connected.is_empty() {
                for action in prod.iter_mut() {
                    action.extend_from_slice(&connected);
                }
            }

            prod
        }
    }
}
