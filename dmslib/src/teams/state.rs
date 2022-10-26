use super::*;

/// State of a single team. Use a `Vec` to represent multiple teams.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum TeamState {
    OnBus(Index),
    EnRoute(Index, Index, Time),
}

impl Default for TeamState {
    fn default() -> Self {
        TeamState::OnBus(usize::MAX)
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BusState {
    Damaged = -1,
    Unknown = 0,
    Energized = 1,
}

impl Default for BusState {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Eq, Clone)]
pub struct State {
    pub buses: Vec<BusState>,
    pub teams: Vec<TeamState>,
}

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
    pub fn start_state(graph: &Graph, teams: Vec<TeamState>) -> State {
        State {
            buses: vec![BusState::Unknown; graph.connected.len()],
            teams,
        }
    }

    /// Applies the given action to this state, returns the outcomes in a pair as follows:
    /// - `Vec<TeamState>`: The resulting state of teams (note that team transitions are
    /// deterministic).
    /// - `Vec<(f64, Vec<BusState>)>`: Resulting bus states alongside their probabilities.
    pub fn apply_action(
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
    pub fn energize(&self, graph: &Graph) -> Option<Vec<(f64, Vec<BusState>)>> {
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
    pub fn get_cost(&self) -> f64 {
        self.buses
            .iter()
            .filter(|&b| *b != BusState::Energized)
            .count() as f64
    }

    pub fn is_terminal(&self, graph: &Graph) -> bool {
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

impl State {
    /// Returns a vector such that the value at index i contains:
    /// 1. If the status of bus at index i is unknown,
    ///    a. the smallest j value such that bus at index i is in beta_j(s)
    ///    b. usize::MAX if there's no such j
    /// 2. 0 if the status of bus at index i is energized or damaged.
    ///
    /// For each bus, minbeta array holds the number of energizations required
    /// to energize that bus. By traversing the graph starting from immediately
    /// energizable buses, we determine minbeta values and hence unreachable buses,
    /// for which minbeta = infinity.
    #[inline]
    pub fn minbetas(&self, graph: &Graph) -> Vec<Index> {
        let mut minbeta: Vec<Index> = self
            .buses
            .iter()
            .enumerate()
            .map(|(i, bus)| {
                if bus != &BusState::Unknown {
                    return 0;
                }
                if graph.connected[i] {
                    return 1;
                }
                for &j in graph.branches[i].iter() {
                    if self.buses[j] == BusState::Energized {
                        return 1;
                    }
                }
                usize::MAX
            })
            .collect();
        {
            // Determine the remaining beta values
            let mut deque: VecDeque<Index> = minbeta
                .iter()
                .enumerate()
                .filter_map(|(i, &beta)| if beta == 1 { Some(i) } else { None })
                .collect();
            while let Some(i) = deque.pop_front() {
                let next_beta: Index = minbeta[i] + 1;
                for &j in graph.branches[i].iter() {
                    if next_beta < minbeta[j] {
                        minbeta[j] = next_beta;
                        deque.push_back(j);
                    }
                }
            }
        }
        minbeta
    }

    /// Returns an iterator to applicable and feasible actions in this state.
    /// A(s) in paper.
    #[inline]
    pub fn actions<T: ActionIterator>(&self, graph: &Graph) -> T {
        T::from_state(self, graph)
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        let buses_len = self.buses.len();
        let teams_len = self.teams.len();
        assert_eq!(
            buses_len,
            other.buses.len(),
            "Equality is undefined for states of different systems."
        );
        assert_eq!(
            teams_len,
            other.teams.len(),
            "Equality is undefined for states of different systems."
        );
        for i in 0..buses_len {
            if self.buses[i] != other.buses[i] {
                return false;
            }
        }
        for i in 0..teams_len {
            if self.teams[i] != other.teams[i] {
                return false;
            }
        }
        true
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
    pub successor: usize,
    /// Probability of this transition.
    /// The probabilities of all transitions of an action should add up to 1.
    pub p: f64,
    /// Cost that incurs when this transition is taken.
    pub cost: f64,
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
