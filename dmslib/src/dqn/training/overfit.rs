use itertools::Itertools;
use ndarray::Array2;

use crate::{
    dqn::environment::Tensorizer,
    policy::determine_horizon,
    teams::{
        state::{BusState, State},
        TeamAction,
    },
    types::StateIndex,
};

use super::*;

struct CustomSolver<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> {
    /// Action iterator.
    iterator: AI,
    /// Reference to a graph.
    graph: &'a Graph,
    /// State indexer.
    states: SI,
    /// 3D vector of transitions:
    /// - `transitions[i]`: Actions of state i
    /// - `transitions[i][j]`: Transitions of action j in state i
    transitions: Vec<Vec<Vec<TT>>>,
    actions: Vec<Vec<Vec<TeamAction>>>,
    start: usize,
}

impl<'a, TT: Transition, AI: ActionSet<'a>, SI: StateIndexer> CustomSolver<'a, TT, AI, SI> {
    #[inline]
    fn explore_state<AA: ActionApplier<TT>>(&mut self, input: (usize, State)) {
        let (index, state) = input;
        let cost = state.get_cost();
        let (actions, action_transitions): (Vec<Vec<TeamAction>>, Vec<Vec<TT>>) =
            if state.is_terminal(self.graph) {
                (
                    vec![],
                    vec![vec![TT::terminal_transition(index as StateIndex, cost)]],
                )
            } else if let Some(bus_outcomes) = state.energize(self.graph) {
                assert_eq!(index, 0);
                self.start = 1;
                (
                    vec![],
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
                        .collect()],
                )
            } else {
                let state = state.to_action_state(self.graph);
                let actions = self.iterator.prepare(&state).collect_vec();
                let transitions = actions
                    .iter()
                    .map(|action| {
                        AA::apply(&state, cost, self.graph, action)
                            .into_iter()
                            .map(|(mut transition, successor_state)| {
                                // Index the successor states
                                let successor_index = self.states.index_state(successor_state);
                                transition.set_successor(successor_index as StateIndex);
                                transition
                            })
                            .collect()
                    })
                    .collect();
                (actions, transitions)
            };

        if self.transitions.len() <= index {
            self.transitions.resize_with(index + 1, Default::default);
        }
        self.transitions[index] = action_transitions;

        if self.actions.len() <= index {
            self.actions.resize_with(index + 1, Default::default);
        }
        self.actions[index] = actions;
    }

    fn solve<AA, PS>(
        graph: &'a Graph,
        teams: Vec<TeamState>,
        horizon: Option<usize>,
    ) -> ExactSolution<TT>
    where
        AA: ActionApplier<TT>,
        PS: PolicySynthesizer<TT>,
    {
        let mut explorer = CustomSolver {
            iterator: AI::setup(graph),
            graph,
            states: SI::new(graph, &teams),
            transitions: Vec::new(),
            actions: Vec::new(),
            start: 0,
        };
        explorer
            .states
            .index_state(State::start_state(graph, teams));

        while let Some(i) = explorer.states.next() {
            explorer.explore_state::<AA>(i);
        }

        let (bus_states, team_states) = explorer.states.deconstruct();
        let transitions = explorer.transitions;

        let auto_horizon = determine_horizon(&transitions);
        let horizon = if let Some(v) = horizon {
            if auto_horizon > v {
                log::warn!("Given horizon ({v}) is smaller than determined ({auto_horizon})");
            }
            v
        } else {
            log::info!("Automatically determined horizon: {auto_horizon}");
            auto_horizon
        };
        let (values, _policy) = PS::synthesize_policy(&transitions, horizon);
        log::info!(
            "Value from exact solution (horizon = {}): {}",
            horizon,
            get_min_value(&values)
        );

        ExactSolution {
            bus_states,
            team_states,
            transitions,
            actions: explorer.actions,
            values,
            start: explorer.start,
            horizon,
        }
    }
}

struct ExactSolution<TT: Transition> {
    pub bus_states: Array2<BusState>,
    pub team_states: Array2<TeamState>,
    /// 3D vector of transitions:
    /// - `transitions[i]`: Actions of state i
    /// - `transitions[i][j]`: Transitions of action j in state i
    transitions: Vec<Vec<Vec<TT>>>,
    actions: Vec<Vec<Vec<TeamAction>>>,
    values: Vec<Vec<Value>>,
    start: usize,
    horizon: usize,
}

impl<TT: Transition> ExactSolution<TT> {
    fn get_state(&self, index: usize) -> State {
        State {
            buses: self.bus_states.row(index).to_vec(),
            teams: self.team_states.row(index).to_vec(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OverfitTrainerSettings {
    pub lr: f64,
    pub gradient_clip: Option<f64>,
}

/// A "cheating" trainer that generates the exact solution first, and then tries to train the
/// network so that it replicates the Q function of the exact solution.
///
/// This is useless in practical applications, but implemented here to ensure that:
/// - The deep-learning model works correctly.
/// - The model can overfit to the Q function (hence the name `OverfitTrainer`).
pub struct OverfitTrainer<'a, AI, TT, AA, PS, SI>
where
    AI: ActionSet<'a>,
    TT: Transition,
    AA: ActionApplier<TT>,
    PS: PolicySynthesizer<TT>,
    SI: StateIndexer,
{
    solution: ExactSolution<TT>,
    tensorizer: Tensorizer,
    graph: &'a Graph,
    initial_state: Vec<TeamState>,
    /// Random number generator.
    rng: StdRng,
    /// Device.
    device: tch::Device,

    /// Primary model. This model will be trained.
    model: Model,
    /// Optimizer for the primary model.
    opt: nn::Optimizer,

    gradient_clip: Option<f64>,

    output_vec: Vec<f32>,

    _phantom: std::marker::PhantomData<(&'a (), AI, TT, AA, PS, SI)>,
}

impl<'a, AI, TT, AA, PS, SI> OverfitTrainer<'a, AI, TT, AA, PS, SI>
where
    AI: ActionSet<'a>,
    TT: Transition,
    AA: ActionApplier<TT>,
    PS: PolicySynthesizer<TT>,
    SI: StateIndexer,
{
    pub fn new(
        graph: &'a Graph,
        teams: Vec<TeamState>,
        _model_settings: ModelSettings,
        settings: OverfitTrainerSettings,
        config: teams::Config,
    ) -> Self {
        let device = tch::Device::cuda_if_available();

        let solution =
            CustomSolver::<TT, AI, SI>::solve::<AA, PS>(graph, teams.clone(), config.horizon);
        let tensorizer = Tensorizer::new(graph, teams.len());

        let rng = create_rng();

        let model = Model::new(device, tensorizer.input_size, tensorizer.output_size);
        // Optimizer
        let opt = nn::Adam::default().build(&model.vs, settings.lr).unwrap();

        let output_vec = vec![0.0; tensorizer.output_size as usize];

        OverfitTrainer {
            solution,
            tensorizer,
            graph,
            initial_state: teams,
            rng,
            device,
            model,
            opt,
            gradient_clip: settings.gradient_clip,
            output_vec,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, AI, TT, AA, PS, SI> DqnTrainer for OverfitTrainer<'a, AI, TT, AA, PS, SI>
where
    AI: ActionSet<'a>,
    TT: Transition,
    AA: ActionApplier<TT>,
    PS: PolicySynthesizer<TT>,
    SI: StateIndexer,
{
    /// Train the model for the given number of iterations and return the average loss.
    fn train(&mut self, iterations: usize) -> f64 {
        let mut losses: Vec<f64> = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            // Sample an action
            let index = self
                .rng
                .gen_range(self.solution.start..self.solution.transitions.len());
            let state = self.solution.get_state(index);
            let actions = &self.solution.actions[index];
            let values = &self.solution.values[index];

            let state = self
                .tensorizer
                .state_to_tensor(&state)
                .to_device(self.device);
            let action_filter = self
                .tensorizer
                .action_filter(actions, 0.0, 1.0)
                .to_device(self.device);

            let predicted_values = self.model.forward(&state) * action_filter;

            let expected_values = {
                self.output_vec.fill(0.0);
                for (action, &value) in actions.iter().zip(values.iter()) {
                    self.output_vec[self.tensorizer.action_to_number(action)] = value;
                }
                Tensor::from_slice(&self.output_vec).to_device(self.device)
            };

            // Compute loss & backward step
            let loss = predicted_values.mse_loss(&expected_values, tch::Reduction::Mean);
            if let Some(clip) = self.gradient_clip {
                self.opt.backward_step_clip(&loss, clip);
            } else {
                self.opt.backward_step(&loss);
            }
            losses.push(loss.double_value(&[]));
        }
        let average_loss: f64 = losses.into_iter().sum::<f64>() / (iterations as f64);
        average_loss
    }

    fn evaluate(&mut self) -> Value {
        let ExploreResult {
            bus_states: _,
            team_states: _,
            transitions,
            max_memory: _,
        } = dqn_explore::<TT, AI, SI, AA>(
            self.graph,
            self.initial_state.clone(),
            &self.model,
            &mut self.tensorizer,
        );
        let (values, _policy) = PS::synthesize_policy(&transitions, self.solution.horizon);
        get_min_value(&values)
    }
}

pub type NaiveOverfitTrainer<'a> = OverfitTrainer<
    'a,
    NaiveActions,
    RegularTransition,
    NaiveActionApplier,
    NaivePolicySynthesizer,
    BitStackStateIndexer,
>;
