use super::*;
use crate::utils::{is_graph_cyclic, sorted_intersection, sorted_intersects};
use itertools::structs::{Combinations, CombinationsWithReplacement};

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
    /// Node (bus or initial position) at which each team is located, represented by its index.
    /// usize;:MAX if en-route.
    team_nodes: Vec<Index>,
    /// Bus at which each team is located, represented as index in target_buses.
    /// usize;:MAX if en-route or not in target_buses.
    team_buses_target: Vec<Index>,
    /// Set of buses in beta_1
    energizable_buses: Vec<Index>,
    /// True if the progress condition is satisfied by an en-route team.
    progress_satisfied: bool,
}

impl ProtoIterator {
    /// Construct ProtoIterator from a state and graph.
    fn prepare_from_state(state: &State, graph: &Graph) -> ProtoIterator {
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
        let team_nodes = state
            .teams
            .iter()
            .map(|team| match team {
                TeamState::OnBus(i) => *i,
                TeamState::EnRoute(_, _, _) => usize::MAX,
            })
            .collect();
        let team_buses_target: Vec<Index> = state
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
            team_nodes,
            team_buses_target,
            energizable_buses,
            progress_satisfied,
        }
    }
}

/// Trait that represents an iterator for feasible action set.
/// A(s) in paper.
pub trait ActionIterator<'a>: Iterator<Item = Vec<TeamAction>> + Sized {
    fn setup(graph: &'a Graph) -> Self;
    /// Construct this iterator from ProtoIterator.
    fn prepare_from_proto(&mut self, proto: ProtoIterator, state: &State) -> &mut Self;
    /// Construct this iterator from a state and graph.
    #[inline]
    fn prepare_from_state(&mut self, state: &State, graph: &Graph) -> &mut Self {
        self.prepare_from_proto(ProtoIterator::prepare_from_state(state, graph), state)
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
    team_buses_target: Vec<Index>,
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
            if (action[i] as usize) == self.team_buses_target[i] {
                // TODO: Encode this as wait?
                action[i] += 1;
            }
            if (action[i] as usize) < self.target_buses.len() {
                return Some(action);
            } else {
                action[i] = if self.team_states[i] == TeamActionState::OnUnknownBus {
                    WAIT_ACTION
                } else if self.team_buses_target[i] == 0 {
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

impl ActionIterator<'_> for NaiveIterator {
    fn setup(_graph: &Graph) -> Self {
        NaiveIterator {
            target_buses: Vec::default(),
            minbeta: Vec::default(),
            team_states: Vec::default(),
            team_buses_target: Vec::default(),
            progress_satisfied: false,
            next: None,
        }
    }

    fn prepare_from_proto(&mut self, proto: ProtoIterator, _state: &State) -> &mut Self {
        let ProtoIterator {
            target_buses,
            minbeta,
            team_states,
            team_nodes: _,
            team_buses_target,
            energizable_buses: _,
            progress_satisfied,
        } = proto;
        self.target_buses = target_buses;
        self.minbeta = minbeta;
        self.team_states = team_states;
        self.team_buses_target = team_buses_target;
        self.progress_satisfied = progress_satisfied;
        self.next = None;
        self.reset();
        self
    }
}

/// An action iterator that eliminates non-optimal permutations.
pub struct PermutationalIterator<'a> {
    /// Travel times between each edge.
    travel_times: &'a Array2<Time>,
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
    team_nodes: Vec<Index>,
    /// Teams that have to move, i.e., standing on a known bus or an initial location outside bus.
    must_move_teams: Vec<usize>,
    /// Teams that are allowed to wait, but can also move.
    can_move_teams: Vec<usize>,
    /// The number of team that are currently moving from can_move_teams
    moving_team_count: usize,
    /// Iterator over which additional teams will move from the set of teams that can wait.
    moving_team_iter: Combinations<std::vec::IntoIter<usize>>,
    bus_combination_iter: CombinationsWithReplacement<std::vec::IntoIter<usize>>,
    /// True if the progress condition is satisfied by an en-route team.
    progress_satisfied: bool,
    /// Currently ordered teams
    ordered_teams: Vec<usize>,
    /// Next permutations stack
    next_permutations: Vec<Vec<usize>>,
}

impl<'a> PermutationalIterator<'a> {
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
            if !self.progress_satisfied && bus_combination.iter().all(|&i| self.minbeta[i] > 1) {
                self.next_bus_combination();
            }
            let team_nodes = self
                .ordered_teams
                .iter()
                .map(|&i| self.team_nodes[i])
                .collect_vec();
            // Get the intersection between team_nodes and targets
            let bus_target_intersection = sorted_intersection(
                &bus_combination,
                &team_nodes.iter().cloned().sorted().collect(),
            );

            // Permutation iterator
            let permutations = bus_combination
                .into_iter()
                .permutations(self.ordered_teams.len())
                .collect_vec();
            // Whether each permutation is eliminated
            let mut eliminated: Vec<bool> = vec![false; permutations.len()];

            // Check cycles (teams changing buses with each other)
            if bus_target_intersection.len() > 1 {
                for (i, permutation) in permutations.iter().enumerate() {
                    let edges: Vec<(usize, usize)> = team_nodes
                        .iter()
                        .cloned()
                        .zip(permutation.iter().cloned())
                        .filter_map(|(a, b)| {
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
                        eliminated[i] = true;
                    }
                }
            }

            // Compare each permutation
            for i in 0..permutations.len() {
                if eliminated[i] {
                    continue;
                }
                for j in (i + 1)..permutations.len() {
                    if eliminated[j] {
                        continue;
                    }
                    let a = team_nodes
                        .iter()
                        .zip(permutations[i].iter())
                        .map(|(&x, &y)| self.travel_times[[x, y]])
                        .collect_vec();
                    let b = team_nodes
                        .iter()
                        .zip(permutations[j].iter())
                        .map(|(&x, &y)| self.travel_times[[x, y]])
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

            self.next_permutations = eliminated
                .into_iter()
                .rev()
                .zip(permutations.into_iter().rev())
                .filter_map(
                    |(eliminated, permutation)| {
                        if eliminated {
                            None
                        } else {
                            Some(permutation)
                        }
                    },
                )
                .collect_vec();
            true
        } else {
            false
        }
    }

    /// Called when self.next_permutations is empty. Changes bus or team combination.
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

impl<'a> Iterator for PermutationalIterator<'a> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.next_permutations.pop() {
            let mut action = self
                .team_states
                .iter()
                .map(|s| match s {
                    TeamActionState::EnRoute => CONTINUE_ACTION,
                    _ => WAIT_ACTION,
                })
                .collect_vec();
            for (&team_index, &target) in self.ordered_teams.iter().zip(next.iter()) {
                action[team_index] = target as TeamAction;
            }
            Some(action)
        } else if self.get_next_permutations() {
            self.next()
        } else {
            None
        }
    }
}

impl<'a> ActionIterator<'a> for PermutationalIterator<'a> {
    fn setup(graph: &'a Graph) -> PermutationalIterator<'a> {
        PermutationalIterator {
            travel_times: &graph.travel_times,
            target_buses: Vec::default(),
            minbeta: Vec::default(),
            team_states: Vec::default(),
            team_nodes: Vec::default(),
            must_move_teams: Vec::default(),
            can_move_teams: Vec::default(),
            moving_team_count: 0,
            moving_team_iter: Vec::default().into_iter().combinations(0),
            bus_combination_iter: Vec::default().into_iter().combinations_with_replacement(0),
            progress_satisfied: false,
            ordered_teams: Vec::default(),
            next_permutations: Vec::default(),
        }
    }

    fn prepare_from_proto(&mut self, proto: ProtoIterator, _state: &State) -> &mut Self {
        let ProtoIterator {
            target_buses,
            minbeta,
            team_states,
            team_nodes,
            team_buses_target: _,
            energizable_buses: _,
            progress_satisfied,
        } = proto;
        self.target_buses = target_buses;
        self.minbeta = minbeta;
        self.team_states = team_states;
        self.team_nodes = team_nodes;
        self.progress_satisfied = progress_satisfied;

        self.must_move_teams.clear();
        self.can_move_teams.clear();
        for (i, state) in self.team_states.iter().enumerate() {
            match state {
                TeamActionState::OnUnknownBus => self.can_move_teams.push(i),
                TeamActionState::OnKnownBus => self.must_move_teams.push(i),
                TeamActionState::EnRoute => {}
            }
        }
        self.moving_team_count = 0;
        self.prepare_moving_team_iter();

        self
    }
}

/// An action iterator that wraps around another action iterator and checks for "wait for moving
/// teams" condition during initialization. If the condition is met, only wait action will be
/// issued. Otherwise, the underlying iterator will be initialized and used.
pub struct WaitMovingIterator<'a, T: ActionIterator<'a>> {
    /// Underlying iterator.
    iter: T,
    /// Whether we are in waiting state
    waiting_state: bool,
    /// The wait action for this state if the "wait for moving teams" condition is satisfied.
    wait_action: Option<Vec<TeamAction>>,
    /// This struct semantically stores a reference with `'a` lifetime due to wrapped
    /// ActionIterator.
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T: ActionIterator<'a>> Iterator for WaitMovingIterator<'a, T> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.waiting_state {
            self.wait_action.take()
        } else {
            self.iter.next()
        }
    }
}

impl<'a, T: ActionIterator<'a>> ActionIterator<'a> for WaitMovingIterator<'a, T> {
    fn setup(graph: &'a Graph) -> Self {
        Self {
            iter: T::setup(graph),
            waiting_state: false,
            wait_action: None,
            _phantom: std::marker::PhantomData,
        }
    }

    fn prepare_from_proto(&mut self, proto: ProtoIterator, state: &State) -> &mut Self {
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
            self.iter.prepare_from_proto(proto, state);
        }
        self
    }
}

/// An action iterator that wraps around another action iterator and eliminates actions according
/// to the "components on the way" condition.
///
/// If an energizable component (i.e., in beta_1) that is on the way is skipped in an action, it
/// will be eliminated.
pub struct OnWayIterator<'a, T: ActionIterator<'a>> {
    /// Underlying iterator.
    iter: T,
    /// For each path i to j, there's an entry for the list of components on that path in ascending
    /// order.
    on_way: Array2<Vec<Index>>,
    /// The set of buses in beta_1
    energizable_buses: Vec<Index>,
    /// Node (bus or initial position) at which each team is located, represented by its index.
    /// usize;:MAX if en-route.
    team_nodes: Vec<Index>,
    /// This struct semantically stores a reference with `'a` lifetime due to wrapped
    /// ActionIterator.
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, T: ActionIterator<'a>> Iterator for OnWayIterator<'a, T> {
    type Item = Vec<TeamAction>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(action) = self.iter.next() {
            let on_way: bool = self.team_nodes.iter().zip(action.iter()).any(|(&i, &j)| {
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

impl<'a, T: ActionIterator<'a>> ActionIterator<'a> for OnWayIterator<'a, T> {
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
            iter: T::setup(graph),
            on_way,
            energizable_buses: Vec::default(),
            team_nodes: Vec::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    #[inline]
    fn prepare_from_proto(&mut self, proto: ProtoIterator, state: &State) -> &mut Self {
        self.team_nodes = proto.team_nodes.clone();
        self.energizable_buses = proto.energizable_buses.clone();
        self.iter.prepare_from_proto(proto, state);
        self
    }
}
