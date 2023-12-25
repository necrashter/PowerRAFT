//! Experience Replay Memory module for Deep Q-Learning.
use std::collections::VecDeque;

use rand::rngs::StdRng;
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

/// Settings struct for ReplayMemory.
#[derive(Serialize, Deserialize, Debug)]
pub struct ReplayMemorySettings {
    pub capacity: usize,
    pub init: usize,
    pub minibatch: usize,
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

/// Represents a minibatch sample from [`ReplayMemory`].
pub struct ExperienceSample {
    /// Current states.
    /// [B, N] where B is the minibatch size and N is the input size.
    pub states: Tensor,
    /// Represents index of the taken action for each state.
    /// Datatype is i64 unlike other fields.
    /// [B, 1] where B is the minibatch size.
    pub actions: Tensor,
    /// Cost of the taken action for each state.
    /// [B, 1] where B is the minibatch size.
    pub costs: Tensor,
    /// Successor states for each taken action.
    /// [B, S, N] where S is the number of successors and N is the input size.
    pub successors: Tensor,
    /// Action filters for each successor state.
    /// [B, S, M] where S is the number of successors and M is the output size.
    pub action_filters: Tensor,
    /// Probability of each successor state.
    /// [B, S] where B is the minibatch size and S is the number of successors.
    pub probabilities: Tensor,
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

    /// Sample a batch from the ReplayMemory.
    pub fn sample_batch(&mut self, size: usize) -> ExperienceSample {
        let mut states: Vec<Tensor> = Vec::with_capacity(size);
        let mut actions: Vec<i64> = Vec::with_capacity(size);
        let mut costs: Vec<f32> = Vec::with_capacity(size);
        // TODO: Not all successors have the same size
        let mut successors: Vec<Tensor> = Vec::with_capacity(size);
        let mut action_filters: Vec<Tensor> = Vec::with_capacity(size);
        let mut probabilities: Vec<Tensor> = Vec::with_capacity(size);

        for i in rand::seq::index::sample(&mut self.rng, self.experiences.len(), size) {
            let experience = &self.experiences[i];
            states.push(experience.state.shallow_clone());
            actions.push(experience.action);
            costs.push(experience.cost as f32);
            successors.push(experience.successors.shallow_clone());
            action_filters.push(experience.action_filters.shallow_clone());
            probabilities.push(experience.probabilities.shallow_clone());
        }

        ExperienceSample {
            states: Tensor::stack(&states, 0),
            actions: Tensor::from_slice(&actions)
                .unsqueeze(1)
                .to_device(self.device),
            costs: Tensor::from_slice(&costs)
                .unsqueeze(1)
                .to_device(self.device),
            successors: Tensor::stack(&successors, 0),
            action_filters: Tensor::stack(&action_filters, 0),
            probabilities: Tensor::stack(&probabilities, 0),
        }
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
