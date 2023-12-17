use crate::ALLOCATOR;

use super::*;

pub struct ExploreResult<TT: Transition> {
    pub bus_states: Array2<BusState>,
    pub team_states: Array2<TeamState>,
    pub transitions: Vec<Vec<Vec<TT>>>,
    pub max_memory: usize,
}

/// Generic trait for the functions that explore the actions of a given state.
pub trait Explorer<'a, TT: Transition> {
    /// Explore the possible states starting from the given team state.
    fn explore<AA: ActionApplier<TT>>(
        graph: &'a Graph,
        teams: Vec<TeamState>,
    ) -> ExploreResult<TT> {
        Self::memory_limited_explore::<AA>(graph, teams, usize::MAX).unwrap()
    }

    /// Explore the possible states starting from the given team state.
    ///
    /// When the memory usage reported by global allocator exceeds the limit,
    /// [`SolveFailure::OutOfMemory`] will be returned;
    fn memory_limited_explore<AA: ActionApplier<TT>>(
        graph: &'a Graph,
        teams: Vec<TeamState>,
        memory_limit: usize,
    ) -> Result<ExploreResult<TT>, SolveFailure>;
}

mod naive;
pub use naive::NaiveExplorer;

mod random;
pub use random::RandomExplorer;

mod greedy;
pub use greedy::GreedyExplorer;
