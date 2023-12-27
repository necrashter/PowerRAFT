//! Deep Q-Learning Module
use rand::{rngs::StdRng, Rng};
use serde::{Deserialize, Serialize};
use tch::{
    nn::{self, OptimizerConfig},
    Tensor,
};

use crate::{
    create_rng,
    policy::{
        get_min_value, NaivePolicySynthesizer, PolicySynthesizer, RegularTransition, Transition,
    },
    teams::{
        state::{BitStackStateIndexer, StateIndexer, TeamState},
        transitions::{ActionApplier, NaiveActionApplier},
        ActionSet, Graph, NaiveActions,
    },
    types::Value,
    RANDOM_SEED,
};

mod environment;
use self::{environment::Environment, replay::ExperienceSample};

mod replay;
use self::replay::ReplayMemory;

mod exploration;
pub use self::exploration::EvaluationResult;

mod training;
pub use self::training::{DqnTrainer, NaiveClassicTrainer, TrainerSettings};

mod model;
pub use self::model::*;

/// Seed libtorch using the [`RANDOM_SEED`] variable if present.
pub fn load_torch_seed() {
    RANDOM_SEED.with_borrow(|seed| {
        if let Some(seed) = seed {
            tch::manual_seed(*seed as i64);
        }
    });
}
