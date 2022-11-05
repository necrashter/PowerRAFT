use super::*;
use crate::utils::{are_indices_sorted, get_repeating_indices, sorted_intersects};
use itertools::structs::CombinationsWithReplacement;

/// Stores action-related information for a state.
pub struct ActionState {
    /// Underlying state
    pub state: State,
    /// Each element of this list at position i will give the smallest j for which
    /// `i` is an element of beta_j(s). j=0 is there's no such j.
    pub minbeta: Vec<Index>,
    /// This vector contains the elements in the set of reachable buses with Unknown
    /// status, beta(s), in ascending order.
    target_buses: Vec<Index>,
    /// Each element of this list at position i will give the smallest j for which
    /// `target_buses[i]` is an element of beta_j(s). j=0 is there's no such j.
    target_minbeta: Vec<Index>,
    /// Node (bus or initial position) at which each team is located, represented by its index.
    /// usize;:MAX if en-route.
    team_nodes: Vec<Index>,
    /// Set of buses in beta_1
    energizable_buses: Vec<Index>,
    /// True if the progress condition is satisfied by an en-route team.
    progress_satisfied: bool,
}

impl State {
    /// Construct ActionState from a state and graph.
    pub fn to_action_state(self, graph: &Graph) -> ActionState {
        let minbeta = self.compute_minbeta(graph);
        let (target_buses, target_minbeta): (Vec<Index>, Vec<Index>) = minbeta
            .iter()
            .enumerate()
            .filter(|(_i, &beta)| beta != 0 && beta != usize::MAX)
            .unzip();
        let team_nodes = self
            .teams
            .iter()
            .map(|team| match team {
                TeamState::OnBus(i) => *i,
                TeamState::EnRoute(_, _, _) => usize::MAX,
            })
            .collect();
        let energizable_buses: Vec<Index> = target_buses
            .iter()
            .zip(target_minbeta.iter())
            .filter_map(|(&i, &beta)| if beta == 1 { Some(i) } else { None })
            .collect();
        let progress_satisfied = self.teams.iter().any(|team| {
            if let TeamState::EnRoute(_, b, _) = team {
                energizable_buses.binary_search(b).is_ok()
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
    type IT<'b>: Iterator<Item = Vec<TeamAction>> + Sized + 'b
    where
        Self: 'b;

    /// Setup the action set from [`Graph`] before the exploration begins.
    fn setup(graph: &'a Graph) -> Self;

    /// Prepare an iterator from state action info.
    fn prepare<'b>(&'b self, action_state: &'b ActionState) -> Self::IT<'b>;

    fn all_actions_in_state(&self, state: &State, graph: &Graph) -> Vec<Vec<TeamAction>> {
        let action_state = state.clone().to_action_state(graph);
        self.prepare(&action_state).collect()
    }
}

/// Naive action iterator without any action-eliminating optimizations.
pub struct NaiveIt<'a> {
    action_state: &'a ActionState,
    /// Next action
    next: Option<Vec<TeamAction>>,
}

impl<'a> NaiveIt<'a> {
    // Reset the iterator
    fn reset(&mut self) {
        let mut next: Option<Vec<TeamAction>> = Some(
            self.action_state
                .state
                .teams
                .iter()
                .map(|team_state| match team_state {
                    TeamState::OnBus(_) => 0,
                    TeamState::EnRoute(_, _, _) => usize::MAX,
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
            if let TeamState::EnRoute(_, _, _) = self.action_state.state.teams[i] {
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
                .any(|&i| i != usize::MAX && self.action_state.target_minbeta[i] == 1)
    }
}

impl Iterator for NaiveIt<'_> {
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
                    if let TeamState::EnRoute(_, destination, _) = team {
                        *destination
                    } else {
                        self.action_state.target_buses[target]
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

pub struct NaiveActions;

impl<'a> ActionSet<'a> for NaiveActions {
    type IT<'b> = NaiveIt<'b>;

    fn setup(_graph: &Graph) -> Self {
        NaiveActions
    }

    fn prepare<'b>(&self, action_state: &'b ActionState) -> Self::IT<'b> {
        let mut it = NaiveIt {
            action_state,
            next: None,
        };
        it.reset();
        it
    }
}

/// An action iterator that eliminates non-optimal permutations.
pub struct PermutationalIt<'a> {
    /// ActionState reference
    action_state: &'a ActionState,
    /// Travel times between each edge.
    travel_times: &'a Array2<Time>,
    /// Teams that are ready to receive orders, i.e., not en-route.
    ready_teams: Vec<usize>,
    /// The node on which each ready team is located.
    ready_team_nodes: Vec<usize>,
    /// Iterator over bus combinations
    bus_combination_iter: CombinationsWithReplacement<std::vec::IntoIter<usize>>,
    /// Stack of next actions from the permutations of last team-bus combination.
    next_actions: Vec<Vec<TeamAction>>,
}

impl<'a> PermutationalIt<'a> {
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
                    .all(|&i| self.action_state.minbeta[i] > 1)
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
                        .map(|(&x, &y)| self.travel_times[[self.ready_team_nodes[x], y]])
                        .collect_vec();
                    let b = permutations[j]
                        .iter()
                        .zip(bus_combination.iter())
                        .map(|(&x, &y)| self.travel_times[[self.ready_team_nodes[x], y]])
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
                    }
                }
            }

            let action_template = self
                .action_state
                .state
                .teams
                .iter()
                .map(|s| match s {
                    TeamState::OnBus(_) => usize::MAX,
                    TeamState::EnRoute(_, destination, _) => *destination,
                })
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

impl<'a> Iterator for PermutationalIt<'a> {
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

pub struct PermutationalActions<'a> {
    travel_times: &'a Array2<Time>,
}

impl<'a> ActionSet<'a> for PermutationalActions<'a> {
    fn setup(graph: &'a Graph) -> Self {
        Self {
            travel_times: &graph.travel_times,
        }
    }

    type IT<'b> = PermutationalIt<'b> where Self: 'b;

    fn prepare<'b>(&'b self, action_state: &'b ActionState) -> Self::IT<'b> {
        let (ready_teams, ready_team_nodes): (Vec<usize>, Vec<usize>) = action_state
            .state
            .teams
            .iter()
            .enumerate()
            .filter_map(|(i, t)| match t {
                TeamState::OnBus(b) => Some((i, b)),
                TeamState::EnRoute(_, _, _) => None,
            })
            .unzip();
        let bus_combination_iter = action_state
            .target_buses
            .clone()
            .into_iter()
            .combinations_with_replacement(ready_teams.len());
        let mut it = PermutationalIt {
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

impl<'a, T: Iterator<Item = Vec<TeamAction>> + Sized> Iterator for WaitMovingIterator<'a, T> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.waiting_state {
            self.wait_action.take()
        } else {
            self.iter.next()
        }
    }
}

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

    type IT<'b> = WaitMovingIterator<'b, T::IT<'b>> where Self: 'b, T: 'b;

    fn prepare<'b>(&'b self, action_state: &'b ActionState) -> Self::IT<'b> {
        let action: Vec<TeamAction> = action_state
            .state
            .teams
            .iter()
            .filter_map(|t| match t {
                TeamState::OnBus(b) => {
                    if *b >= action_state.state.buses.len()
                        || action_state.state.buses[*b] != BusState::Unknown
                    {
                        None
                    } else {
                        Some(*b)
                    }
                }
                TeamState::EnRoute(_, destination, _) => Some(*destination),
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
/// to the "components on the way" condition.
///
/// If an energizable component (i.e., in beta_1) that is on the way is skipped in an action, it
/// will be eliminated.
pub struct EnergizedOnWayIterator<'a, T: Iterator<Item = Vec<TeamAction>> + Sized> {
    /// Underlying iterator.
    iter: T,
    /// For each path i to j, there's an entry for the list of components on that path in ascending
    /// order.
    on_way: &'a Array2<Vec<Index>>,
    action_state: &'a ActionState,
}

impl<'a, T: Iterator<Item = Vec<TeamAction>> + Sized> Iterator for EnergizedOnWayIterator<'a, T> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(action) = self.iter.next() {
            let on_way: bool =
                self.action_state
                    .team_nodes
                    .iter()
                    .zip(action.iter())
                    .any(|(&i, &j)| {
                        if i == usize::MAX {
                            false
                        } else {
                            sorted_intersects(
                                self.on_way[[i, j as usize]].iter(),
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

pub struct FilterEnergizedOnWay<'a, T: ActionSet<'a>> {
    base: T,
    /// For each path i to j, there's an entry for the list of components on that path in ascending
    /// order.
    on_way: Array2<Vec<Index>>,
    /// This struct semantically stores a reference with `'a` lifetime due to wrapped
    /// ActionSet.
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T: ActionSet<'a>> ActionSet<'a> for FilterEnergizedOnWay<'a, T> {
    fn setup(graph: &'a Graph) -> Self {
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
            base: T::setup(graph),
            on_way,
            _phantom: std::marker::PhantomData,
        }
    }

    type IT<'b> = EnergizedOnWayIterator<'b, T::IT<'b>> where T: 'b, Self: 'b;

    fn prepare<'b>(&'b self, action_state: &'b ActionState) -> Self::IT<'b> {
        EnergizedOnWayIterator {
            iter: self.base.prepare(action_state),
            action_state,
            on_way: &self.on_way,
        }
    }
}
