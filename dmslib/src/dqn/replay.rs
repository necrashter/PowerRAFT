//! Experience Replay Memory module for Deep Q-Learning.
use std::collections::VecDeque;

use rand::{rngs::StdRng, Rng};
use serde::{Deserialize, Serialize};
use tch::Tensor;

use crate::{
    create_rng,
    policy::Transition,
    teams::{transitions::ActionApplier, ActionSet},
    types::Cost,
};

use super::environment::Environment;

/// Represents a single instance in ReplayMemory.
pub struct Experience {
    /// Current state in the experience.
    /// [N] where N is the input size.
    pub state: Tensor,
    /// Represents index of the taken action.
    /// Has to be i64 because of Torch.
    pub action: i64,
    /// Cost of the taken action.
    pub cost: Cost,
    /// Successor states for the taken action.
    /// [S, N] where S is the number of successors and N is the input size.
    pub successors: Tensor,
    /// Action filters for each successor state.
    /// [S, M] where S is the number of successors and M is the output size.
    pub action_filters: Tensor,
    /// Probability for each successor state.
    /// [S] where S is the number of successors.
    pub probabilities: Tensor,
}

impl Experience {
    pub fn shallow_clone(&self) -> Experience {
        Experience {
            state: self.state.shallow_clone(),
            action: self.action,
            cost: self.cost,
            successors: self.successors.shallow_clone(),
            action_filters: self.action_filters.shallow_clone(),
            probabilities: self.probabilities.shallow_clone(),
        }
    }
}

/// Settings struct for ReplayMemory.
#[derive(Serialize, Deserialize, Debug)]
pub struct ReplayMemorySettings {
    pub capacity: usize,
    pub init: usize,
}

/// Experience Replay Memory for Deep Q-Learning.
pub struct ReplayMemory {
    /// Cyclic buffer of stored experiences..
    experiences: VecDeque<Experience>,
    /// Random number generator.
    rng: StdRng,
    /// Device.
    device: tch::Device,
}

impl ReplayMemory {
    pub fn new(capacity: usize, device: tch::Device) -> Self {
        let rng = create_rng();
        Self {
            experiences: VecDeque::with_capacity(capacity),
            rng,
            device,
        }
    }

    /// Add a new experience, replacing the oldest one if the capacity is full.
    pub fn add(&mut self, mut experience: Experience) {
        if self.experiences.len() == self.experiences.capacity() {
            self.experiences.pop_front();
        }
        // Move to device before adding.
        experience.state = experience.state.to_device(self.device);
        experience.successors = experience.successors.to_device(self.device);
        experience.action_filters = experience.action_filters.to_device(self.device);
        experience.probabilities = experience.probabilities.to_device(self.device);
        self.experiences.push_back(experience);
    }

    /// Sample an experience from the ReplayMemory.
    pub fn sample_experience(&mut self) -> Experience {
        let i = self.rng.gen_range(0..self.experiences.len());
        self.experiences[i].shallow_clone()
    }

    /// Generate experiences from the environment by taking random actions.
    pub fn fill<'a, AI: ActionSet<'a>, TT: Transition, AA: ActionApplier<TT>>(
        &mut self,
        count: usize,
        env: &mut Environment<'a, AI>,
    ) {
        for _ in 0..count {
            let experience = env.take_random_action::<TT, AA>();
            self.add(experience);
            if env.is_terminal() {
                env.reset::<TT, AA>();
            }
        }
    }
}
