use super::*;

/// Model architecture information.
/// TODO
#[derive(Serialize, Deserialize, Debug)]
pub struct ModelSettings {}

type ModelFunc = Box<dyn Fn(&Tensor) -> Tensor>;

pub struct Model {
    pub vs: nn::VarStore,
    pub func: ModelFunc,
}

impl Model {
    pub fn new(device: tch::Device, input_size: i64, output_size: i64) -> Self {
        let vs = nn::VarStore::new(device);
        let root = &vs.root();
        let seq = nn::seq()
            .add(nn::linear(
                root / "l1",
                input_size,
                8096,
                Default::default(),
            ))
            .add_fn(|xs| xs.relu())
            .add(nn::linear(
                root / "l3",
                8096,
                output_size,
                Default::default(),
            ));
        let func = Box::new(move |xs: &Tensor| xs.apply(&seq));
        Self { vs, func }
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
