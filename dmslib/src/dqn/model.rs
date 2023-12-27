use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum ModelLayer {
    Linear(i64),
    Relu,
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

type ModelFunc = Box<dyn Fn(&Tensor) -> Tensor>;

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
                let func: ModelFunc = if *normalize_advantages {
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
                Self { vs, func }
            }
        }
    }

    /// A forward pass on the network. Syntactic sugar for calling the `func` field.
    #[inline(always)]
    pub fn forward(&self, input: &Tensor) -> Tensor {
        (self.func)(input)
    }

    /// Copies parameters from another model.
    /// Panics on error (mismatch).
    pub fn copy_from(&mut self, other: &Model) {
        self.vs.copy(&other.vs).expect("Cannot copy from model");
    }
}
