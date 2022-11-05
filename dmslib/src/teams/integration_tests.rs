//! Integration tests
//!
//! Test MDP construction and policy synthesis as a whole.

use super::*;

const SYSTEM_PAPER_EXAMPLE_0: &'static str =
    include_str!("../../../graphs/FieldTeams/paperE0.json");

#[test]
#[ignore]
fn pe0_1_team() {
    let input_graph: webclient::Graph = serde_json::from_str(SYSTEM_PAPER_EXAMPLE_0).unwrap();
    let problem: Problem = input_graph
        .to_teams_problem(vec![webclient::Team {
            index: Some(0),
            latlng: None,
        }])
        .unwrap();

    let optimal_value = 129.283203125;

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 645);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, PermutationalActions>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 645);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, FilterEnergizedOnWay<NaiveActions>>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 544);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, FilterEnergizedOnWay<PermutationalActions>>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 544);

    // Timed

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, NaiveActions>,
        TimedActionApplier<TimeUntilArrival>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 433);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, FilterEnergizedOnWay<PermutationalActions>>,
        TimedActionApplier<TimeUntilArrival>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 367);
}

#[test]
#[ignore]
fn pe0_2_team() {
    let input_graph: webclient::Graph = serde_json::from_str(SYSTEM_PAPER_EXAMPLE_0).unwrap();
    let problem: Problem = input_graph
        .to_teams_problem(vec![
            webclient::Team {
                index: Some(1),
                latlng: None,
            },
            webclient::Team {
                index: Some(6),
                latlng: None,
            },
        ])
        .unwrap();

    let optimal_value = 132.0810546875;

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 11545);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, PermutationalActions>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 9039);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, FilterEnergizedOnWay<NaiveActions>>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 8234);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, FilterEnergizedOnWay<PermutationalActions>>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 6551);

    // Timed

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, NaiveActions>,
        TimedActionApplier<TimeUntilArrival>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 9089);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, FilterEnergizedOnWay<PermutationalActions>>,
        TimedActionApplier<TimeUntilArrival>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone());
    assert_eq!(solution.get_min_value(), optimal_value);
    assert_eq!(solution.transitions.len(), 5762);
}