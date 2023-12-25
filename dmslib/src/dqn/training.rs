use crate::teams;

use super::*;

pub trait DqnTrainer {
    /// Train the model for the given number of iterations and return the average loss.
    fn train(&mut self, iterations: usize) -> f64;

    /// Evaluate the model by generating a full policy and computing its value.
    ///
    /// Horizon should come from the teams::Config provided in `new` method.
    fn evaluate(&mut self) -> Value;
}

mod classic;
pub use self::classic::*;

mod overfit;
pub use self::overfit::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum TrainerSettings {
    NaiveClassic(ClassicTrainerSettings),
    NaiveOverfit(OverfitTrainerSettings),
}

impl TrainerSettings {
    pub fn build<'a>(
        self,
        graph: &'a Graph,
        teams: Vec<TeamState>,
        model_settings: ModelSettings,
        config: teams::Config,
    ) -> Box<dyn DqnTrainer + 'a> {
        match self {
            TrainerSettings::NaiveClassic(settings) => Box::new(NaiveClassicTrainer::new(
                graph,
                teams,
                model_settings,
                settings,
                config,
            )),
            TrainerSettings::NaiveOverfit(settings) => Box::new(NaiveOverfitTrainer::new(
                graph,
                teams,
                model_settings,
                settings,
                config,
            )),
        }
    }
}
