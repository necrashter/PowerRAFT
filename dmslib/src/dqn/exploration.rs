use crate::{
    teams::state::{State, StateIndexer},
    types::StateIndex,
};

use super::{environment::Tensorizer, *};

use itertools::Itertools;

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

            // Compute valid actions in this state.
            let state = state.to_action_state(self.graph);
            let actions = self.iterator.prepare(&state).collect_vec();
            let action_filter = self
                .tensorizer
                .action_filter(&actions, f32::INFINITY, 0.0)
                .to_device(device);

            // Get the output from the model.
            let mut output = self.model.forward(&input);
            // Filter invalid actions
            output += action_filter;
            let qval = output.min_dim(0, false).0.double_value(&[]);
            self.qvals.push(qval);

            // Get the valid action with minimum value
            let best_action = output.argmin(0, false).int64_value(&[]);
            let action = actions
                .into_iter()
                .find(|action| self.tensorizer.action_to_number(action) as i64 == best_action)
                .unwrap();

            vec![AA::apply(&state, cost, self.graph, &action)
                .into_iter()
                .map(|(mut transition, successor_state)| {
                    // Index the successor states
                    let successor_index = self.states.index_state(successor_state);
                    transition.set_successor(successor_index as StateIndex);
                    transition
                })
                .collect()]
        };
        if self.transitions.len() <= index {
            self.transitions.resize_with(index + 1, Default::default);
        }
        self.transitions[index] = action_transitions;
    }
}

pub struct EvaluationResult {
    pub value: Value,
    pub avg_q: f64,
    pub states: usize,
}

pub fn dqn_evaluate<
    'a,
    'b,
    TT: Transition,
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
) -> EvaluationResult {
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
    };
    explorer
        .states
        .index_state(State::start_state(graph, teams));

    while let Some(i) = explorer.states.next() {
        explorer.explore_state::<AA>(i);
    }

    let qvals_len = explorer.qvals.len();
    let avg_q = explorer.qvals.into_iter().sum::<f64>() / (qvals_len as f64);
    let states = explorer.states.get_state_count();

    let transitions = explorer.transitions;

    let (values, _policy) = PS::synthesize_policy(&transitions, horizon);
    let value = get_min_value(&values);

    EvaluationResult {
        value,
        avg_q,
        states,
    }
}
