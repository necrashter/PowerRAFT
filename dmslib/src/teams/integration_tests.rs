//! Integration tests
//!
//! Test MDP construction and policy synthesis as a whole.

use super::*;

const SYSTEM_PAPER_EXAMPLE_0: &str = include_str!("../../../graphs/FieldTeams/paperE0.json");

#[test]
#[ignore]
fn pe0_1_team() {
    let input_graph: io::Graph = serde_json::from_str(SYSTEM_PAPER_EXAMPLE_0).unwrap();
    let (problem, config) = input_graph
        .to_teams_problem(
            vec![io::Team {
                index: Some(0),
                latlng: None,
            }],
            Some(30),
        )
        .unwrap();
    const OPTIMAL_VALUE: f64 = 129.283203125;

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 645);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions, SortedStateIndexer<NaiveStateIndexer>>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 645);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, PermutationalActions, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 645);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, FilterEnergizedOnWay<NaiveActions>, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 544);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<
            RegularTransition,
            FilterEnergizedOnWay<PermutationalActions>,
            NaiveStateIndexer,
        >,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 544);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, FilterOnWay<NaiveActions>, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 544);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, FilterOnWay<PermutationalActions>, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 544);

    // TimedTransition equivalence with time = 1

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, NaiveActions, NaiveStateIndexer>,
        TimedActionApplier<ConstantTime>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 645);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<
            TimedTransition,
            FilterEnergizedOnWay<PermutationalActions>,
            NaiveStateIndexer,
        >,
        TimedActionApplier<ConstantTime>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 544);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, FilterOnWay<PermutationalActions>, NaiveStateIndexer>,
        TimedActionApplier<ConstantTime>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 544);

    // Timed

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, NaiveActions, NaiveStateIndexer>,
        TimedActionApplier<TimeUntilArrival>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 433);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<
            TimedTransition,
            FilterEnergizedOnWay<PermutationalActions>,
            NaiveStateIndexer,
        >,
        TimedActionApplier<TimeUntilArrival>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 367);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<
            TimedTransition,
            FilterEnergizedOnWay<PermutationalActions>,
            NaiveStateIndexer,
        >,
        TimedActionApplier<TimeUntilEnergization>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 367);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, FilterOnWay<PermutationalActions>, NaiveStateIndexer>,
        TimedActionApplier<TimeUntilEnergization>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 367);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<
            TimedTransition,
            FilterOnWay<PermutationalActions>,
            SortedStateIndexer<NaiveStateIndexer>,
        >,
        TimedActionApplier<TimeUntilEnergization>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 367);
}

#[test]
#[ignore]
fn pe0_2_team() {
    let input_graph: io::Graph = serde_json::from_str(SYSTEM_PAPER_EXAMPLE_0).unwrap();
    let (problem, config) = input_graph
        .to_teams_problem(
            vec![
                io::Team {
                    index: Some(1),
                    latlng: None,
                },
                io::Team {
                    index: Some(6),
                    latlng: None,
                },
            ],
            Some(30),
        )
        .unwrap();
    const OPTIMAL_VALUE: f64 = 132.0810546875;

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 11545);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions, SortedStateIndexer<NaiveStateIndexer>>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 6161);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, PermutationalActions, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 9039);
    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<
            RegularTransition,
            PermutationalActions,
            SortedStateIndexer<NaiveStateIndexer>,
        >,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 5190);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, FilterEnergizedOnWay<NaiveActions>, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 8234);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<
            RegularTransition,
            FilterEnergizedOnWay<PermutationalActions>,
            NaiveStateIndexer,
        >,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 6551);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, FilterOnWay<NaiveActions>, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 7161);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<
            RegularTransition,
            FilterOnWay<NaiveActions>,
            SortedStateIndexer<NaiveStateIndexer>,
        >,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 3912);

    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, FilterOnWay<PermutationalActions>, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 5847);

    // TimedTransition equivalence with time = 1

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, NaiveActions, NaiveStateIndexer>,
        TimedActionApplier<ConstantTime>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 11545);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<
            TimedTransition,
            FilterEnergizedOnWay<PermutationalActions>,
            NaiveStateIndexer,
        >,
        TimedActionApplier<ConstantTime>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 6551);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, FilterOnWay<PermutationalActions>, NaiveStateIndexer>,
        TimedActionApplier<ConstantTime>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 5847);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<
            TimedTransition,
            FilterOnWay<PermutationalActions>,
            SortedStateIndexer<NaiveStateIndexer>,
        >,
        TimedActionApplier<ConstantTime>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 3489);

    // Timed

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, NaiveActions, NaiveStateIndexer>,
        TimedActionApplier<TimeUntilArrival>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 9089);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<
            TimedTransition,
            FilterEnergizedOnWay<PermutationalActions>,
            NaiveStateIndexer,
        >,
        TimedActionApplier<TimeUntilArrival>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 5762);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<
            TimedTransition,
            FilterEnergizedOnWay<PermutationalActions>,
            NaiveStateIndexer,
        >,
        TimedActionApplier<TimeUntilEnergization>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 4751);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, FilterOnWay<PermutationalActions>, NaiveStateIndexer>,
        TimedActionApplier<TimeUntilArrival>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 5216);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<TimedTransition, FilterOnWay<PermutationalActions>, NaiveStateIndexer>,
        TimedActionApplier<TimeUntilEnergization>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 4217);

    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<
            TimedTransition,
            FilterOnWay<PermutationalActions>,
            SortedStateIndexer<NaiveStateIndexer>,
        >,
        TimedActionApplier<TimeUntilEnergization>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 2478);
}

#[test]
#[ignore]
fn save_test_pe0_1_team() {
    let input_graph: io::Graph = serde_json::from_str(SYSTEM_PAPER_EXAMPLE_0).unwrap();
    let problem = io::TeamProblem {
        name: Some("Save Test Team Problem PE0 1-Team".to_string()),
        graph: input_graph,
        teams: vec![io::Team {
            index: Some(0),
            latlng: None,
        }],
        horizon: Some(10),
        pfo: None,
        time_func: Default::default(),
    };

    let solution = problem.clone().solve_naive().unwrap();

    let mut path: std::path::PathBuf = std::env::temp_dir();
    path.push("dmslib-test.pe0-1-team.bin");
    io::fs::save_solution(problem.clone(), solution.clone(), &path).unwrap();

    let io::fs::SaveFile {
        problem: saved_problem,
        solution: saved_solution,
    } = io::fs::load_solution(&path).unwrap();

    let saved_solution = if let io::fs::GenericTeamSolution::Regular(s) = saved_solution {
        s
    } else {
        panic!("saved solution type is wrong");
    };

    assert_eq!(problem, saved_problem);
    assert_eq!(solution, saved_solution);
}
