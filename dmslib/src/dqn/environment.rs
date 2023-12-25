//! Environment module for DQN.
use crate::{
    create_rng,
    policy::Transition,
    teams::{
        state::{State, TeamState},
        transitions::ActionApplier,
        ActionSet, Graph, TeamAction,
    },
    types::{Cost, Probability, Time},
};

use rand::{rngs::StdRng, seq::SliceRandom, Rng};
use tch::Tensor;

use super::replay::Experience;

/// State transition for the `Environment`.
/// This needs to be separate because Environment doesn't use state indices.
#[derive(Clone, PartialEq, Debug)]
struct EnvTransition {
    /// The successor state.
    pub successor: State,
    /// Probability of this transition.
    /// The probabilities of all transitions of an action should add up to 1.
    pub p: Probability,
    /// Cost that incurs when this transition is taken.
    pub cost: Cost,
    /// Passed time when this transition is taken.
    pub time: Time,
}

impl EnvTransition {
    #[inline]
    fn terminal_transition(successor: State, cost: Cost) -> Self {
        Self {
            successor,
            p: 1.0,
            cost,
            time: 1,
        }
    }
}

/// Struct for converting states/actions to input/output tensors.
pub struct Tensorizer {
    bus_count: usize,
    /// Time dimensions for each team. Max travel time + 1 (for time = 0).
    time_dims: usize,
    /// Each bus has 3 states, each team can be in one of the buses and one of the time dims.
    /// Note that Torch requires i64 for Tensor sizes.
    pub input_size: i64,
    /// Maximum number of actions.
    /// Note that Torch requires i64 for Tensor sizes.
    pub output_size: i64,

    /// Temporary Vec for storing the data representing the input tensor.
    input_vec: Vec<f32>,
    /// Temporary Vec for storing the data representing the output tensor.
    output_vec: Vec<f32>,
}

impl Tensorizer {
    pub fn new(graph: &Graph, team_count: usize) -> Self {
        let bus_count = graph.pfs.len();
        // Time dimensions for each team. Max travel time + 1 (for time = 0).
        let time_dims = *graph.travel_times.iter().max().unwrap() as usize + 1;
        // Each bus has 3 states, each team can be in one of the buses and one of the time dims.
        let input_size = bus_count * 3 + team_count * (bus_count + time_dims);
        // Maximum number of actions.
        let output_size = bus_count.pow(team_count as u32);
        // Note that Torch requires i64
        let input_size: i64 = input_size.try_into().unwrap();
        let output_size: i64 = output_size.try_into().unwrap();

        let input_vec = vec![0.0; input_size as usize];
        let output_vec = vec![0.0; output_size as usize];
        Tensorizer {
            bus_count,
            time_dims,
            input_size,
            output_size,
            input_vec,
            output_vec,
        }
    }

    /// Convert the given state to an input tensor.
    pub fn state_to_tensor(&mut self, state: &State) -> Tensor {
        self.input_vec.fill(0.0);
        let mut index = 0;
        for state in state.buses.iter() {
            let state = *state as usize;
            self.input_vec[index + state] = 1.0;
            index += 3;
        }
        for team in state.teams.iter() {
            self.input_vec[index + team.index as usize] = 1.0;
            index += self.bus_count;
            self.input_vec[index + team.time as usize] = 1.0;
            index += self.time_dims;
        }
        debug_assert_eq!(index, self.input_size as usize);
        Tensor::from_slice(&self.input_vec)
    }

    /// Create a filter tensor for the given actions.
    pub fn action_filter(
        &mut self,
        actions: &[Vec<TeamAction>],
        invalid: f32,
        valid: f32,
    ) -> Tensor {
        self.output_vec.fill(invalid);
        for action in actions {
            let index = self.action_to_number(action);
            self.output_vec[index] = valid;
        }
        Tensor::from_slice(&self.output_vec)
    }

    /// Convert the given action to its index in the network.
    pub fn action_to_number(&self, action: &[TeamAction]) -> usize {
        let mut out: usize = 0;
        for (i, team) in action.iter().enumerate() {
            out += (*team as usize) * self.bus_count.pow(i as u32);
        }
        out
    }
}

/// Environment simulation for Deep Q-Learning.
///
/// This class will use the thread local RANDOM_SEED variable if it's not None.
pub struct Environment<'a, AI: ActionSet<'a>> {
    /// Action iterator.
    iterator: AI,
    /// Reference to a graph.
    pub graph: &'a Graph,

    /// Initial state.
    pub initial_state: State,
    /// Current state.
    state: State,
    /// Current state.
    actions: Vec<Vec<TeamAction>>,
    /// Transitions of current state:
    /// - `transitions`: Actions in current state.
    /// - `transitions[i]`: Transitions of action i.
    transitions: Vec<Vec<EnvTransition>>,

    /// Random number generator.
    rng: StdRng,

    pub tensorizer: Tensorizer,
}

impl<'a, AI: ActionSet<'a>> Environment<'a, AI> {
    /// Construct a new environment.
    /// This will use the thread local RANDOM_SEED variable if it's not None.
    pub fn new<TT: Transition, AA: ActionApplier<TT>>(
        graph: &'a Graph,
        teams: Vec<TeamState>,
    ) -> Self {
        let rng = create_rng();

        let tensorizer = Tensorizer::new(graph, teams.len());
        let initial_state = State::start_state(graph, teams);
        let state = initial_state.clone();

        let mut out = Environment {
            iterator: AI::setup(graph),
            graph,
            initial_state,
            state,
            actions: Vec::new(),
            transitions: Vec::new(),
            rng,
            tensorizer,
        };

        out.prepare_current_state::<TT, AA>();

        out
    }

    /// Updates the actions and transitions after a state update.
    ///
    /// If energization without movement succeeds in the given state, one of the outcomes
    /// will be selected.
    fn prepare_current_state<TT: Transition, AA: ActionApplier<TT>>(&mut self) {
        // Check energization.
        if let Some(bus_outcomes) = self.state.energize(self.graph) {
            // Energization succeeded, select one of the successors.
            let outcomes: Vec<_> = bus_outcomes
                .into_iter()
                .map(|(p, bus_state)| {
                    let successor_state = State {
                        teams: self.state.teams.clone(),
                        buses: bus_state,
                    };
                    (successor_state, p)
                })
                .collect();
            let outcome = outcomes
                .choose_weighted(&mut self.rng, |item| item.1)
                .unwrap();
            self.state = outcome.0.clone();
        }
        // Generate transitions.
        let cost = self.state.get_cost();
        if self.state.is_terminal(self.graph) {
            self.actions = vec![self.state.teams.iter().map(|team| team.index).collect()];
            self.transitions = vec![vec![EnvTransition::terminal_transition(
                self.state.clone(),
                cost,
            )]];
        } else {
            let state = self.state.clone().to_action_state(self.graph);
            self.actions = self.iterator.prepare(&state).collect();
            self.transitions = self
                .actions
                .iter()
                .map(|action| {
                    AA::apply(&state, cost, self.graph, action)
                        .into_iter()
                        .map(|(transition, successor)| EnvTransition {
                            successor,
                            p: transition.get_probability(),
                            cost: transition.get_cost(),
                            time: transition.get_time(),
                        })
                        .collect()
                })
                .collect();
        }
    }

    /// Reset the environment with the given initial state.
    ///
    /// If energization without movement succeeds in the given state, one of the outcomes
    /// will be selected.
    pub fn reset_with_state<TT: Transition, AA: ActionApplier<TT>>(&mut self, state: State) {
        self.state = state;
        self.prepare_current_state::<TT, AA>();
    }

    /// Reset the environment with the default initial state.
    ///
    /// If energization without movement succeeds in the given state, one of the outcomes
    /// will be selected.
    pub fn reset<TT: Transition, AA: ActionApplier<TT>>(&mut self) {
        self.reset_with_state::<TT, AA>(self.initial_state.clone());
    }

    /// Apply the action with the given index.
    pub fn take_action_by_index<TT: Transition, AA: ActionApplier<TT>>(
        &mut self,
        action_index: usize,
    ) -> Experience {
        let state = self.current_state_to_tensor();
        let cost = self.state.get_cost();
        let action = self
            .tensorizer
            .action_to_number(&self.actions[action_index]) as i64;

        let (probabilities, (successors, action_filters)): (Vec<_>, (Vec<_>, Vec<_>)) = self
            .transitions[action_index]
            .iter()
            .map(|transition| {
                let successor = self.tensorizer.state_to_tensor(&transition.successor);

                let state = &transition.successor;
                let actions: Vec<_> = if state.is_terminal(self.graph) {
                    // Action iterator cannot handle terminal states.
                    vec![state.teams.iter().map(|team| team.index).collect()]
                } else {
                    let state = state.clone().to_action_state(self.graph);
                    self.iterator.prepare(&state).collect()
                };
                let filter = self.tensorizer.action_filter(&actions, f32::INFINITY, 0.0);

                (transition.p, (successor, filter))
            })
            .unzip();

        let successors = Tensor::stack(&successors, 0);
        let action_filters = Tensor::stack(&action_filters, 0);
        let probabilities = Tensor::from_slice(&probabilities);

        let transition = self.transitions[action_index]
            .choose_weighted(&mut self.rng, |t| t.p)
            .unwrap();
        self.state = transition.successor.clone();
        self.prepare_current_state::<TT, AA>();

        Experience {
            state,
            action,
            cost,
            successors,
            action_filters,
            probabilities,
        }
    }

    pub fn take_random_action<TT: Transition, AA: ActionApplier<TT>>(&mut self) -> Experience {
        let action = self.rng.gen_range(0..self.actions.len());
        self.take_action_by_index::<TT, AA>(action)
    }

    pub fn current_state_to_tensor(&mut self) -> Tensor {
        self.tensorizer.state_to_tensor(&self.state)
    }

    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal(self.graph)
    }

    /// Create a filter tensor for the actions in the current state.
    /// Valid actions will be marked by 0.0 in the Tensor.
    /// Invalid actions will be marked by infinity.
    pub fn action_filter(&mut self) -> Tensor {
        self.tensorizer
            .action_filter(&self.actions, f32::INFINITY, 0.0)
    }

    /// Convert action number (its index in the network's output) to
    /// its index in the current action list.
    ///
    /// Panics if not found.
    pub fn action_number_to_index(&self, action: i64) -> usize {
        for (i, a) in self.actions.iter().enumerate() {
            if self.tensorizer.action_to_number(a) as i64 == action {
                return i;
            }
        }
        panic!("Given action is not found in the current state of Environment");
    }
}
