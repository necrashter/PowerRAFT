use dmslib::policy::*;
use dmslib::teams::state::*;
use dmslib::teams::transitions::*;
use dmslib::teams::*;
use iai_callgrind::{black_box, library_benchmark, library_benchmark_group, main};

const SYSTEM_PAPER_EXAMPLE_0: &str = include_str!("../../graphs/FieldTeams/paperE0.json");

// These are the same test cases from integration tests.

fn setup_1_team() -> (Problem, Config) {
    let input_graph: dmslib::io::Graph = serde_json::from_str(SYSTEM_PAPER_EXAMPLE_0).unwrap();
    input_graph
        .to_teams_problem(
            vec![dmslib::io::Team {
                index: Some(0),
                latlng: None,
            }],
            Some(30),
        )
        .unwrap()
}

fn setup_2_team() -> (Problem, Config) {
    use dmslib::io;
    let input_graph: io::Graph = serde_json::from_str(SYSTEM_PAPER_EXAMPLE_0).unwrap();
    input_graph
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
        .unwrap()
}

#[library_benchmark]
#[bench::with_1_team(setup_1_team())]
#[bench::with_2_teams(setup_2_team())]
fn solve_naive(input: (Problem, Config)) {
    let (problem, config) = input;
    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions, NaiveStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    black_box(solution);
}

#[library_benchmark]
#[bench::with_1_team(setup_1_team())]
#[bench::with_2_teams(setup_2_team())]
fn solve_naive_bitstack(input: (Problem, Config)) {
    let (problem, config) = input;
    let solution = solve_generic::<
        RegularTransition,
        NaiveExplorer<RegularTransition, NaiveActions, BitStackStateIndexer>,
        NaiveActionApplier,
        NaivePolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    black_box(solution);
}

#[library_benchmark]
#[bench::with_1_team(setup_1_team())]
#[bench::with_2_teams(setup_2_team())]
fn solve_opt(input: (Problem, Config)) {
    let (problem, config) = input;
    let solution = solve_generic::<
        TimedTransition,
        NaiveExplorer<
            TimedTransition,
            FilterEnergizedOnWay<PermutationalActions>,
            SortedStateIndexer<BitStackStateIndexer>,
        >,
        TimedActionApplier<TimeUntilEnergization>,
        NaiveTimedPolicySynthesizer,
    >(&problem.graph, problem.initial_teams.clone(), &config)
    .unwrap();
    black_box(solution);
}

library_benchmark_group!(
    name = bench_fibonacci_group;
    benchmarks = solve_naive, solve_naive_bitstack, solve_opt
);

main!(library_benchmark_groups = bench_fibonacci_group);
