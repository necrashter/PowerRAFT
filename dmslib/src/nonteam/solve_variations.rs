//! Variations of solve function
use super::*;

/// Solve a field-team restoration problem on this graph with the given teams without any
/// action elimination or optimizations.
pub fn solve_naive(
    graph: &Graph,
    config: &Config,
) -> Result<Solution<RegularTransition>, SolveFailure> {
    solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(graph, config)
}

/// Macro for generating solve code that reads class names from variables and constructs a code
/// that calls the appropriate solve function variation.
macro_rules! generate_solve_code {
    ($tt:ty; $ps:ty; $si:ty; $aa:ty; $act:ty; $g:expr, $oh:expr) => {
        solve_generic::<
            $tt,
            NaiveExplorer<$tt, $act, $si>,
            $aa,
            $ps,
        >($g, $oh)
    };
    // Iterate through action set
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier = $aa:ty,
        indexer = $si:ty,
        action_set($actstr:ident) = [$act1:ty],
        solve($g:expr, $oh:expr)
    ) => {
        if $actstr == stringify!($act1) {
            generate_solve_code!($tt; $ps; $si; $aa; $act1; $g, $oh)
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
        solve($g:expr, $oh:expr)
    ) => {
        if $actstr == stringify!($act1) {
            generate_solve_code!($tt; $ps; $si; $aa; $act1; $g, $oh)
        } else {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer = $si,
                action_set($actstr) = [$($rem),+],
                solve($g, $oh)
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
        solve($g:expr, $oh:expr)
    ) => {
        if $sistr == stringify!($si) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer = $si,
                action_set($actstr) = [$($acts),+],
                solve($g, $oh)
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
        solve($g:expr, $oh:expr)
    ) => {
        if $sistr == stringify!($si) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer = $si,
                action_set($actstr) = [$($acts),+],
                solve($g, $oh)
            )
        } else {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer($sistr) = [$($sis),+],
                action_set($actstr) = [$($acts),+],
                solve($g, $oh)
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
        solve($g:expr, $oh:expr)
    ) => {
        if $appstr == stringify!($aa) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer($sistr) = [$($sis),+],
                action_set($actstr) = [$($acts),+],
                solve($g, $oh)
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
        solve($g:expr, $oh:expr)
    ) => {
        if $appstr == stringify!($aa) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                indexer($sistr) = [$($sis),+],
                action_set($actstr) = [$($acts),+],
                solve($g, $oh)
            )
        } else {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier($appstr) = [$($aarem),+],
                indexer($sistr) = [$($sis),+],
                action_set($actstr) = [$($acts),+],
                solve($g, $oh)
            )
        }
    };
}

/// Solve the field-teams restoration problem with [`RegularTransition`]s (classic MDP
/// transitions without time) and the given action set class.
pub fn solve_custom(
    graph: &Graph,
    config: &Config,
    indexer: &str,
    action_set: &str,
) -> Result<Solution<RegularTransition>, SolveFailure> {
    generate_solve_code! {
        transition = RegularTransition,
        policy = NaivePolicySynthesizer,
        action_applier = NaiveActionApplier,
        indexer(indexer) = [
            NaiveStateIndexer,
            BitStackStateIndexer,
        ],
        action_set(action_set) = [
            NaiveActions,
        ],
        solve(graph, config)
    }
}

/// Solve the field-teams restoration problem with the given:
/// - action applier class
/// - action set class
///
/// Returns a [`io::BenchmarkResult`] on success.
pub fn benchmark_custom(
    graph: &Graph,
    config: &Config,
    indexer: &str,
    action_set: &str,
) -> Result<io::BenchmarkResult, SolveFailure> {
    Ok(solve_custom(graph, config, indexer, action_set)?.to_benchmark_result())
}
