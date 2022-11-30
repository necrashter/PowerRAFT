//! Variations of solve function
use crate::io::OptimizationInfo;

use super::*;

/// Solve a field-team restoration problem on this graph with the given teams without any
/// action elimination or optimizations.
pub fn solve_naive(
    graph: &Graph,
    initial_teams: Vec<TeamState>,
    horizon: Option<usize>,
) -> Result<Solution<RegularTransition>, SolveFailure> {
    solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(graph, initial_teams, horizon)
}

/// Macro for generating solve code that reads class names from variables and constructs a code
/// that calls the appropriate solve function variation.
macro_rules! generate_solve_code {
    ($tt:ty; $ps:ty; $si:ty; $aa:ty; $act:ty; $g:expr, $it:expr, $oh:expr) => {
        solve_generic::<
            $tt,
            NaiveExplorer<$tt, $act, $si>,
            $aa,
            $ps,
        >($g, $it, $oh)
    };
    // Iterate through action set
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier = $aa:ty,
        indexer = $si:ty,
        action_set($actstr:ident) = [$act1:ty],
        solve($g:expr, $it:expr, $oh:expr)
    ) => {
        if $actstr == stringify!($act1) {
            generate_solve_code!($tt; $ps; $si; $aa; $act1; $g, $it, $oh)
        } else {
            Err(SolveFailure::BadInput(format!("Undefined action set: {}", $actstr)))
        }
    };
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier = $aa:ty,
        indexer = $si:ty,
        action_set($actstr:ident) = [$act1:ty, $($rem:ty),+ $(,)?],
        solve($g:expr, $it:expr, $oh:expr)
    ) => {
        if $actstr == stringify!($act1) {
            generate_solve_code!($tt; $ps; $si; $aa; $act1; $g, $it, $oh)
        } else {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer = $si,
                action_set($actstr) = [$($rem),+],
                solve($g, $it, $oh)
            )
        }
    };
    // Iterate through State Indexer
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier = $aa:ty,
        indexer($sistr:ident) = [$si:ty],
        action_set($actstr:ident) = [$($acts:ty),+ $(,)?],
        solve($g:expr, $it:expr, $oh:expr)
    ) => {
        if $sistr == stringify!($si) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer = $si,
                action_set($actstr) = [$($acts),+],
                solve($g, $it, $oh)
            )
        } else {
            Err(SolveFailure::BadInput(format!("Undefined state indexer: {}", $sistr)))
        }
    };
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier = $aa:ty,
        indexer($sistr:ident) = [$si:ty, $($sis:ty),+ $(,)?],
        action_set($actstr:ident) = [$($acts:ty),+ $(,)?],
        solve($g:expr, $it:expr, $oh:expr)
    ) => {
        if $sistr == stringify!($si) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer = $si,
                action_set($actstr) = [$($acts),+],
                solve($g, $it, $oh)
            )
        } else {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer($sistr) = [$($sis),+],
                action_set($actstr) = [$($acts),+],
                solve($g, $it, $oh)
            )
        }
    };
    // Iterate through action applier
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier($appstr:ident) = [$aa:ty],
        indexer($sistr:ident) = [$($sis:ty),+ $(,)?],
        action_set($actstr:ident) = [$($acts:ty),+ $(,)?],
        solve($g:expr, $it:expr, $oh:expr)
    ) => {
        if $appstr == stringify!($aa) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer($sistr) = [$($sis),+],
                action_set($actstr) = [$($acts),+],
                solve($g, $it, $oh)
            )
        } else {
            Err(SolveFailure::BadInput(format!("Undefined action applier: {}", $actstr)))
        }
    };
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier($appstr:ident) = [$aa:ty, $($aarem:ty),+ $(,)?],
        indexer($sistr:ident) = [$($sis:ty),+ $(,)?],
        action_set($actstr:ident) = [$($acts:ty),+ $(,)?],
        solve($g:expr, $it:expr, $oh:expr)
    ) => {
        if $appstr == stringify!($aa) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer($sistr) = [$($sis),+],
                action_set($actstr) = [$($acts),+],
                solve($g, $it, $oh)
            )
        } else {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier($appstr) = [$($aarem),+],
                indexer($sistr) = [$($sis),+],
                action_set($actstr) = [$($acts),+],
                solve($g, $it, $oh)
            )
        }
    };
}

/// Solve the field-teams restoration problem with [`RegularTransition`]s (classic MDP
/// transitions without time) and the given action set class.
pub fn solve_custom_regular(
    graph: &Graph,
    initial_teams: Vec<TeamState>,
    horizon: Option<usize>,
    indexer: &str,
    action_set: &str,
) -> Result<Solution<RegularTransition>, SolveFailure> {
    generate_solve_code! {
        transition = RegularTransition,
        policy = NaivePolicySynthesizer,
        action_applier = NaiveActionApplier,
        indexer(indexer) = [
            NaiveStateIndexer,
            SortedStateIndexer,
        ],
        action_set(action_set) = [
            NaiveActions,
            PermutationalActions,
            FilterOnWay<NaiveActions>,
            FilterOnWay<PermutationalActions>,
            FilterEnergizedOnWay<NaiveActions>,
            FilterEnergizedOnWay<PermutationalActions>,
        ],
        solve(graph, initial_teams, horizon)
    }
}

/// Solve the field-teams restoration problem with [`TimedTransition`]s and the given:
/// - action applier class (variations of `TimedActionApplier<T>` where `T` determines time)
/// - action set class
pub fn solve_custom_timed(
    graph: &Graph,
    initial_teams: Vec<TeamState>,
    horizon: Option<usize>,
    indexer: &str,
    action_set: &str,
    action_applier: &str,
) -> Result<Solution<TimedTransition>, SolveFailure> {
    generate_solve_code! {
        transition = TimedTransition,
        policy = NaiveTimedPolicySynthesizer,
        action_applier(action_applier) = [
            TimedActionApplier<ConstantTime>,
            TimedActionApplier<TimeUntilArrival>,
            TimedActionApplier<TimeUntilEnergization>,
        ],
        indexer(indexer) = [
            NaiveStateIndexer,
            SortedStateIndexer,
        ],
        action_set(action_set) = [
            NaiveActions,
            PermutationalActions,
            FilterOnWay<NaiveActions>,
            FilterOnWay<PermutationalActions>,
            FilterEnergizedOnWay<NaiveActions>,
            FilterEnergizedOnWay<PermutationalActions>,
        ],
        solve(graph, initial_teams, horizon)
    }
}

/// Solve the field-teams restoration problem with the given:
/// - action applier class
/// - action set class
///
/// Returns a [`io::BenchmarkResult`] on success.
pub fn benchmark_custom(
    graph: &Graph,
    initial_teams: Vec<TeamState>,
    horizon: Option<usize>,
    indexer: &str,
    action_set: &str,
    action_applier: &str,
) -> Result<io::BenchmarkResult, SolveFailure> {
    if action_applier == stringify!(NaiveActionApplier) {
        Ok(
            solve_custom_regular(graph, initial_teams, horizon, indexer, action_set)?
                .to_benchmark_result(),
        )
    } else {
        Ok(solve_custom_timed(
            graph,
            initial_teams,
            horizon,
            indexer,
            action_set,
            action_applier,
        )?
        .to_benchmark_result())
    }
}

const BENCHMARK_STATE_INDEXERS: &[&str] = &[
    stringify!(NaiveStateIndexer),
    stringify!(SortedStateIndexer),
];

const BENCHMARK_ACTION_APPLIERS: &[&str] = &[
    "NaiveActionApplier",
    "TimedActionApplier<TimeUntilArrival>",
    "TimedActionApplier<TimeUntilEnergization>",
];

const BENCHMARK_ACTION_SETS: &[&str] = &[
    "NaiveActions",
    "PermutationalActions",
    "FilterEnergizedOnWay<NaiveActions>",
    "FilterEnergizedOnWay<PermutationalActions>",
    "FilterOnWay<NaiveActions>",
    "FilterOnWay<PermutationalActions>",
];

pub fn all_optimizations() -> Vec<OptimizationInfo> {
    itertools::iproduct!(
        BENCHMARK_STATE_INDEXERS,
        BENCHMARK_ACTION_SETS,
        BENCHMARK_ACTION_APPLIERS
    )
    .map(|(indexer, actions, transitions)| OptimizationInfo {
        indexer: indexer.to_string(),
        actions: actions.to_string(),
        transitions: transitions.to_string(),
    })
    .collect()
}

/// Run all optimization combination possibilities on this field-teams restoration problem.
pub fn benchmark_all(
    graph: &Graph,
    initial_teams: Vec<TeamState>,
    horizon: Option<usize>,
) -> Vec<io::OptimizationBenchmarkResult> {
    itertools::iproduct!(
        BENCHMARK_STATE_INDEXERS,
        BENCHMARK_ACTION_SETS,
        BENCHMARK_ACTION_APPLIERS
    )
    .map(|(indexer, action_applier, action_set)| {
        let result = benchmark_custom(
            graph,
            initial_teams.clone(),
            horizon,
            indexer,
            action_set,
            action_applier,
        );
        let optimizations = io::OptimizationInfo {
            indexer: indexer.to_string(),
            actions: action_set.to_string(),
            transitions: action_applier.to_string(),
        };
        io::OptimizationBenchmarkResult {
            optimizations,
            result,
        }
    })
    .collect()
}
