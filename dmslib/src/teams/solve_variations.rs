//! Variations of solve function
use crate::io::OptimizationInfo;

use super::*;

/// Solve a field-team restoration problem on this graph with the given teams without any
/// action elimination or optimizations.
pub fn solve_naive(
    graph: &Graph,
    initial_teams: Vec<TeamState>,
    horizon: Option<usize>,
) -> Solution<RegularTransition> {
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
        Ok(solve_generic::<
            $tt,
            NaiveExplorer<$tt, $act, $si>,
            $aa,
            $ps,
        >($g, $it, $oh))
    };
    // Iterate through action set
    (
        transition = $tt:ty,
        policy = $ps:ty,
        indexer = $si:ty,
        action_applier = $aa:ty,
        action_set($actstr:ident) = [$act1:ty],
        solve($g:expr, $it:expr, $oh:expr)
    ) => {
        if $actstr == stringify!($act1) {
            generate_solve_code!($tt; $ps; $si; $aa; $act1; $g, $it, $oh)
        } else {
            Err(format!("Undefined action set: {}", $actstr))
        }
    };
    (
        transition = $tt:ty,
        policy = $ps:ty,
        indexer = $si:ty,
        action_applier = $aa:ty,
        action_set($actstr:ident) = [$act1:ty, $($rem:ty),+ $(,)?],
        solve($g:expr, $it:expr, $oh:expr)
    ) => {
        if $actstr == stringify!($act1) {
            generate_solve_code!($tt; $ps; $si; $aa; $act1; $g, $it, $oh)
        } else {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                indexer = $si,
                action_applier = $aa,
                action_set($actstr) = [$($rem),+],
                solve($g, $it, $oh)
            )
        }
    };
    // Iterate through action applier
    (
        transition = $tt:ty,
        policy = $ps:ty,
        indexer = $si:ty,
        action_applier($appstr:ident) = [$aa:ty],
        action_set($actstr:ident) = [$($acts:ty),+ $(,)?],
        solve($g:expr, $it:expr, $oh:expr)
    ) => {
        if $appstr == stringify!($aa) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                indexer = $si,
                action_applier = $aa,
                action_set($actstr) = [$($acts),+],
                solve($g, $it, $oh)
            )
        } else {
            Err(format!("Undefined action applier: {}", $actstr))
        }
    };
    (
        transition = $tt:ty,
        policy = $ps:ty,
        indexer = $si:ty,
        action_applier($appstr:ident) = [$aa:ty, $($aarem:ty),+ $(,)?],
        action_set($actstr:ident) = [$($acts:ty),+ $(,)?],
        solve($g:expr, $it:expr, $oh:expr)
    ) => {
        if $appstr == stringify!($aa) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                indexer = $si,
                action_applier = $aa,
                action_set($actstr) = [$($acts),+],
                solve($g, $it, $oh)
            )
        } else {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                indexer = $si,
                action_applier($appstr) = [$($aarem),+],
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
    action_set: &str,
) -> Result<Solution<RegularTransition>, String> {
    generate_solve_code! {
        transition = RegularTransition,
        policy = NaivePolicySynthesizer,
        indexer = NaiveStateIndexer,
        action_applier = NaiveActionApplier,
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
    action_set: &str,
    action_applier: &str,
) -> Result<Solution<TimedTransition>, String> {
    generate_solve_code! {
        transition = TimedTransition,
        policy = NaiveTimedPolicySynthesizer,
        indexer = NaiveStateIndexer,
        action_applier(action_applier) = [
            TimedActionApplier<ConstantTime>,
            TimedActionApplier<TimeUntilArrival>,
            TimedActionApplier<TimeUntilEnergization>,
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
    action_set: &str,
    action_applier: &str,
) -> Result<io::BenchmarkResult, String> {
    if action_applier == stringify!(NaiveActionApplier) {
        Ok(solve_custom_regular(graph, initial_teams, horizon, action_set)?.to_benchmark_result())
    } else {
        Ok(
            solve_custom_timed(graph, initial_teams, horizon, action_set, action_applier)?
                .to_benchmark_result(),
        )
    }
}

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

/// Return an iterator to all possible optimization combinations in `(action_set, action_applier)`
/// form.
pub fn iter_optimizations() -> itertools::Product<
    std::slice::Iter<'static, &'static str>,
    std::slice::Iter<'static, &'static str>,
> {
    itertools::iproduct!(BENCHMARK_ACTION_SETS, BENCHMARK_ACTION_APPLIERS)
}

pub fn all_optimizations() -> Vec<OptimizationInfo> {
    iter_optimizations()
        .map(|(actions, transitions)| OptimizationInfo {
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
    iter_optimizations()
        .map(|(action_applier, action_set)| {
            let result = benchmark_custom(
                graph,
                initial_teams.clone(),
                horizon,
                action_set,
                action_applier,
            )
            .expect("Invalid optimization constant class name");
            let optimizations = io::OptimizationInfo {
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
