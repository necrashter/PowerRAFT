use std::path::Path;

use crate::teams;

use super::evaluation::EvaluationResult;
use super::*;

pub trait DqnTrainer {
    /// Train the model for the given number of iterations and return the average loss.
    fn train(&mut self, iterations: usize) -> f64;

    /// Evaluate the model by generating a full policy and computing its value.
    ///
    /// Horizon should come from the teams::Config provided in `new` method.
    ///
    /// `top_k` defines how many actions to select from the network in each state.
    fn evaluate(&mut self, settings: EvaluationSettings) -> EvaluationResult;

    /// Load the model from the given checkpoint.
    fn load_checkpoint(&mut self, path: &Path) -> Result<(), tch::TchError>;
    /// Save the model into the given checkpoint file.
    fn save_checkpoint(&mut self, path: &Path) -> Result<(), tch::TchError>;
}

mod classic;
pub use self::classic::*;

mod a2c;
pub use self::a2c::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum TrainerSettings {
    NaiveClassic(ClassicTrainerSettings),
    TimedClassic(ClassicTrainerSettings),
    AeClassic(ClassicTrainerSettings),

    NaiveA2c(A2cTrainerSettings),
    TimedA2c(A2cTrainerSettings),
    AeA2c(A2cTrainerSettings),
}

impl TrainerSettings {
    pub fn build<'a>(
        self,
        graph: &'a Graph,
        teams: Vec<TeamState>,
        model_settings: ModelSettings,
        config: teams::Config,
        device: tch::Device,
    ) -> Result<Box<dyn DqnTrainer + 'a>, String> {
        match self {
            TrainerSettings::NaiveClassic(settings) => Ok(Box::new(NaiveClassicTrainer::new(
                graph,
                teams,
                model_settings,
                settings,
                config,
                device,
            )?)),
            TrainerSettings::TimedClassic(settings) => Ok(Box::new(TimedClassicTrainer::new(
                graph,
                teams,
                model_settings,
                settings,
                config,
                device,
            )?)),
            TrainerSettings::AeClassic(settings) => Ok(Box::new(AeClassicTrainer::new(
                graph,
                teams,
                model_settings,
                settings,
                config,
                device,
            )?)),
            TrainerSettings::NaiveA2c(settings) => Ok(Box::new(NaiveA2cTrainer::new(
                graph,
                teams,
                model_settings,
                settings,
                config,
                device,
            )?)),
            TrainerSettings::TimedA2c(settings) => Ok(Box::new(TimedA2cTrainer::new(
                graph,
                teams,
                model_settings,
                settings,
                config,
                device,
            )?)),
            TrainerSettings::AeA2c(settings) => Ok(Box::new(AeA2cTrainer::new(
                graph,
                teams,
                model_settings,
                settings,
                config,
                device,
            )?)),
        }
    }
}
