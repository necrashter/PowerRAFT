use crate::webclient;

use itertools::Itertools;
use ndarray::{Array1, Array2, ArrayView1};
use std::collections::HashMap;
use std::time::Instant;

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

pub type Index = usize;
pub type Time = usize;

/// Contains information about the distribution system.
pub struct Graph {
    /// Travel times between each edge.
    travel_times: Array2<Time>,
    /// Adjacency list for branch connections.
    branches: Vec<Vec<Index>>,
    /// True if a bus at given index is directly connected to energy resource.
    connected: Vec<bool>,
    /// Probability of failures.
    pfs: Array1<f64>,
}

/// State of a single team. Use a `Vec` to represent multiple teams.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum TeamState {
    OnBus(Index),
    EnRoute(Index, Index, Time),
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BusState {
    Damaged = -1,
    Unknown = 0,
    Energized = 1,
}

#[derive(PartialEq, Eq, Clone)]
pub struct State {
    buses: Vec<BusState>,
    teams: Vec<TeamState>,
}

/// Represents the actions of a single team.
/// Wait: -1 (WAIT_ACTION constant), Continue: -2 (CONTINUE_ACTION constant), Move: index of the bus.
pub type TeamAction = isize;
pub const WAIT_ACTION: isize = -1;
pub const CONTINUE_ACTION: isize = -2;

/// Performs recursive energization with given team and bus state on the given graph.
/// Outcomes are a list of probability and bus state pairs.
fn recursive_energization(
    graph: &Graph,
    teams: &[TeamState],
    buses: Vec<BusState>,
) -> Vec<(f64, Vec<BusState>)> {
    // Buses on which a team is present
    let team_buses: Vec<usize> = teams
        .iter()
        .filter_map(|team| match team {
            TeamState::OnBus(i) => Some(*i),
            TeamState::EnRoute(_, _, _) => None,
        })
        .unique()
        .collect();
    // All energization outcomes with probability.
    let mut outcomes: Vec<(f64, Vec<BusState>)> = Vec::new();
    // Recursive energization process
    let mut queue: Vec<(f64, Vec<BusState>)> = vec![(1.0, buses)];
    while let Some(next) = queue.pop() {
        let (p, mut state) = next;
        // Alpha as defined in paper
        let alpha: Vec<usize> = team_buses
            .clone()
            .into_iter()
            .filter(|i| {
                let i = *i;
                state[i] == BusState::Unknown && {
                    graph.connected[i]
                        || graph.branches[i]
                            .iter()
                            .any(|j| state[*j] == BusState::Energized)
                }
            })
            .collect();
        if alpha.is_empty() {
            outcomes.push((p, state));
            continue;
        }

        for &i in &alpha {
            state[i] = BusState::Damaged;
        }
        'permutations: loop {
            let p = alpha.iter().fold(p, |acc, &i| {
                let pf = graph.pfs[i];
                acc * if state[i] == BusState::Damaged {
                    pf
                } else {
                    1.0 - pf
                }
            });
            queue.push((p, state.clone()));
            for &i in &alpha {
                if state[i] == BusState::Damaged {
                    state[i] = BusState::Energized;
                    continue 'permutations;
                } else {
                    state[i] = BusState::Damaged;
                }
            }
            break 'permutations;
        }
    }
    outcomes
}

impl State {
    /// Creates the starting state from given team configuration.
    fn start_state(graph: &Graph, teams: Vec<TeamState>) -> State {
        State {
            buses: vec![BusState::Unknown; graph.connected.len()],
            teams,
        }
    }

    /// Applies the given action to this state, returns the outcomes in a pair as follows:
    /// - `Vec<TeamState>`: The resulting state of teams (note that team transitions are
    /// deterministic).
    /// - `Vec<(f64, Vec<BusState>)>`: Resulting bus states alongside their probabilities.
    fn apply_action(
        &self,
        graph: &Graph,
        actions: &Vec<TeamAction>,
    ) -> (Vec<TeamState>, Vec<(f64, Vec<BusState>)>) {
        debug_assert_eq!(actions.len(), self.teams.len());
        // New team state
        let teams: Vec<TeamState> = self
            .teams
            .iter()
            .zip(actions.iter())
            .map(|(team, action)| {
                let team = team.clone();
                let action = *action;
                match team {
                    TeamState::OnBus(source) => {
                        if action == WAIT_ACTION {
                            TeamState::OnBus(source)
                        } else {
                            debug_assert!(action != CONTINUE_ACTION);
                            let dest = action as usize;
                            let travel_time = graph.travel_times[(source, dest)];
                            if travel_time == 1 {
                                TeamState::OnBus(dest)
                            } else {
                                TeamState::EnRoute(source, dest, 1)
                            }
                        }
                    }
                    TeamState::EnRoute(source, dest, t) => {
                        debug_assert!(action == CONTINUE_ACTION);
                        let travel_time = graph.travel_times[(source, dest)];
                        if travel_time - t == 1 {
                            TeamState::OnBus(dest)
                        } else {
                            TeamState::EnRoute(source, dest, t + 1)
                        }
                    }
                }
            })
            .collect();
        let outcomes = recursive_energization(graph, &teams, self.buses.clone());
        (teams, outcomes)
    }

    /// Attempt to energize without moving the teams.
    fn energize(&self, graph: &Graph) -> Option<Vec<(f64, Vec<BusState>)>> {
        let outcomes = recursive_energization(graph, &self.teams, self.buses.clone());
        if outcomes.len() == 1 {
            // No energizations happened
            debug_assert_eq!(outcomes[0].0, 1.0);
            debug_assert_eq!(outcomes[0].1, self.buses);
            None
        } else {
            Some(outcomes)
        }
    }

    /// Cost function: the count of unenergized (damaged or unknown) buses.
    fn get_cost(&self) -> f64 {
        self.buses
            .iter()
            .filter(|&b| *b != BusState::Energized)
            .count() as f64
    }

    fn is_terminal(&self, graph: &Graph) -> bool {
        !self.buses.iter().enumerate().any(|(i, bus)| {
            if *bus != BusState::Unknown {
                return false;
            }
            if graph.connected[i] {
                return true;
            }
            for &j in graph.branches[i].iter() {
                if self.buses[j] == BusState::Energized {
                    return true;
                }
            }
            false
        })
    }
}

#[derive(PartialEq, Debug)]
enum TeamActionState {
    OnUnknownBus,
    OnKnownBus,
    EnRoute,
}

/// An iterator for feasible action set.
pub struct ActionIterator {
    /// Set of buses with Unknown status.
    unknown_buses: Vec<usize>,
    /// `bus_energizable[i]` is `true` if `unknown_buses[i]` is bus_energizable, i.e.,
    /// connected to an energy source or an energized bus.
    /// Beta(s) from paper.
    bus_energizable: Vec<bool>,
    /// State of the teams
    team_states: Vec<TeamActionState>,
    /// Bus at which each team is located, represented as index in unknown_buses.
    /// usize;:MAX if en-route or not in unknown_buses.
    team_buses: Vec<usize>,
    /// True if the progress condition is satisfied by an en-route team.
    progress_satisfied: bool,
    /// Next action
    next: Option<Vec<TeamAction>>,
}

impl State {
    /// Returns an iterator to applicable and feasible actions in this state.
    /// A(s) in paper.
    pub fn actions(&self, graph: &Graph) -> ActionIterator {
        let unknown_buses: Vec<usize> = self
            .buses
            .iter()
            .enumerate()
            .filter_map(|(i, bus)| {
                if bus == &BusState::Unknown {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        let bus_energizable: Vec<bool> = unknown_buses
            .iter()
            .map(|&busi| {
                let i = busi as usize;
                if graph.connected[i] {
                    return true;
                }
                for &j in graph.branches[i].iter() {
                    if self.buses[j] == BusState::Energized {
                        return true;
                    }
                }
                false
            })
            .collect();
        let team_states: Vec<TeamActionState> = self
            .teams
            .iter()
            .map(|team| match team {
                TeamState::OnBus(i) => {
                    if self.buses[*i] == BusState::Unknown {
                        TeamActionState::OnUnknownBus
                    } else {
                        TeamActionState::OnKnownBus
                    }
                }
                TeamState::EnRoute(_, _, _) => TeamActionState::EnRoute,
            })
            .collect();
        let team_buses: Vec<usize> = self
            .teams
            .iter()
            .map(|team| match team {
                TeamState::OnBus(i) => match unknown_buses.binary_search(i) {
                    Ok(j) => j,
                    Err(_) => usize::MAX,
                },
                TeamState::EnRoute(_, _, _) => usize::MAX,
            })
            .collect();
        let energizable_buses: Vec<usize> = unknown_buses
            .iter()
            .zip(bus_energizable.iter())
            .filter_map(|(i, e)| if *e { Some(*i as usize) } else { None })
            .collect();
        let progress_satisfied = self.teams.iter().any(|team| {
            if let TeamState::EnRoute(_, b, _) = team {
                energizable_buses.binary_search(b).is_ok()
            } else {
                false
            }
        });
        let mut it = ActionIterator {
            unknown_buses,
            bus_energizable,
            team_states,
            team_buses,
            next: None,
            progress_satisfied,
        };
        it.reset();
        it
    }
}

impl ActionIterator {
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
                action[i] += 1;
            }
            if (action[i] as usize) < self.unknown_buses.len() {
                return Some(action);
            } else {
                action[i] = if self.team_states[i] == TeamActionState::OnUnknownBus {
                    WAIT_ACTION
                } else if self.team_buses[i] == 0 {
                    debug_assert!(1 < self.unknown_buses.len());
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
                .any(|&i| i >= 0 && self.bus_energizable[i as usize])
    }
}

impl Iterator for ActionIterator {
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
                        self.unknown_buses[i as usize] as isize
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

/// Hash is implemented for index lookup for a given state.
impl std::hash::Hash for State {
    fn hash<H: std::hash::Hasher>(&self, hash_state: &mut H) {
        // We don't hash bus/team size because it will be the same in a given HashMap
        for bus in self.buses.iter() {
            let i = match bus {
                BusState::Damaged => -1,
                BusState::Unknown => 0,
                BusState::Energized => 1,
            };
            i.hash(hash_state);
        }
        for t in self.teams.iter() {
            match t {
                TeamState::OnBus(i) => {
                    0.hash(hash_state);
                    i.hash(hash_state);
                }
                TeamState::EnRoute(i, j, k) => {
                    1.hash(hash_state);
                    i.hash(hash_state);
                    j.hash(hash_state);
                    k.hash(hash_state);
                }
            }
        }
    }
}

/// Represents a possible transition as a result of an action.
pub struct Transition {
    /// Index of the successor state.
    successor: usize,
    /// Probability of this transition.
    /// The probabilities of all transitions of an action should add up to 1.
    p: f64,
    /// Cost that incurs when this transition is taken.
    cost: f64,
}

/// Convert webclient types to teams problem.
pub fn solve(graph: webclient::Graph, teams: Vec<webclient::Team>) -> Result<Solution, String> {
    let start_time = Instant::now();

    let bus_count = graph.nodes.len();
    let team_count = teams.len();

    let mut locations: Vec<webclient::LatLng> =
        graph.nodes.iter().map(|node| node.latlng.clone()).collect();
    let pfs: Array1<f64> = graph.nodes.iter().map(|node| node.pf).collect();

    for (i, team) in teams.iter().enumerate() {
        if team.index.is_none() && team.latlng.is_none() {
            return Err(format!("Team {i} has neither index nor latlng!"));
        }
    }

    for res in graph.resources.iter() {
        if res.kind.is_some() {
            return Err(String::from(
                "Only transmission grid is supported for teams!",
            ));
        }
    }

    let teams_state: Vec<TeamState> = teams
        .into_iter()
        .map(|t| {
            if let Some(i) = t.index {
                TeamState::OnBus(i)
            } else {
                let i = locations.len();
                // We did error checking above
                locations.push(t.latlng.as_ref().unwrap().clone());
                TeamState::OnBus(i)
            }
        })
        .collect();

    let lnodes = locations.len();
    let mut travel_times = Array2::<Time>::zeros((lnodes, lnodes));

    for (i1, l1) in locations.iter().enumerate() {
        for (i2, l2) in locations.iter().enumerate().skip(i1 + 1) {
            let time = l1.distance_to(l2).ceil() as Time;
            travel_times[(i1, i2)] = time;
            travel_times[(i2, i1)] = time;
        }
    }

    let mut branches = vec![Vec::new(); graph.nodes.len()];

    for branch in graph.branches.iter() {
        let a = branch.nodes.0;
        let b = branch.nodes.1;
        // TODO: throw error on duplicate branch?
        branches[a].push(b);
        branches[b].push(a);
    }

    let mut connected: Vec<bool> = vec![false; graph.nodes.len()];

    for x in graph.external.iter() {
        connected[x.node] = true;
    }

    let mut solgen = SolutionGenerator::new(Graph {
        travel_times,
        branches,
        connected,
        pfs,
    });
    let generation_start_time = Instant::now();
    solgen.explore(teams_state);
    let generation_time: f64 = generation_start_time.elapsed().as_secs_f64();
    let (values, policy) = solgen.synthesize_policy();

    let mut team_nodes = Array2::<f64>::zeros((locations.len(), 2));
    for (i, location) in locations.into_iter().enumerate() {
        team_nodes[(i, 0)] = location.0;
        team_nodes[(i, 1)] = location.1;
    }

    let state_count = solgen.states.len();
    let states: Array1<BusState> = solgen
        .states
        .iter()
        .flat_map(|state| &state.buses)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    let states: Array2<BusState> = match states.into_shape((state_count, bus_count)) {
        Ok(x) => x,
        Err(e) => {
            return Err(format!("Cannot reshape bus state array: {e}"));
        }
    };
    let teams: Array1<TeamState> = solgen
        .states
        .iter()
        .flat_map(|state| &state.teams)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    let teams: Array2<TeamState> = match teams.into_shape((state_count, team_count)) {
        Ok(x) => x,
        Err(e) => {
            return Err(format!("Cannot reshape team state array: {e}"));
        }
    };
    let transitions = solgen.transitions;
    let travel_times = solgen.graph.travel_times;

    let total_time: f64 = start_time.elapsed().as_secs_f64();

    Ok(Solution {
        total_time,
        generation_time,
        team_nodes,
        travel_times,
        states,
        teams,
        transitions,
        values,
        policy,
    })
}

/// A struct that contains the solution to a team-based restoration problem.
/// First run `explore` and then `synthesize_policy`.
struct SolutionGenerator {
    graph: Graph,
    /// TODO: store 2 ndarrays, for team and bus states
    /// There will be less indirection.
    states: Vec<State>,
    /// Reverse index
    state_to_index: HashMap<State, usize>,
    transitions: Vec<Vec<Vec<Transition>>>,
}

impl SolutionGenerator {
    /// New solution structure from graph.
    pub fn new(graph: Graph) -> SolutionGenerator {
        SolutionGenerator {
            graph,
            states: Vec::new(),
            state_to_index: HashMap::new(),
            transitions: Vec::new(),
        }
    }

    /// Explore the possible states starting from the given team state.
    fn explore(&mut self, teams: Vec<TeamState>) {
        let mut index = self.index_state(&State::start_state(&self.graph, teams));
        while index < self.states.len() {
            let state = self.states[index].clone();
            let cost = state.get_cost();
            let action_transitions: Vec<Vec<Transition>> = if state.is_terminal(&self.graph) {
                vec![vec![Transition {
                    successor: index,
                    p: 1.0,
                    cost,
                }]]
            } else if let Some(bus_outcomes) = state.energize(&self.graph) {
                assert!(
                    index == 0,
                    "Energization succeeded at the start of a non-initial state"
                );
                vec![bus_outcomes
                    .into_iter()
                    .map(|(p, bus_state)| {
                        let successor_state = State {
                            teams: state.teams.clone(),
                            buses: bus_state,
                        };
                        let successor_index = self.index_state(&successor_state);
                        Transition {
                            successor: successor_index,
                            p,
                            cost: 0.0,
                        }
                    })
                    .collect()]
            } else {
                state
                    .actions(&self.graph)
                    .map(|action| {
                        let (team_outcome, bus_outcomes) = state.apply_action(&self.graph, &action);
                        bus_outcomes
                            .into_iter()
                            .map(|(p, bus_state)| {
                                let successor_state = State {
                                    teams: team_outcome.clone(),
                                    buses: bus_state,
                                };
                                let successor_index = self.index_state(&successor_state);
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
            self.transitions.push(action_transitions);
            index += 1;
        }
    }

    /// Get the index of given state, adding it to the hasmap when necessary.
    fn index_state(&mut self, s: &State) -> usize {
        match self.state_to_index.get(s) {
            Some(i) => *i,
            None => {
                let i = self.states.len();
                self.states.push(s.clone());
                self.state_to_index.insert(s.clone(), i);
                i
            }
        }
    }

    /// Synthesize a policy, an array containing the index of optimal actions in each state.
    /// Must be run after running `explore`, i.e., state space must not be empty.
    ///
    /// Returns a pair containing action values and index of optimal action in each state.
    fn synthesize_policy(&mut self) -> (Vec<Vec<f64>>, Vec<usize>) {
        assert!(
            !self.states.is_empty(),
            "States must be non-empty during policy synthesis"
        );
        let mut values: Array1<f64> = Array1::zeros(self.states.len());
        const OPTIMIZATION_HORIZON: usize = 30;
        for _ in 1..OPTIMIZATION_HORIZON {
            let prev_val = values;
            values = Array1::zeros(self.states.len());
            for (i, action) in self.transitions.iter().enumerate() {
                let optimal_value: f64 = action
                    .iter()
                    .map(|transitions| {
                        transitions
                            .iter()
                            .map(|t| t.p * (t.cost + prev_val[t.successor]))
                            .sum()
                    })
                    .min_by(|a: &f64, b| {
                        a.partial_cmp(b)
                            .expect("Transition values must be comparable in value iteration")
                    })
                    .expect("No actions in a state");
                values[i] = optimal_value;
            }
        }

        let mut state_action_values: Vec<Vec<f64>> = Vec::new();
        state_action_values.reserve(self.states.len());
        let mut policy: Vec<usize> = vec![0; self.states.len()];

        let prev_val = values;
        for (i, action) in self.transitions.iter().enumerate() {
            let action_values: Vec<f64> = action
                .iter()
                .map(|transitions| {
                    transitions
                        .iter()
                        .map(|t| t.p * (t.cost + prev_val[t.successor]))
                        .sum()
                })
                .collect();
            let optimal_action = action_values
                .iter()
                .enumerate()
                .min_by(|a: &(usize, &f64), b: &(usize, &f64)| {
                    a.1.partial_cmp(b.1)
                        .expect("Transition values must be comparable in value iteration")
                })
                .expect("No actions in a state")
                .0;
            state_action_values.push(action_values);
            policy[i] = optimal_action;
        }
        (state_action_values, policy)
    }
}

pub struct Solution {
    /// Total time to generate the complete solution in seconds.
    pub total_time: f64,
    /// Total time to generate the MDP without policy synthesis in seconds.
    pub generation_time: f64,

    /// Latitude and longtitude values of vertices in team graph.
    pub team_nodes: Array2<f64>,
    /// Travel time between each node
    pub travel_times: Array2<Time>,

    /// Array of bus states.
    pub states: Array2<BusState>,
    /// Array of team states.
    pub teams: Array2<TeamState>,
    /// Array of actions for each state, each entry containing a list of transitions
    /// This has to be triple Vec because each state has arbitrary number of actions and each
    /// action has arbitrary number of transitions.
    pub transitions: Vec<Vec<Vec<Transition>>>,

    /// Value function for each action.
    pub values: Vec<Vec<f64>>,
    /// Index of optimal actions in each state.
    pub policy: Vec<usize>,
}

impl Serialize for Solution {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(8))?;
        map.serialize_entry("totalTime", &self.total_time)?;
        map.serialize_entry("generationTime", &self.generation_time)?;

        map.serialize_entry("teamNodes", &Array2Serializer(&self.team_nodes))?;
        map.serialize_entry("travelTimes", &Array2Serializer(&self.travel_times))?;

        map.serialize_entry("states", &Array2Serializer(&self.states))?;
        map.serialize_entry("teams", &Array2Serializer(&self.teams))?;
        map.serialize_entry("transitions", &self.transitions)?;

        map.serialize_entry("values", &self.values)?;
        map.serialize_entry("policy", &self.policy)?;
        map.end()
    }
}

/// Private helper for 2D array serialization.
/// Array is serialized as list of lists.
struct Array2Serializer<'a, T>(&'a Array2<T>);

/// Private helper for 2D array serialization.
/// This is a row in array.
struct ArrayRowSerializer<'a, T>(ArrayView1<'a, T>);

impl<'a, T: Serialize> Serialize for Array2Serializer<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.shape()[0]))?;
        for row in self.0.rows() {
            seq.serialize_element(&ArrayRowSerializer(row))?;
        }
        seq.end()
    }
}

impl<'a, T: Serialize> Serialize for ArrayRowSerializer<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for i in self.0.iter() {
            seq.serialize_element(i)?;
        }
        seq.end()
    }
}

impl Serialize for TeamState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TeamState::OnBus(a) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("node", a)?;
                map.end()
            }
            TeamState::EnRoute(a, b, t) => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("node", a)?;
                map.serialize_entry("target", b)?;
                map.serialize_entry("time", t)?;
                // TODO: travel time
                map.end()
            }
        }
    }
}

impl Serialize for BusState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BusState::Damaged => serializer.serialize_str("D"),
            BusState::Unknown => serializer.serialize_str("U"),
            BusState::Energized => serializer.serialize_str("TG"),
        }
    }
}

impl Serialize for Transition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(4))?;
        seq.serialize_element(&self.successor)?;
        seq.serialize_element(&self.p)?;
        seq.serialize_element(&self.cost)?;
        seq.serialize_element(&1)?;
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_paper_example_graph() -> Graph {
        Graph {
            travel_times: ndarray::arr2(&[
                [0, 1, 2, 1, 2, 2],
                [1, 0, 1, 2, 2, 2],
                [2, 1, 0, 2, 2, 1],
                [1, 2, 2, 0, 1, 2],
                [2, 2, 2, 1, 0, 1],
                [2, 2, 1, 2, 1, 0],
            ]),
            branches: vec![vec![1], vec![0, 2], vec![1], vec![4], vec![3, 5], vec![4]],
            connected: vec![true, false, false, true, false, false],
            pfs: ndarray::arr1(&[0.5, 0.5, 0.25, 0.25, 0.25, 0.25]),
        }
    }

    fn check_sets<T: PartialEq>(output: &Vec<T>, expected: &Vec<T>) {
        assert_eq!(output.len(), expected.len());
        for a in expected {
            assert!(output.contains(a));
        }
    }

    #[test]
    fn paper_example_4_1_1() {
        let graph = get_paper_example_graph();
        let buses: Vec<BusState> = vec![
            BusState::Energized,
            BusState::Unknown,
            BusState::Unknown,
            BusState::Energized,
            BusState::Damaged,
            BusState::Unknown,
        ];
        let teams: Vec<TeamState> = vec![TeamState::OnBus(0), TeamState::EnRoute(4, 2, 1)];
        let state = State { buses, teams };

        assert_eq!(state.get_cost(), 4.0);

        let actions: Vec<_> = state.actions(&graph).collect();

        assert_eq!(actions, vec![vec![1, -2]]);
    }

    #[test]
    fn on_energized_bus_actions() {
        let graph = get_paper_example_graph();
        let buses: Vec<BusState> = vec![
            BusState::Energized,
            BusState::Unknown,
            BusState::Unknown,
            BusState::Energized,
            BusState::Unknown,
            BusState::Unknown,
        ];
        let teams: Vec<TeamState> = vec![TeamState::OnBus(0), TeamState::OnBus(3)];
        let state = State { buses, teams };

        assert_eq!(state.get_cost(), 4.0);

        let actions: Vec<_> = state.actions(&graph).collect();
        let expected_actions: Vec<Vec<TeamAction>> = vec![
            vec![1, 1],
            vec![1, 2],
            vec![1, 4],
            vec![1, 5],
            vec![4, 1],
            vec![4, 2],
            vec![4, 4],
            vec![4, 5],
            vec![2, 1],
            vec![2, 4],
            vec![5, 1],
            vec![5, 4],
        ];
        check_sets(&actions, &expected_actions);
    }
}
