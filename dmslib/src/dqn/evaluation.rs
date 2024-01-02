use crate::{
    io::GenericTeamSolution,
    teams::{
        state::{State, StateIndexer},
        Solution,
    },
    types::StateIndex,
};

use super::{environment::Tensorizer, *};

use itertools::Itertools;
use tch::IndexOp;

pub struct DqnExplorer<'a, 'b, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> {
    /// Action iterator.
    iterator: AI,
    /// Reference to a graph.
    graph: &'a Graph,

    // Note that these need a different lifetime because graph outlives them.
    model: &'b Model,
    tensorizer: &'b mut Tensorizer,

    /// State indexer.
    states: SI,
    /// 3D vector of transitions:
    /// - `transitions[i]`: Actions of state i
    /// - `transitions[i][j]`: Transitions of action j in state i
    transitions: Vec<Vec<Vec<TT>>>,

    qvals: Vec<f64>,

    /// How many actions to select from the network in each state.
    top_k: usize,
}

impl<'a, 'b, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> DqnExplorer<'a, 'b, TT, AI, SI> {
    /// Explore the actions and transitions of the initial state.
    ///
    /// Energization without team movement is allowed to succeed only in the initial state.
    /// Panics if this happens in other states.
    #[inline]
    fn explore_state<AA: ActionApplier<TT>>(&mut self, input: (usize, State)) {
        let (index, state) = input;
        let cost = state.get_cost();
        let action_transitions: Vec<Vec<TT>> = if state.is_terminal(self.graph) {
            vec![vec![TT::terminal_transition(index as StateIndex, cost)]]
        } else if let Some(bus_outcomes) = state.energize(self.graph) {
            assert_eq!(
                index, 0,
                "Energization succeeded at the start of a non-initial state"
            );
            vec![bus_outcomes
                .into_iter()
                .map(|(p, bus_state)| {
                    let successor_state = State {
                        teams: state.teams.clone(),
                        buses: bus_state,
                    };
                    let successor_index = self.states.index_state(successor_state);
                    TT::time1_transition(successor_index as StateIndex, cost, p)
                })
                .collect()]
        } else {
            // Input tensor for the model.
            let device = self.model.vs.device();
            let input = self.tensorizer.state_to_tensor(&state).to_device(device);

            // Get the output from the model.
            let output = self.model.forward(&input);

            // Compute the valid actions in this state.
            let state = state.to_action_state(self.graph);
            let mut actions = self
                .iterator
                .prepare(&state)
                .map(|action| {
                    // Add the network's output Q-value to each action.
                    let i = self.tensorizer.action_to_number(&action) as i64;
                    let qval = output.i(i).double_value(&[]);
                    self.qvals.push(qval);
                    (qval, action)
                })
                .collect_vec();

            // Sory by Q-values
            actions.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            actions
                .into_iter()
                .take(self.top_k)
                .map(|(_qval, action)| {
                    AA::apply(&state, cost, self.graph, &action)
                        .into_iter()
                        .map(|(mut transition, successor_state)| {
                            // Index the successor states
                            let successor_index = self.states.index_state(successor_state);
                            transition.set_successor(successor_index as StateIndex);
                            transition
                        })
                        .collect()
                })
                .collect()
        };
        if self.transitions.len() <= index {
            self.transitions.resize_with(index + 1, Default::default);
        }
        self.transitions[index] = action_transitions;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct EvaluationSettings {
    /// How many actions to select from the network in each state.
    pub top_k: usize,
}

impl Default for EvaluationSettings {
    fn default() -> Self {
        EvaluationSettings { top_k: 1 }
    }
}

pub struct EvaluationResult {
    pub value: Value,
    pub avg_q: f64,
    pub states: usize,
}

pub fn dqn_evaluate_custom<
    'a,
    'b,
    TT: Transition + 'static,
    AI: ActionSet<'a>,
    SI: StateIndexer,
    AA: ActionApplier<TT>,
    PS: PolicySynthesizer<TT>,
>(
    graph: &'a Graph,
    teams: Vec<TeamState>,
    model: &'b Model,
    tensorizer: &'b mut Tensorizer,
    horizon: usize,
    settings: EvaluationSettings,
) -> (EvaluationResult, GenericTeamSolution) {
    let EvaluationSettings { top_k } = settings;

    // Don't calculate gradients
    let _guard = tch::no_grad_guard();

    let mut explorer = DqnExplorer {
        iterator: AI::setup(graph),
        graph,
        states: SI::new(graph, &teams),
        transitions: Vec::new(),
        model,
        tensorizer,
        qvals: Vec::new(),
        top_k,
    };
    explorer
        .states
        .index_state(State::start_state(graph, teams));

    while let Some(i) = explorer.states.next() {
        explorer.explore_state::<AA>(i);
    }

    let qvals_len = explorer.qvals.len();
    let avg_q = explorer.qvals.into_iter().sum::<f64>() / (qvals_len as f64);
    let num_states = explorer.states.get_state_count();

    let (states, teams) = explorer.states.deconstruct();
    let transitions = explorer.transitions;

    let (values, policy) = PS::synthesize_policy(&transitions, horizon);
    let value = get_min_value(&values);

    let result = EvaluationResult {
        value,
        avg_q,
        states: num_states,
    };

    let solution = Solution {
        total_time: 0.0,
        generation_time: 0.0,
        max_memory: 0,
        states,
        teams,
        transitions,
        values,
        policy,
        horizon,
    };

    let solution = solution.into_io(graph);

    (result, solution.into())
}
