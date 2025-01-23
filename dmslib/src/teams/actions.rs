use super::*;
use crate::utils::{are_indices_sorted, get_repeating_indices, sorted_intersects};
use itertools::structs::CombinationsWithReplacement;

/// Stores action-related information for a state.
pub struct ActionState {
    /// Underlying state
    pub state: State,
    /// Each element of this list at position i will give the smallest j for which
    /// `i` is an element of beta_j(s).
    /// j=0 if the bus is not Unknown,
    /// `usize::MAX` if unreachable.
    pub minbeta: Vec<BusIndex>,
    /// This vector contains the elements in the set of reachable buses with Unknown
    /// status, beta(s), in ascending order.
    target_buses: Vec<BusIndex>,
    /// Each element of this list at position i will give the smallest j for which
    /// `target_buses[i]` is an element of beta_j(s). j=0 is there's no such j.
    target_minbeta: Vec<BusIndex>,
    /// Node (bus or initial position) at which each team is located, represented by its index.
    /// usize;:MAX if en-route.
    team_nodes: Vec<BusIndex>,
    /// Set of buses in beta_1
    energizable_buses: Vec<BusIndex>,
    /// True if the progress condition is satisfied by an en-route team.
    progress_satisfied: bool,
}

impl State {
    /// Construct ActionState from a state and graph.
    pub fn to_action_state(self, graph: &Graph) -> ActionState {
        let minbeta = self.compute_minbeta(graph);
        let (target_buses, target_minbeta): (Vec<BusIndex>, Vec<BusIndex>) = minbeta
            .iter()
            .enumerate()
            .filter_map(|(i, &beta)| {
                if beta != 0 && beta != BusIndex::MAX {
                    Some((i as BusIndex, beta))
                } else {
                    None
                }
            })
            .unzip();
        let team_nodes = self
            .teams
            .iter()
            .map(|team| {
                if team.time == 0 {
                    team.index
                } else {
                    BusIndex::MAX
                }
            })
            .collect();
        let energizable_buses: Vec<BusIndex> = target_buses
            .iter()
            .zip(target_minbeta.iter())
            .filter_map(|(&i, &beta)| if beta == 1 { Some(i as BusIndex) } else { None })
            .collect();
        let progress_satisfied = self.teams.iter().any(|team| {
            if team.time > 0 {
                energizable_buses.binary_search(&team.index).is_ok()
            } else {
                false
            }
        });
        ActionState {
            state: self,
            minbeta,
            target_buses,
            target_minbeta,
            team_nodes,
            energizable_buses,
            progress_satisfied,
        }
    }
}

/// Trait that represents an iterator for feasible action set.
/// A(s) in paper.
pub trait ActionSet<'a> {
    /// The type of the iterator for this action set type.
    type IT<'b>: Iterator<Item = Vec<TeamAction>> + Sized + 'b
    where
        Self: 'b;

    /// Setup the action set from [`Graph`] before the exploration begins.
    fn setup(graph: &'a Graph) -> Self;

    /// Prepare an iterator from state action info.
    fn prepare<'b>(&'b self, action_state: &'b ActionState) -> Self::IT<'b>;

    /// Return all actions in a state as a `Vec`.
    #[inline]
    fn all_actions_in_state(&self, state: &State, graph: &Graph) -> Vec<Vec<TeamAction>> {
        let action_state = state.clone().to_action_state(graph);
        self.prepare(&action_state).collect()
    }
}

/// Naive action iterator without any action-eliminating optimizations.
///
/// See [`NaiveActions`].
pub struct NaiveIterator<'a> {
    action_state: &'a ActionState,
    /// Next action
    next: Option<Vec<TeamAction>>,
}

impl NaiveIterator<'_> {
    /// Reset the iterator
    fn reset(&mut self) {
        let mut next: Option<Vec<TeamAction>> = Some(
            self.action_state
                .state
                .teams
                .iter()
                .map(|team_state| {
                    if team_state.time == 0 {
                        0
                    } else {
                        BusIndex::MAX
                    }
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
            if self.action_state.state.teams[i].time > 0 {
                // En-route
                continue;
            }
            action[i] += 1;
            if (action[i] as usize) < self.action_state.target_buses.len() {
                return Some(action);
            } else {
                action[i] = 0;
            }
        }
        // If we reach this point every action is wait -> we wrapped around; no more actions
        None
    }

    /// Returns true if the progress condition is satisfied.
    /// Progress condition assures that at least one team is going to an energizable bus.
    fn progress_condition(&self, action: &[TeamAction]) -> bool {
        self.action_state.progress_satisfied
            || action
                .iter()
                .any(|&i| i != BusIndex::MAX && self.action_state.target_minbeta[i as usize] == 1)
    }
}

impl Iterator for NaiveIterator<'_> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next.take();
        if let Some(action) = current {
            let current: Vec<TeamAction> = self
                .action_state
                .state
                .teams
                .iter()
                .zip(action.iter())
                .map(|(team, &target)| {
                    if team.time > 0 {
                        team.index
                    } else {
                        self.action_state.target_buses[target as usize]
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

/// Naive action set definition without any action-eliminating optimizations.
pub struct NaiveActions;

impl ActionSet<'_> for NaiveActions {
    type IT<'b> = NaiveIterator<'b>;

    fn setup(_graph: &Graph) -> Self {
        NaiveActions
    }

    fn prepare<'b>(&self, action_state: &'b ActionState) -> Self::IT<'b> {
        let mut it = NaiveIterator {
            action_state,
            next: None,
        };
        it.reset();
        it
    }
}

/// See [`PermutationalActions`].
pub struct PermutationalIterator<'a> {
    /// ActionState reference
    action_state: &'a ActionState,
    /// Travel times between each edge.
    travel_times: &'a Array2<Time>,
    /// Teams that are ready to receive orders, i.e., not en-route.
    ready_teams: Vec<usize>,
    /// The node on which each ready team is located.
    ready_team_nodes: Vec<BusIndex>,
    /// Iterator over bus combinations
    bus_combination_iter: CombinationsWithReplacement<std::vec::IntoIter<BusIndex>>,
    /// Stack of next actions from the permutations of last team-bus combination.
    next_actions: Vec<Vec<TeamAction>>,
}

impl PermutationalIterator<'_> {
    /// Get the next target bus combination from `self.bus_combination_iter`.
    /// Consider the permutations and eliminate non-optimal ones.
    ///
    /// Called when self.next_actions is empty.
    fn next_bus_combination(&mut self) -> bool {
        if let Some(bus_combination) = self.bus_combination_iter.next() {
            // Check progress condition
            if !self.action_state.progress_satisfied
                && bus_combination
                    .iter()
                    .all(|&i| self.action_state.minbeta[i as usize] > 1)
            {
                return self.next_bus_combination();
            }
            let repeating_indices = get_repeating_indices(&bus_combination);

            // Permutation iterator
            let permutations = (0..self.ready_teams.len())
                .permutations(self.ready_teams.len())
                .filter(|permutation| {
                    // Remove the permutations that are equivalent due to repeating elements in
                    // combination.
                    if !are_indices_sorted(permutation, &repeating_indices) {
                        return false;
                    }
                    true
                })
                .collect_vec();

            // Compare each permutation
            // Whether each permutation is eliminated
            let mut eliminated: Vec<bool> = vec![false; permutations.len()];
            for i in 0..permutations.len() {
                if eliminated[i] {
                    continue;
                }
                for j in (i + 1)..permutations.len() {
                    if eliminated[j] {
                        continue;
                    }
                    let a = permutations[i]
                        .iter()
                        .zip(bus_combination.iter())
                        .map(|(&x, &y)| {
                            self.travel_times[(self.ready_team_nodes[x] as usize, y as usize)]
                        })
                        .collect_vec();
                    let b = permutations[j]
                        .iter()
                        .zip(bus_combination.iter())
                        .map(|(&x, &y)| {
                            self.travel_times[(self.ready_team_nodes[x] as usize, y as usize)]
                        })
                        .collect_vec();
                    let mut all_smaller_eq = true;
                    let mut all_greater_eq = true;
                    for (x, y) in a.iter().zip(b.iter()) {
                        match x.cmp(y) {
                            std::cmp::Ordering::Less => {
                                all_greater_eq = false;
                            }
                            std::cmp::Ordering::Equal => {}
                            std::cmp::Ordering::Greater => {
                                all_smaller_eq = false;
                            }
                        }
                    }
                    if all_smaller_eq {
                        // All travel times in a is smaller, eliminate b
                        eliminated[j] = true;
                    } else if all_greater_eq {
                        // All travel times in b is smaller, eliminate a
                        eliminated[i] = true;
                        // i has been eliminated.
                        break;
                    }
                }
            }

            let action_template = self
                .action_state
                .state
                .teams
                .iter()
                .map(|s| if s.time == 0 { BusIndex::MAX } else { s.index })
                .collect_vec();
            self.next_actions = eliminated
                .into_iter()
                .rev()
                .zip(permutations.into_iter().rev())
                .filter_map(|(eliminated, permutation)| {
                    if eliminated {
                        None
                    } else {
                        let mut action = action_template.clone();
                        for (&perm_i, &bus) in permutation.iter().zip(bus_combination.iter()) {
                            let team_index = self.ready_teams[perm_i];
                            action[team_index] = bus as TeamAction;
                        }
                        Some(action)
                    }
                })
                .collect_vec();
            true
        } else {
            false
        }
    }
}

impl Iterator for PermutationalIterator<'_> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(action) = self.next_actions.pop() {
            Some(action)
        } else if self.next_bus_combination() {
            self.next()
        } else {
            None
        }
    }
}

/// An action set definition that eliminates non-optimal permutations of orders.
///
/// For example, if action (1, 2) takes (2, 2) time for each team and (2, 1) takes (1, 1), then
/// the first action is eliminated. Likewise, an action with time (2, 2) would be eliminated in
/// favor of actions with time (1, 2) or (2, 1).
///
/// If both take the same amount of time, e.g., time (2, 2) and (2, 2),  then one of them is
/// eliminated.
pub struct PermutationalActions<'a> {
    travel_times: &'a Array2<Time>,
}

impl<'a> ActionSet<'a> for PermutationalActions<'a> {
    fn setup(graph: &'a Graph) -> Self {
        Self {
            travel_times: &graph.travel_times,
        }
    }

    type IT<'b>
        = PermutationalIterator<'b>
    where
        Self: 'b;

    fn prepare<'b>(&'b self, action_state: &'b ActionState) -> Self::IT<'b> {
        let (ready_teams, ready_team_nodes): (Vec<usize>, Vec<BusIndex>) = action_state
            .state
            .teams
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                if t.time == 0 {
                    Some((i, t.index))
                } else {
                    None
                }
            })
            .unzip();
        let bus_combination_iter = action_state
            .target_buses
            .clone()
            .into_iter()
            .combinations_with_replacement(ready_teams.len());
        let mut it = PermutationalIterator {
            action_state,
            travel_times: self.travel_times,
            ready_teams,
            ready_team_nodes,
            bus_combination_iter,
            next_actions: Vec::new(),
        };
        it.next_bus_combination();
        it
    }
}

/// An action iterator that wraps around another action iterator and checks for "wait for moving
/// teams" condition during initialization. If the condition is met, only wait action will be
/// issued. Otherwise, the underlying iterator will be initialized and used.
///
/// NOTE: This doesn't work correctly under all conditions.
pub struct WaitMovingIterator<'a, T: Iterator<Item = Vec<TeamAction>> + Sized> {
    /// Underlying iterator.
    iter: T,
    /// Whether we are in waiting state
    waiting_state: bool,
    /// The wait action for this state if the "wait for moving teams" condition is satisfied.
    wait_action: Option<Vec<TeamAction>>,
    /// This struct semantically stores a reference with `'a` lifetime due to wrapped
    /// ActionSet.
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<T: Iterator<Item = Vec<TeamAction>> + Sized> Iterator for WaitMovingIterator<'_, T> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.waiting_state {
            self.wait_action.take()
        } else {
            self.iter.next()
        }
    }
}

/// NOTE: This doesn't work correctly under all conditions.
pub struct WaitMovingActions<'a, T: ActionSet<'a>> {
    base: T,
    /// This struct semantically stores a reference with `'a` lifetime due to wrapped
    /// ActionSet.
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T: ActionSet<'a>> ActionSet<'a> for WaitMovingActions<'a, T> {
    fn setup(graph: &'a Graph) -> Self {
        Self {
            base: T::setup(graph),
            _phantom: std::marker::PhantomData,
        }
    }

    type IT<'b>
        = WaitMovingIterator<'b, T::IT<'b>>
    where
        Self: 'b,
        T: 'b;

    fn prepare<'b>(&'b self, action_state: &'b ActionState) -> Self::IT<'b> {
        let action: Vec<TeamAction> = action_state
            .state
            .teams
            .iter()
            .filter_map(|t| {
                if t.time == 0 {
                    if (t.index as usize) >= action_state.state.buses.len()
                        || action_state.state.buses[t.index as usize] != BusState::Unknown
                    {
                        None
                    } else {
                        Some(t.index)
                    }
                } else {
                    Some(t.index)
                }
            })
            .collect_vec();
        let waiting_state =
            action_state.progress_satisfied && action.len() == action_state.state.teams.len();
        let iter = self.base.prepare(action_state);
        let wait_action = if waiting_state { Some(action) } else { None };
        WaitMovingIterator {
            iter,
            waiting_state,
            wait_action,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// An action iterator that wraps around another action iterator and eliminates actions according
/// to the "energized components on the way" condition:
/// - If an energizable component (i.e., in `beta_1` set) that is on the way is skipped in an
///   action, it will be eliminated.
///
/// See [`FilterEnergizedOnWay`].
pub struct EnergizedOnWayIterator<'a, T: Iterator<Item = Vec<TeamAction>> + Sized> {
    /// Underlying iterator.
    iter: T,
    /// For each path i to j, there's an entry for the list of components on that path in ascending
    /// order.
    on_way: &'a Array2<Vec<BusIndex>>,
    action_state: &'a ActionState,
}

impl<T: Iterator<Item = Vec<TeamAction>> + Sized> Iterator for EnergizedOnWayIterator<'_, T> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(action) = self.iter.next() {
            let on_way: bool =
                self.action_state
                    .team_nodes
                    .iter()
                    .zip(action.iter())
                    .any(|(&i, &j)| {
                        if i == BusIndex::MAX {
                            false
                        } else {
                            sorted_intersects(
                                self.on_way[(i as usize, j as usize)].iter(),
                                self.action_state.energizable_buses.iter(),
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

/// A struct that wraps another action set definition and eliminates the actions in which a
/// team skips an energizable component (i.e., in `beta_1` set ) on its way.
pub struct FilterEnergizedOnWay<'a, T: ActionSet<'a>> {
    base: T,
    /// For each path i to j, there's an entry for the list of components on that path in ascending
    /// order.
    on_way: Array2<Vec<BusIndex>>,
    /// This struct semantically stores a reference with `'a` lifetime due to wrapped
    /// ActionSet.
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T: ActionSet<'a>> ActionSet<'a> for FilterEnergizedOnWay<'a, T> {
    fn setup(graph: &'a Graph) -> Self {
        let on_way = graph.get_components_on_way();
        Self {
            base: T::setup(graph),
            on_way,
            _phantom: std::marker::PhantomData,
        }
    }

    type IT<'b>
        = EnergizedOnWayIterator<'b, T::IT<'b>>
    where
        T: 'b,
        Self: 'b;

    fn prepare<'b>(&'b self, action_state: &'b ActionState) -> Self::IT<'b> {
        EnergizedOnWayIterator {
            iter: self.base.prepare(action_state),
            action_state,
            on_way: &self.on_way,
        }
    }
}

/// A struct that wraps another action set definition and eliminates a given action if
/// there's another action that sends teams to the same buses or buses on the way.
///
/// Unlike other action sets, this one collects all actions and compares them with one another.
///
/// Complexity is O(actions^2 * teams).
pub struct FilterOnWay<'a, T: ActionSet<'a>> {
    base: T,
    /// For each path i to j, there's an entry for the list of components on that path in ascending
    /// order.
    on_way: Array2<Vec<BusIndex>>,
    /// This struct semantically stores a reference with `'a` lifetime due to wrapped ActionSet.
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T: ActionSet<'a>> ActionSet<'a> for FilterOnWay<'a, T> {
    fn setup(graph: &'a Graph) -> Self {
        let on_way = graph.get_components_on_way();
        Self {
            base: T::setup(graph),
            on_way,
            _phantom: std::marker::PhantomData,
        }
    }

    type IT<'b>
        = std::vec::IntoIter<Vec<TeamAction>>
    where
        Self: 'b;

    fn prepare<'b>(&'b self, action_state: &'b ActionState) -> Self::IT<'b> {
        let actions = self.base.prepare(action_state).collect_vec();
        let mut eliminated = vec![false; actions.len()];
        let team_nodes = &action_state.team_nodes;

        for i in 0..actions.len() {
            if eliminated[i] {
                continue;
            }
            for j in (i + 1)..actions.len() {
                if eliminated[j] {
                    continue;
                }
                let mut j_is_on_way = true;
                let mut i_is_on_way = true;
                for (&team, (&ai, &aj)) in team_nodes
                    .iter()
                    .zip(actions[i].iter().zip(actions[j].iter()))
                {
                    if ai == aj {
                        continue;
                    }
                    if self.on_way[(team as usize, ai as usize)]
                        .binary_search(&aj)
                        .is_err()
                    {
                        // aj is NOT on way
                        j_is_on_way = false;
                    }
                    if self.on_way[(team as usize, aj as usize)]
                        .binary_search(&ai)
                        .is_err()
                    {
                        // ai is NOT on way
                        i_is_on_way = false;
                    }
                }
                if i_is_on_way {
                    debug_assert!(!j_is_on_way);
                    eliminated[j] = true;
                } else if j_is_on_way {
                    // All travel times in b is smaller, eliminate a
                    eliminated[i] = true;
                    // i has been eliminated.
                    break;
                }
            }
        }

        actions
            .into_iter()
            .zip(eliminated)
            .filter_map(|(action, e)| if e { None } else { Some(action) })
            .collect_vec()
            .into_iter()
    }
}
