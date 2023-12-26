use super::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum ModelLayer {
    Linear(i64),
    Relu,
}

/// Model architecture information.
#[derive(Serialize, Deserialize, Debug)]
pub enum ModelSettings {
    Sequential(Vec<ModelLayer>),
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
            ModelSettings::Sequential(layers) => {
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
