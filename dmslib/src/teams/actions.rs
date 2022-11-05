use super::*;
use crate::utils::{
    are_indices_sorted, get_repeating_indices, is_graph_cyclic, sorted_intersection,
    sorted_intersects,
};
use itertools::structs::{Combinations, CombinationsWithReplacement};

/// Simplified state of a team as fas as actions are concerned.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TeamActionState {
    OnUnknownBus,
    OnKnownBus,
    EnRoute,
}

/// Stores action-related information for a state.
pub struct ActionState {
    /// Underlying state
    pub state: State,
    /// Each element of this list at position i will give the smallest j for which
    /// `i` is an element of beta_j(s). j=0 is there's no such j.
    pub minbeta: Vec<Index>,
    /// This vector contains the elements in the set of reachable buses with Unknown
    /// status, beta(s), in ascending order.
    pub target_buses: Vec<Index>,
    /// Each element of this list at position i will give the smallest j for which
    /// `target_buses[i]` is an element of beta_j(s). j=0 is there's no such j.
    pub target_minbeta: Vec<Index>,
    /// State of the teams
    pub team_states: Vec<TeamActionState>,
    /// Node (bus or initial position) at which each team is located, represented by its index.
    /// usize;:MAX if en-route.
    pub team_nodes: Vec<Index>,
    /// Bus at which each team is located, represented as index in target_buses.
    /// usize;:MAX if en-route or not in target_buses.
    pub team_buses_target: Vec<Index>,
    /// Set of buses in beta_1
    pub energizable_buses: Vec<Index>,
    /// True if the progress condition is satisfied by an en-route team.
    pub progress_satisfied: bool,
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
        let team_states: Vec<TeamActionState> = self
            .teams
            .iter()
            .map(|team| match team {
                TeamState::OnBus(i) => {
                    let i = *i;
                    if i >= self.buses.len() {
                        // The team is at a starting position, so it has to move.
                        // This is treated like a known bus.
                        TeamActionState::OnKnownBus
                    } else if self.buses[i] == BusState::Unknown {
                        TeamActionState::OnUnknownBus
                    } else {
                        TeamActionState::OnKnownBus
                    }
                }
                TeamState::EnRoute(_, _, _) => TeamActionState::EnRoute,
            })
            .collect();
        let team_nodes = self
            .teams
            .iter()
            .map(|team| match team {
                TeamState::OnBus(i) => *i,
                TeamState::EnRoute(_, _, _) => usize::MAX,
            })
            .collect();
        let team_buses_target: Vec<Index> = self
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
            team_states,
            team_nodes,
            team_buses_target,
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
                .team_states
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
                debug_assert_eq!(self.action_state.team_states[i], TeamActionState::EnRoute);
                continue;
            }
            action[i] += 1;
            if (action[i] as usize) == self.action_state.team_buses_target[i] {
                // TODO: Encode this as wait?
                action[i] += 1;
            }
            if (action[i] as usize) < self.action_state.target_buses.len() {
                return Some(action);
            } else {
                action[i] = if self.action_state.team_states[i] == TeamActionState::OnUnknownBus {
                    WAIT_ACTION
                } else if self.action_state.team_buses_target[i] == 0 {
                    debug_assert!(1 < self.action_state.target_buses.len());
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
        self.action_state.progress_satisfied
            || action
                .iter()
                .any(|&i| i >= 0 && self.action_state.target_minbeta[i as usize] == 1)
    }
}

impl Iterator for NaiveIt<'_> {
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
                        self.action_state.target_buses[i as usize] as isize
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
    /// Travel times between each edge.
    travel_times: &'a Array2<Time>,
    action_state: &'a ActionState,
    /// Teams that have to move, i.e., standing on a known bus or an initial location outside bus.
    must_move_teams: Vec<usize>,
    /// Teams that are allowed to wait, but can also move.
    can_move_teams: Vec<usize>,
    /// The number of team that are currently moving from can_move_teams
    moving_team_count: usize,
    /// Iterator over which additional teams will move from the set of teams that can wait.
    moving_team_iter: Combinations<std::vec::IntoIter<usize>>,
    bus_combination_iter: CombinationsWithReplacement<std::vec::IntoIter<usize>>,
    /// Currently ordered teams
    ordered_teams: Vec<usize>,
    /// Stack of next actions from the permutations of last team-bus combination.
    next_actions: Vec<Vec<TeamAction>>,
}

impl<'a> PermutationalIt<'a> {
    /// Call this function after changing moving_team_count
    fn prepare_moving_team_iter(&mut self) -> bool {
        self.moving_team_iter = self
            .can_move_teams
            .clone()
            .into_iter()
            .combinations(self.moving_team_count);
        self.next_team_combination()
    }

    /// Get the next moving team combination from `self.moving_team_iter`.
    fn next_team_combination(&mut self) -> bool {
        let next_combination = self.moving_team_iter.next();
        if let Some(combination) = next_combination {
            // Total moving teams: must_move_teams + combination
            self.ordered_teams = self
                .must_move_teams
                .iter()
                .cloned()
                .chain(combination.into_iter())
                .collect();
            self.bus_combination_iter = self
                .action_state
                .target_buses
                .clone()
                .into_iter()
                .combinations_with_replacement(self.ordered_teams.len());
            self.next_bus_combination()
        } else {
            false
        }
    }

    /// Get the next target bus combination from `self.bus_combination_iter`.
    /// Consider the permutations and eliminate non-optimal ones.
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
            // The node on which each ordered team is located.
            let ordered_team_nodes = self
                .ordered_teams
                .iter()
                .map(|&i| self.action_state.team_nodes[i])
                .collect_vec();
            // Get the intersection between team_nodes and targets.
            let bus_target_intersection = sorted_intersection(
                &bus_combination,
                &ordered_team_nodes.iter().cloned().sorted().collect(),
            );
            let repeating_indices = get_repeating_indices(&bus_combination);

            // Permutation iterator
            let permutations = (0..self.ordered_teams.len())
                .permutations(self.ordered_teams.len())
                .filter(|permutation| {
                    // Remove the permutations that are equivalent due to repeating elements in
                    // combination.
                    if !are_indices_sorted(permutation, &repeating_indices) {
                        return false;
                    }
                    // Remove the permutations that send a team to the bus on which it's located.
                    for (&team_index, &target) in permutation.iter().zip(bus_combination.iter()) {
                        if ordered_team_nodes[team_index] == target {
                            return false;
                        }
                    }
                    // Check cycles (teams changing buses with each other)
                    // There cannot be cycles if intersection is not greater than 1.
                    if bus_target_intersection.len() > 1 {
                        let edges: Vec<(usize, usize)> = permutation
                            .iter()
                            .cloned()
                            .zip(bus_combination.iter().cloned())
                            .filter_map(|(a, b)| {
                                let a = ordered_team_nodes[a];
                                let a = bus_target_intersection.binary_search(&a);
                                let b = bus_target_intersection.binary_search(&b);
                                match (a, b) {
                                    (Ok(x), Ok(y)) => Some((x, y)),
                                    _ => None,
                                }
                            })
                            .sorted()
                            .collect();
                        if is_graph_cyclic(bus_target_intersection.len(), &edges) {
                            return false;
                        }
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
                        .map(|(&x, &y)| self.travel_times[[ordered_team_nodes[x], y]])
                        .collect_vec();
                    let b = permutations[j]
                        .iter()
                        .zip(bus_combination.iter())
                        .map(|(&x, &y)| self.travel_times[[ordered_team_nodes[x], y]])
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
                .team_states
                .iter()
                .map(|s| match s {
                    TeamActionState::EnRoute => CONTINUE_ACTION,
                    _ => WAIT_ACTION,
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
                            let team_index = self.ordered_teams[perm_i];
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

    /// Called when self.next_actions is empty. Changes bus or team combination.
    fn get_next_permutations(&mut self) -> bool {
        if self.next_bus_combination() || self.next_team_combination() {
            true
        } else if self.moving_team_count < self.can_move_teams.len() {
            self.moving_team_count += 1;
            self.prepare_moving_team_iter()
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
        } else if self.get_next_permutations() {
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
        let mut must_move_teams = Vec::new();
        let mut can_move_teams = Vec::new();
        for (i, state) in action_state.team_states.iter().enumerate() {
            match state {
                TeamActionState::OnUnknownBus => can_move_teams.push(i),
                TeamActionState::OnKnownBus => must_move_teams.push(i),
                TeamActionState::EnRoute => {}
            }
        }
        let mut it = PermutationalIt {
            travel_times: self.travel_times,
            action_state,
            must_move_teams,
            can_move_teams,
            moving_team_count: 0,
            moving_team_iter: Vec::new().into_iter().combinations(0),
            bus_combination_iter: Vec::new().into_iter().combinations_with_replacement(0),
            ordered_teams: Vec::new(),
            next_actions: Vec::new(),
        };
        it.prepare_moving_team_iter();
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
            .team_states
            .iter()
            .filter_map(|t| match t {
                TeamActionState::EnRoute => Some(CONTINUE_ACTION),
                TeamActionState::OnUnknownBus => Some(WAIT_ACTION),
                TeamActionState::OnKnownBus => None,
            })
            .collect_vec();
        let waiting_state =
            action_state.progress_satisfied && action.len() == action_state.team_states.len();
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
                        if i == usize::MAX || j == CONTINUE_ACTION || j == WAIT_ACTION {
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
