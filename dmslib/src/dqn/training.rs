use crate::teams;

use super::exploration::EvaluationResult;
use super::*;

pub trait DqnTrainer {
    /// Train the model for the given number of iterations and return the average loss.
    fn train(&mut self, iterations: usize) -> f64;

    /// Evaluate the model by generating a full policy and computing its value.
    ///
    /// Horizon should come from the teams::Config provided in `new` method.
    fn evaluate(&mut self) -> EvaluationResult;
}

mod classic;
pub use self::classic::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum TrainerSettings {
    NaiveClassic(ClassicTrainerSettings),
}

impl TrainerSettings {
    pub fn build<'a>(
        self,
        graph: &'a Graph,
        teams: Vec<TeamState>,
        model_settings: ModelSettings,
        config: teams::Config,
        device: tch::Device,
    ) -> Box<dyn DqnTrainer + 'a> {
        match self {
            TrainerSettings::NaiveClassic(settings) => Box::new(NaiveClassicTrainer::new(
                graph,
                teams,
                model_settings,
                settings,
                config,
                device,
            )),
        }
    }
}
