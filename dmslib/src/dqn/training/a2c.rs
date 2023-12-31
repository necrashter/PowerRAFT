use super::*;
use crate::{
    dqn::evaluation::{dqn_evaluate_custom, EvaluationResult},
    policy::{NaiveTimedPolicySynthesizer, TimedTransition},
    teams::{
        transitions::{TimeUntilArrival, TimedActionApplier},
        FilterOnWay, PermutationalActions,
    },
};

#[derive(Serialize, Deserialize, Debug)]
pub struct A2cTrainerSettings {
    pub lr: f64,
    pub gradient_clip: Option<f64>,
}

/// Implements the classic Deep Q-Learning algorithm with experience replay, as described in:
/// "Playing Atari with Deep Reinforcement Learning", 2013.
pub struct A2cTrainer<'a, AI, TT, AA, PS, SI>
where
    AI: ActionSet<'a>,
    TT: Transition,
    AA: ActionApplier<TT>,
    PS: PolicySynthesizer<TT>,
    SI: StateIndexer,
{
    /// Simulation environment for settings.
    env: Environment<'a, AI>,
    /// Device.
    device: tch::Device,

    /// Primary model. This model will be trained.
    model: Model,
    /// Optimizer for the primary model.
    opt: nn::Optimizer,
    /// Iterations since the last target model update.
    settings: A2cTrainerSettings,
    /// Optimization horizon for the evaluate method.
    horizon: usize,

    _phantom: std::marker::PhantomData<(&'a (), TT, AA, PS, SI)>,
}
impl<'a, AI, TT, AA, PS, SI> A2cTrainer<'a, AI, TT, AA, PS, SI>
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
        settings: A2cTrainerSettings,
        config: teams::Config,
        device: tch::Device,
    ) -> Result<Self, String> {
        let horizon = if let Some(value) = config.horizon {
            value
        } else {
            return Err("Optimization horizon must be specified in the configuration.".to_string());
        };

        let env = Environment::<AI>::new::<TT, AA>(graph, teams);

        let model = Model::new(
            device,
            env.tensorizer.input_size,
            env.tensorizer.output_size,
            &model_settings,
        );
        if !model.is_a2c() {
            return Err("A2cTrainer requires A2C model type.".to_string());
        }
        // Optimizer
        let opt = nn::Adam::default().build(&model.vs, settings.lr).unwrap();

        Ok(A2cTrainer {
            env,
            device,
            model,
            opt,
            settings,
            horizon,
            _phantom: std::marker::PhantomData,
        })
    }
}

impl<'a, AI, TT, AA, PS, SI> DqnTrainer for A2cTrainer<'a, AI, TT, AA, PS, SI>
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
            // Convert current state to input tensor
            let input = self.env.current_state_to_tensor().to_device(self.device);
            // Pass through the model
            let (actor, critic) = tch::no_grad(|| self.model.forward_a2c(&input));
            // Filter invalid actions and get action probabilities
            let filtered = actor * self.env.action_filter(0.0, 1.0).to_device(self.device);
            let probs = filtered.softmax(-1, tch::Kind::Float);
            // Sample an action from probabilities
            let action = probs.multinomial(1, true).int64_value(&[]);
            let action_index = self.env.action_number_to_index(action);
            // Take this action
            // TODO: May need to return a different set of variables.
            // Can define a new take_action_a2c method which returns a newly defined struct.
            let _experience = self.env.take_action_by_index::<TT, AA>(action_index);

            // TODO
            // Compute loss
            let loss = critic;

            // Backward step
            if let Some(clip) = self.settings.gradient_clip {
                self.opt.backward_step_clip(&loss, clip);
            } else {
                self.opt.backward_step(&loss);
            }
            losses.push(loss.double_value(&[]));

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
        self.model.vs.load(path)
    }

    fn save_checkpoint(&mut self, path: &Path) -> Result<(), tch::TchError> {
        self.model.vs.save(path)
    }
}

pub type NaiveA2cTrainer<'a> = A2cTrainer<
    'a,
    NaiveActions,
    RegularTransition,
    NaiveActionApplier,
    NaivePolicySynthesizer,
    BitStackStateIndexer,
>;

pub type TimedA2cTrainer<'a> = A2cTrainer<
    'a,
    NaiveActions,
    TimedTransition,
    TimedActionApplier<TimeUntilArrival>,
    NaiveTimedPolicySynthesizer,
    BitStackStateIndexer,
>;

pub type AeA2cTrainer<'a> = A2cTrainer<
    'a,
    FilterOnWay<'a, PermutationalActions<'a>>,
    TimedTransition,
    TimedActionApplier<TimeUntilArrival>,
    NaiveTimedPolicySynthesizer,
    BitStackStateIndexer,
>;
