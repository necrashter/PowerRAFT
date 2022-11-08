//! Variations of solve function
use super::*;

/// Solve a field-team restoration problem on this graph with the given teams without any
/// action elimination or optimizations.
pub fn solve_naive(graph: &Graph, initial_teams: Vec<TeamState>) -> Solution<RegularTransition> {
    solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(graph, initial_teams)
}

/// Macro for generating solve code that reads class names from variables and constructs a code
/// that calls the appropriate solve function variation.
macro_rules! generate_solve_code {
    ($tt:ty; $ps:ty; $aa:ty; $act:ty; $g:expr, $it:expr) => {
        Ok(solve_generic::<
            $tt,
            NaiveExplorer<$tt, $act>,
            $aa,
            $ps,
        >($g, $it))
    };
    // Iterate through action set
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier = $aa:ty,
        action_set($actstr:ident) = [$act1:ty],
        solve($g:expr, $it:expr)
    ) => {
        if $actstr == stringify!($act1) {
            generate_solve_code!($tt; $ps; $aa; $act1; $g, $it)
        } else {
            Err(format!("Undefined action set: {}", $actstr))
        }
    };
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier = $aa:ty,
        action_set($actstr:ident) = [$act1:ty, $($rem:ty),+ $(,)?],
        solve($g:expr, $it:expr)
    ) => {
        if $actstr == stringify!($act1) {
            generate_solve_code!($tt; $ps; $aa; $act1; $g, $it)
        } else {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                action_set($actstr) = [$($rem),+],
                solve($g, $it)
            )
        }
    };
    // Iterate through action applier
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier($appstr:ident) = [$aa:ty],
        action_set($actstr:ident) = [$($acts:ty),+ $(,)?],
        solve($g:expr, $it:expr)
    ) => {
        if $appstr == stringify!($aa) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                action_set($actstr) = [$($acts),+],
                solve($g, $it)
            )
        } else {
            Err(format!("Undefined action applier: {}", $actstr))
        }
    };
    (
        transition = $tt:ty,
        policy = $ps:ty,
        action_applier($appstr:ident) = [$aa:ty, $($aarem:ty),+ $(,)?],
        action_set($actstr:ident) = [$($acts:ty),+ $(,)?],
        solve($g:expr, $it:expr)
    ) => {
        if $appstr == stringify!($aa) {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier = $aa,
                action_set($actstr) = [$($acts),+],
                solve($g, $it)
            )
        } else {
            generate_solve_code!(
                transition = $tt,
                policy = $ps,
                action_applier($appstr) = [$($aarem),+],
                action_set($actstr) = [$($acts),+],
                solve($g, $it)
            )
        }
    };
}

/// Solve the field-teams restoration problem with [`RegularTransition`]s (classic MDP
/// transitions without time) and the given action set class.
pub fn solve_custom_regular(
    graph: &Graph,
    initial_teams: Vec<TeamState>,
    action_set: &str,
) -> Result<Solution<RegularTransition>, String> {
    generate_solve_code! {
        transition = RegularTransition,
        policy = NaivePolicySynthesizer,
        action_applier = NaiveActionApplier,
        action_set(action_set) = [
            NaiveActions,
            PermutationalActions,
            FilterOnWay<NaiveActions>,
            FilterOnWay<PermutationalActions>,
            FilterEnergizedOnWay<NaiveActions>,
            FilterEnergizedOnWay<PermutationalActions>,
        ],
        solve(graph, initial_teams)
    }
}

/// Solve the field-teams restoration problem with [`TimedTransition`]s and the given:
/// - action applier class (variations of `TimedActionApplier<T>` where `T` determines time)
/// - action set class
pub fn solve_custom_timed(
    graph: &Graph,
    initial_teams: Vec<TeamState>,
    action_set: &str,
    action_applier: &str,
) -> Result<Solution<TimedTransition>, String> {
    generate_solve_code! {
        transition = TimedTransition,
        policy = NaiveTimedPolicySynthesizer,
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
        solve(graph, initial_teams)
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
    action_set: &str,
    action_applier: &str,
) -> Result<io::BenchmarkResult, String> {
    if action_applier == stringify!(NaiveActionApplier) {
        Ok(solve_custom_regular(graph, initial_teams, action_set)?.to_benchmark_result())
    } else {
        Ok(
            solve_custom_timed(graph, initial_teams, action_set, action_applier)?
                .to_benchmark_result(),
        )
    }
}
