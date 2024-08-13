use crate::ALLOCATOR;

use super::*;

pub struct ExploreResult<TT: Transition> {
    pub bus_states: Array2<BusState>,
    pub transitions: Vec<Vec<Vec<TT>>>,
    pub max_memory: usize,
}

/// Generic trait for the functions that explore the actions of a given state.
pub trait Explorer<'a, TT: Transition> {
    /// Explore the possible states starting from the given team state.
    fn explore<AA: ActionApplier<TT>>(graph: &'a Graph) -> ExploreResult<TT> {
        Self::memory_limited_explore::<AA>(graph, usize::MAX).unwrap()
    }

    /// Explore the possible states starting from the given team state.
    ///
    /// When the memory usage reported by global allocator exceeds the limit,
    /// [`SolveFailure::OutOfMemory`] will be returned;
    fn memory_limited_explore<AA: ActionApplier<TT>>(
        graph: &'a Graph,
        memory_limit: usize,
    ) -> Result<ExploreResult<TT>, SolveFailure>;
}

mod naive;
pub use naive::NaiveExplorer;
