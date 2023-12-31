use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum ModelLayer {
    Linear(i64),
    Relu,
    Sigmoid,
    Sgn,
    Silu,
}

fn build_seq(root: &nn::Path, layers: &[ModelLayer], input_size: i64) -> (nn::Sequential, i64) {
    let mut last_size = input_size;
    let mut seq = nn::seq();
    for (i, layer) in layers.iter().enumerate() {
        match layer {
            ModelLayer::Linear(size) => {
                let size = *size;
                seq = seq.add(nn::linear(
                    root / format!("layer{i}"),
                    last_size,
                    size,
                    Default::default(),
                ));
                last_size = size;
            }
            ModelLayer::Relu => {
                seq = seq.add_fn(|xs| xs.relu());
            }
            ModelLayer::Sigmoid => {
                seq = seq.add_fn(|xs| xs.sigmoid());
            }
            ModelLayer::Sgn => {
                seq = seq.add_fn(|xs| xs.sgn());
            }
            ModelLayer::Silu => {
                seq = seq.add_fn(|xs| xs.silu());
            }
        }
    }
    (seq, last_size)
}

/// Model architecture information.
#[derive(Serialize, Deserialize, Debug)]
pub enum ModelSettings {
    /// Classic DQN architecture that outputs the Q value for each action.
    Dqn {
        #[serde(default)]
        layers: Vec<ModelLayer>,
    },
    /// Dueling variant of the DQN from [this paper](https://arxiv.org/abs/1511.06581).
    /// It's the same as DQN when viewed as a black-box system, but internally it
    /// estimates V (value) and A (advantage) separately and combines them to output Q.
    DuelingDqn {
        #[serde(default)]
        layers: Vec<ModelLayer>,
        normalize_advantages: bool,
    },
}

pub type DqnFunc = Box<dyn Fn(&Tensor) -> Tensor>;
pub type A2cFunc = Box<dyn Fn(&Tensor) -> (Tensor, Tensor)>;

pub enum ModelFunc {
    /// DQN model type. Returns a Q-value for each possible action.
    Dqn(DqnFunc),
    /// A2C model type. Returns two tensors:
    /// 1. Actor: Preference for each action.
    /// 2. Critic: Expected cost/reward.
    A2c(A2cFunc),
}

pub struct Model {
    pub vs: nn::VarStore,
    pub func: ModelFunc,
}

impl Model {
    pub fn new(
        device: tch::Device,
        input_size: i64,
        output_size: i64,
        model: &ModelSettings,
    ) -> Self {
        let vs = nn::VarStore::new(device);
        let root = &vs.root();
        match model {
            ModelSettings::Dqn { layers } => {
                let (mut seq, last_size) = build_seq(root, layers, input_size);
                // Add the final output layer
                seq = seq.add(nn::linear(
                    root / "output",
                    last_size,
                    output_size,
                    Default::default(),
                ));
                let func = Box::new(move |xs: &Tensor| xs.apply(&seq));
                let func = ModelFunc::Dqn(func);
                Self { vs, func }
            }
            ModelSettings::DuelingDqn {
                layers,
                normalize_advantages,
            } => {
                let (seq, last_size) = build_seq(root, layers, input_size);
                // The final value and advantage layers.
                let value_layer = nn::linear(root / "value", last_size, 1, Default::default());
                let advantage_layer = nn::linear(
                    root / "advantage",
                    last_size,
                    output_size,
                    Default::default(),
                );
                let func: DqnFunc = if *normalize_advantages {
                    Box::new(move |xs: &Tensor| {
                        let zs = xs.apply(&seq);
                        let advantages = zs.apply(&advantage_layer);
                        // We need to use min because we want to minimize cost.
                        let min_advantage = advantages.min_dim(-1, true).0;
                        // Advantage of the best action is now 0, and other actions have
                        // negative advantage. This will ensure that the Q value for the
                        // best action equals V.
                        (min_advantage - advantages) + zs.apply(&value_layer)
                    })
                } else {
                    Box::new(move |xs: &Tensor| {
                        let zs = xs.apply(&seq);
                        zs.apply(&advantage_layer) + zs.apply(&value_layer)
                    })
                };
                let func = ModelFunc::Dqn(func);
                Self { vs, func }
            }
        }
    }

    /// A forward pass on a DQN. Panics if the model type doesn't match.
    pub fn forward_dqn(&self, input: &Tensor) -> Tensor {
        if let ModelFunc::Dqn(func) = &self.func {
            func(input)
        } else {
            panic!("Mismatched model type (expected DQN)")
        }
    }

    /// Returns true if the model type is DQN.
    pub fn is_dqn(&self) -> bool {
        matches!(&self.func, ModelFunc::Dqn(_))
    }

    /// Returns true if the model type is A2C.
    pub fn is_a2c(&self) -> bool {
        matches!(&self.func, ModelFunc::A2c(_))
    }

    /// Returns a tensor of action preferences. Higher actions are preferred.
    /// For DQN, this is negative Q-value (since Q-value is the expected cost).
    /// For A2C, this is the action probabilities.
    pub fn get_action_preferences(&self, input: &Tensor) -> Tensor {
        match &self.func {
            ModelFunc::Dqn(func) => func(input) * -1.0,
            ModelFunc::A2c(func) => func(input).0,
        }
    }

    /// Copies parameters from another model.
    /// Panics on error (mismatch).
    pub fn copy_from(&mut self, other: &Model) {
        self.vs.copy(&other.vs).expect("Cannot copy from model");
    }
}
