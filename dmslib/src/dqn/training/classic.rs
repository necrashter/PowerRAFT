use tch::IndexOp;

use super::*;
use crate::{
    dqn::{
        evaluation::{dqn_evaluate_custom, EvaluationResult},
        replay::{Experience, ReplayMemorySettings},
    },
    policy::{NaiveTimedPolicySynthesizer, TimedTransition},
    teams::{
        transitions::{TimeUntilArrival, TimedActionApplier},
        FilterOnWay, PermutationalActions,
    },
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ClassicTrainerSettings {
    pub replay: ReplayMemorySettings,
    pub lr: f64,
    pub gradient_clip: Option<f64>,
    pub epsilon: f32,
    pub target_update_period: usize,
    /// Discount factor in the Q function.
    pub discount: f64,
}

/// Implements the classic Deep Q-Learning algorithm with experience replay, as described in:
/// "Playing Atari with Deep Reinforcement Learning", 2013.
pub struct ClassicTrainer<'a, AI, TT, AA, PS, SI>
where
    AI: ActionSet<'a>,
    TT: Transition,
    AA: ActionApplier<TT>,
    PS: PolicySynthesizer<TT>,
    SI: StateIndexer,
{
    /// Replay Memory.
    mem: ReplayMemory,
    /// Simulation environment for settings.
    env: Environment<'a, AI>,
    /// Random number generator.
    rng: StdRng,
    /// Device.
    device: tch::Device,

    /// Primary model. This model will be trained.
    model: Model,
    /// Optimizer for the primary model.
    opt: nn::Optimizer,
    /// The target model that will be updated occasionally from the primary model.
    target_model: Model,
    /// Iterations since the last target model update.
    iters_target_update: usize,

    settings: ClassicTrainerSettings,
    /// Optimization horizon for the evaluate method.
    horizon: usize,

    _phantom: std::marker::PhantomData<(&'a (), TT, AA, PS, SI)>,
}
impl<'a, AI, TT, AA, PS, SI> ClassicTrainer<'a, AI, TT, AA, PS, SI>
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
        model_settings: ModelSettings,
        settings: ClassicTrainerSettings,
        config: teams::Config,
        device: tch::Device,
    ) -> Self {
        let horizon = if let Some(value) = config.horizon {
            value
        } else {
            panic!("Optimization horizon must be specified in the configuration.");
        };

        let mut mem = ReplayMemory::new(settings.replay.capacity, device);
        let mut env = Environment::<AI>::new::<TT, AA>(graph, teams);
        let rng = create_rng();

        mem.fill::<AI, TT, AA>(settings.replay.init, &mut env);

        let model = Model::new(
            device,
            env.tensorizer.input_size,
            env.tensorizer.output_size,
            &model_settings,
        );
        // The target model that will be updated occasionally from the primary model.
        let mut target_model = Model::new(
            device,
            env.tensorizer.input_size,
            env.tensorizer.output_size,
            &model_settings,
        );
        target_model.copy_from(&model);
        // Optimizer
        let opt = nn::Adam::default().build(&model.vs, settings.lr).unwrap();

        ClassicTrainer {
            mem,
            env,
            rng,
            device,
            model,
            opt,
            target_model,
            iters_target_update: 0,
            settings,
            horizon,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, AI, TT, AA, PS, SI> DqnTrainer for ClassicTrainer<'a, AI, TT, AA, PS, SI>
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
            // Select an action with epsilon-greedy
            let experience = if self.rng.gen::<f32>() <= self.settings.epsilon {
                self.env.take_random_action::<TT, AA>()
            } else {
                let action = tch::no_grad(|| {
                    // convert to input
                    let input = self.env.current_state_to_tensor().to_device(self.device);
                    // Pass through the model
                    let mut output = self.model.forward(&input);
                    // Filter invalid actions
                    output += self.env.action_filter().to_device(self.device);
                    // Get the valid action with minimum value
                    output.argmin(0, false).int64_value(&[])
                });
                let action_index = self.env.action_number_to_index(action);
                self.env.take_action_by_index::<TT, AA>(action_index)
            };
            // Record the experience.
            self.mem.add(experience);

            // Sample an experience
            let Experience {
                state,
                action,
                cost,
                time,
                successors,
                action_filters,
                probabilities,
            } = self.mem.sample_experience();
            let predicted_value = self.model.forward(&state).i(action);

            let expected_value = tch::no_grad(|| {
                // For successor count S, and input size N,
                // successors: [S, N]
                let mut x = self.target_model.forward(&successors);
                // x: [S, M]
                // Apply the action filter, eliminate invalid actions.
                x += action_filters;
                // Get the value of the best action in each successor.
                let mut x = x.min_dim(1, false).0;
                // x: [S]
                x *= probabilities;
                let x = x.sum(None);
                // Modified value function.
                (&x * self.settings.discount.powf(time)) + (cost * time)
            });

            // Compute loss & backward step
            let loss = predicted_value.mse_loss(&expected_value, tch::Reduction::Mean);
            if let Some(clip) = self.settings.gradient_clip {
                self.opt.backward_step_clip(&loss, clip);
            } else {
                self.opt.backward_step(&loss);
            }
            losses.push(loss.double_value(&[]));

            // Update target_model periodically.
            self.iters_target_update += 1;
            if self.iters_target_update >= self.settings.target_update_period {
                self.target_model.copy_from(&self.model);
                self.iters_target_update = 0;
            }

            if self.env.is_terminal() {
                // Reset for the next episode
                self.env.reset::<TT, AA>();
            }
        }
        let average_loss: f64 = losses.into_iter().sum::<f64>() / (iterations as f64);
        average_loss
    }

    fn evaluate(&mut self, settings: EvaluationSettings) -> EvaluationResult {
        dqn_evaluate_custom::<TT, AI, SI, AA, PS>(
            self.env.graph,
            self.env.initial_state.teams.clone(),
            &self.model,
            &mut self.env.tensorizer,
            self.horizon,
            settings,
        )
    }

    fn load_checkpoint(&mut self, path: &Path) -> Result<(), tch::TchError> {
        self.model.vs.load(path)?;
        self.target_model.copy_from(&self.model);
        Ok(())
    }

    fn save_checkpoint(&mut self, path: &Path) -> Result<(), tch::TchError> {
        self.model.vs.save(path)
    }
}

pub type NaiveClassicTrainer<'a> = ClassicTrainer<
    'a,
    NaiveActions,
    RegularTransition,
    NaiveActionApplier,
    NaivePolicySynthesizer,
    BitStackStateIndexer,
>;

pub type TimedClassicTrainer<'a> = ClassicTrainer<
    'a,
    NaiveActions,
    TimedTransition,
    TimedActionApplier<TimeUntilArrival>,
    NaiveTimedPolicySynthesizer,
    BitStackStateIndexer,
>;

pub type AeClassicTrainer<'a> = ClassicTrainer<
    'a,
    FilterOnWay<'a, PermutationalActions<'a>>,
    TimedTransition,
    TimedActionApplier<TimeUntilArrival>,
    NaiveTimedPolicySynthesizer,
    BitStackStateIndexer,
>;
