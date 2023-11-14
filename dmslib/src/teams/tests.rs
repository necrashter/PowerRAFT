use super::*;

fn get_paper_example_graph(partitions: Option<Vec<Vec<BusIndex>>>) -> Graph {
    Graph {
        travel_times: ndarray::arr2(&[
            [0, 1, 2, 1, 2, 2],
            [1, 0, 1, 2, 2, 2],
            [2, 1, 0, 2, 2, 1],
            [1, 2, 2, 0, 1, 2],
            [2, 2, 2, 1, 0, 1],
            [2, 2, 1, 2, 1, 0],
        ]),
        branches: vec![vec![1], vec![0, 2], vec![1], vec![4], vec![3, 5], vec![4]],
        connected: vec![true, false, false, true, false, false],
        pfs: ndarray::arr1(&[0.5, 0.5, 0.25, 0.25, 0.25, 0.25]),
        team_nodes: Array2::default((0, 0)),
        partitions,
    }
}

fn check_sets<T: PartialEq>(output: &Vec<T>, expected: &Vec<T>) {
    assert_eq!(output.len(), expected.len());
    for a in expected {
        assert!(output.contains(a));
    }
}

#[test]
fn paper_example_4_1_1() {
    let graph = get_paper_example_graph(None);
    let buses: Vec<BusState> = vec![
        BusState::Energized,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Energized,
        BusState::Damaged,
        BusState::Unknown,
    ];
    let teams: Vec<TeamState> = vec![
        TeamState { time: 0, index: 0 },
        TeamState { index: 2, time: 1 },
    ];
    let state = State { buses, teams };

    let cost = state.get_cost();
    assert_eq!(cost, 4 as Cost);

    let iter = NaiveActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);

    assert_eq!(actions, vec![vec![1, 2]]);

    let expected_team_outcome: Vec<TeamState> = vec![
        TeamState { time: 0, index: 1 },
        TeamState { time: 0, index: 2 },
    ];
    let expected_outcomes: Vec<(Probability, State)> = vec![
        (
            0.5,
            State {
                teams: expected_team_outcome.clone(),
                buses: vec![
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Unknown,
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Unknown,
                ],
            },
        ),
        (
            0.5 * 0.25,
            State {
                teams: expected_team_outcome.clone(),
                buses: vec![
                    BusState::Energized,
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Unknown,
                ],
            },
        ),
        (
            0.5 * 0.75,
            State {
                teams: expected_team_outcome,
                buses: vec![
                    BusState::Energized,
                    BusState::Energized,
                    BusState::Energized,
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Unknown,
                ],
            },
        ),
    ];
    let outcomes: Vec<(Probability, State)> =
        NaiveActionApplier::apply_state(&state, cost, &graph, &actions[0])
            .into_iter()
            .map(|(transition, state)| {
                assert_eq!(transition.cost, cost);
                (transition.p, state)
            })
            .collect();
    check_sets(&outcomes, &expected_outcomes);
}

#[test]
fn test_timed_action_applier() {
    let mut graph = get_paper_example_graph(None);
    graph.travel_times.mapv_inplace(|x| 2 * x);
    let buses: Vec<BusState> = vec![
        BusState::Energized,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Energized,
        BusState::Damaged,
        BusState::Unknown,
    ];
    let teams: Vec<TeamState> = vec![
        TeamState { time: 0, index: 0 },
        TeamState { index: 2, time: 2 },
    ];
    let state = State { buses, teams };

    let cost = state.get_cost();
    assert_eq!(cost, 4 as Cost);

    let action: Vec<TeamAction> = vec![1, 2];

    // Naive action
    let expected_team_outcome: Vec<TeamState> = vec![
        TeamState { index: 1, time: 1 },
        TeamState { index: 2, time: 1 },
    ];
    let expected_outcomes: Vec<(Probability, State)> = vec![(
        1.0,
        State {
            teams: expected_team_outcome,
            buses: vec![
                BusState::Energized,
                BusState::Unknown,
                BusState::Unknown,
                BusState::Energized,
                BusState::Damaged,
                BusState::Unknown,
            ],
        },
    )];
    let outcomes: Vec<(Probability, State)> =
        NaiveActionApplier::apply_state(&state, cost, &graph, &action)
            .into_iter()
            .map(|(transition, state)| {
                assert_eq!(transition.cost, cost);
                (transition.p, state)
            })
            .collect();
    check_sets(&outcomes, &expected_outcomes);

    // Timed action
    let expected_team_outcome: Vec<TeamState> = vec![
        TeamState { time: 0, index: 1 },
        TeamState { time: 0, index: 2 },
    ];
    let expected_outcomes: Vec<(Probability, State)> = vec![
        (
            0.5,
            State {
                teams: expected_team_outcome.clone(),
                buses: vec![
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Unknown,
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Unknown,
                ],
            },
        ),
        (
            0.5 * 0.25,
            State {
                teams: expected_team_outcome.clone(),
                buses: vec![
                    BusState::Energized,
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Unknown,
                ],
            },
        ),
        (
            0.5 * 0.75,
            State {
                teams: expected_team_outcome,
                buses: vec![
                    BusState::Energized,
                    BusState::Energized,
                    BusState::Energized,
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Unknown,
                ],
            },
        ),
    ];
    let outcomes: Vec<(Probability, State)> =
        TimedActionApplier::<TimeUntilArrival>::apply_state(&state, cost, &graph, &action)
            .into_iter()
            .map(|(transition, state)| {
                assert_eq!(transition.cost, cost);
                assert_eq!(transition.time, 2);
                (transition.p, state)
            })
            .collect();
    check_sets(&outcomes, &expected_outcomes);
}

#[test]
fn on_energized_bus_actions() {
    let graph = get_paper_example_graph(None);
    let buses: Vec<BusState> = vec![
        BusState::Energized,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Energized,
        BusState::Unknown,
        BusState::Unknown,
    ];
    let teams: Vec<TeamState> = vec![
        TeamState { time: 0, index: 0 },
        TeamState { time: 0, index: 3 },
    ];
    let state = State { buses, teams };

    assert_eq!(state.get_cost(), 4 as Cost);

    let iter = NaiveActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    let expected_actions: Vec<Vec<TeamAction>> = vec![
        vec![1, 1],
        vec![1, 2],
        vec![1, 4],
        vec![1, 5],
        vec![4, 1],
        vec![4, 2],
        vec![4, 4],
        vec![4, 5],
        vec![2, 1],
        vec![2, 4],
        vec![5, 1],
        vec![5, 4],
    ];
    check_sets(&actions, &expected_actions);

    let iter = WaitMovingActions::<NaiveActions>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let expected_actions: Vec<Vec<TeamAction>> = vec![
        vec![1, 1],
        vec![1, 2],
        vec![1, 4],
        vec![4, 1],
        vec![4, 2],
        vec![4, 4],
        vec![5, 1],
        vec![5, 4],
    ];

    let iter = FilterEnergizedOnWay::<NaiveActions>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let iter = WaitMovingActions::<FilterEnergizedOnWay<NaiveActions>>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let expected_actions: Vec<Vec<TeamAction>> = vec![
        vec![1, 1],
        vec![1, 2],
        vec![1, 4],
        vec![1, 5],
        vec![4, 4],
        vec![2, 4],
        vec![5, 4],
    ];

    let iter = PermutationalActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let iter = WaitMovingActions::<PermutationalActions>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let expected_actions: Vec<Vec<TeamAction>> =
        vec![vec![1, 1], vec![1, 2], vec![1, 4], vec![4, 4], vec![5, 4]];

    let iter = FilterEnergizedOnWay::<PermutationalActions>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let iter = WaitMovingActions::<FilterEnergizedOnWay<PermutationalActions>>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);
}

#[test]
fn wait_moving_elimination() {
    let graph = get_paper_example_graph(None);
    let buses: Vec<BusState> = vec![
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Energized,
        BusState::Energized,
        BusState::Energized,
    ];
    let teams: Vec<TeamState> = vec![
        TeamState { time: 0, index: 2 },
        TeamState { index: 0, time: 1 },
    ];
    let state = State { buses, teams };

    assert_eq!(state.get_cost(), 3 as Cost);

    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![2, 0], vec![0, 0], vec![1, 0]];

    let iter = NaiveActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let iter = FilterEnergizedOnWay::<NaiveActions>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let iter = WaitMovingActions::<NaiveActions>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &vec![vec![2, 0]]);

    let iter = FilterEnergizedOnWay::<WaitMovingActions<NaiveActions>>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &vec![vec![2, 0]]);
}

#[test]
fn beta_values_on_paper_example() {
    let graph = get_paper_example_graph(None);
    let dummy_teams = vec![TeamState { time: 0, index: 0 }];

    let state = State {
        buses: vec![
            BusState::Energized,
            BusState::Unknown,
            BusState::Unknown,
            BusState::Energized,
            BusState::Damaged,
            BusState::Unknown,
        ],
        teams: dummy_teams.clone(),
    };
    assert_eq!(
        state.compute_minbeta(&graph),
        vec![0, 1, 2, 0, 0, BusIndex::MAX]
    );

    let state = State {
        buses: vec![BusState::Unknown; 6],
        teams: dummy_teams.clone(),
    };
    assert_eq!(state.compute_minbeta(&graph), vec![1, 2, 3, 1, 2, 3]);

    let state = State {
        buses: vec![
            BusState::Damaged,
            BusState::Unknown,
            BusState::Unknown,
            BusState::Damaged,
            BusState::Unknown,
            BusState::Unknown,
        ],
        teams: dummy_teams,
    };
    assert_eq!(
        state.compute_minbeta(&graph),
        vec![
            0,
            BusIndex::MAX,
            BusIndex::MAX,
            0,
            BusIndex::MAX,
            BusIndex::MAX,
        ]
    );
}

#[test]
fn minimal_nonopt_permutations() {
    let graph = Graph {
        travel_times: ndarray::arr2(&[[0, 1, 1, 2], [1, 0, 2, 1], [1, 2, 0, 1], [2, 1, 1, 0]]),
        branches: vec![vec![], vec![]],
        connected: vec![true, true],
        pfs: ndarray::arr1(&[0.5, 0.5]),
        team_nodes: Array2::default((0, 0)),
        partitions: None,
    };

    let state = State {
        buses: vec![BusState::Unknown, BusState::Unknown],
        teams: vec![
            TeamState { time: 0, index: 2 },
            TeamState { time: 0, index: 3 },
        ],
    };

    assert_eq!(state.compute_minbeta(&graph), vec![1, 1]);

    let expected_actions: Vec<Vec<TeamAction>> =
        vec![vec![0, 0], vec![1, 0], vec![0, 1], vec![1, 1]];
    let iter = NaiveActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let iter = WaitMovingActions::<NaiveActions>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![0, 1]];
    let iter = FilterEnergizedOnWay::<NaiveActions>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let iter = FilterEnergizedOnWay::<WaitMovingActions<NaiveActions>>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![0, 0], vec![0, 1], vec![1, 1]];
    let iter = PermutationalActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let iter = WaitMovingActions::<PermutationalActions>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![0, 1]];
    let iter = FilterEnergizedOnWay::<PermutationalActions>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);

    let iter = FilterEnergizedOnWay::<WaitMovingActions<PermutationalActions>>::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    check_sets(&actions, &expected_actions);
}

#[test]
fn eliminating_cycle_permutations() {
    let graph = get_paper_example_graph(None);
    let buses: Vec<BusState> = vec![
        BusState::Energized,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Energized,
        BusState::Energized,
        BusState::Unknown,
    ];
    // Note that this is not reachable during normal operation except for the initial state (a team
    // positioned on energizable bus), but it is enough for a quick test.
    let teams: Vec<TeamState> = vec![
        TeamState { time: 0, index: 0 },
        TeamState { time: 0, index: 2 },
        TeamState { time: 0, index: 5 },
    ];
    let state = State { buses, teams };

    let eliminated_action = vec![1, 5, 2];

    let iter = NaiveActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    assert!(actions.contains(&eliminated_action));

    let iter = PermutationalActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    assert!(!actions.contains(&eliminated_action));
}

/// Team cannot wait on a bus if all paths to an energy source is broken, i.e., it's not in the
/// beta set.
#[test]
fn cannot_wait_if_no_path() {
    let graph = get_paper_example_graph(None);
    let buses: Vec<BusState> = vec![
        BusState::Energized,
        BusState::Energized,
        BusState::Unknown,
        BusState::Energized,
        BusState::Damaged,
        BusState::Unknown,
    ];
    let teams: Vec<TeamState> = vec![
        TeamState { time: 0, index: 1 },
        TeamState { time: 0, index: 5 },
    ];
    let state = State { buses, teams };

    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![2, 2]];

    let iter = NaiveActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    assert_eq!(actions, expected_actions);

    let iter = PermutationalActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    assert_eq!(actions, expected_actions);
}

/// Checks the action set when all teams are en-route.
#[test]
fn all_enroute_actions() {
    let graph = get_paper_example_graph(None);
    let buses: Vec<BusState> = vec![
        BusState::Energized,
        BusState::Energized,
        BusState::Unknown,
        BusState::Energized,
        BusState::Damaged,
        BusState::Unknown,
    ];
    let teams: Vec<TeamState> = vec![
        TeamState { index: 2, time: 1 },
        TeamState { index: 2, time: 1 },
    ];
    let state = State { buses, teams };

    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![2, 2]];

    let iter = NaiveActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    assert_eq!(actions, expected_actions);

    let iter = PermutationalActions::setup(&graph);
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    assert_eq!(actions, expected_actions);
}

#[test]
fn partition_test() {
    let graph = get_paper_example_graph(Some(vec![vec![0, 1, 2], vec![3, 4, 5]]));
    let iter = SPartActions::setup(&graph);

    let state = State {
        buses: vec![
            BusState::Energized,
            BusState::Unknown,
            BusState::Unknown,
            BusState::Energized,
            BusState::Unknown,
            BusState::Unknown,
        ],
        teams: vec![
            TeamState { time: 0, index: 0 },
            TeamState { time: 0, index: 3 },
        ],
    };
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![1, 4]];
    check_sets(&actions, &expected_actions);

    let state = State {
        buses: vec![
            BusState::Damaged,
            BusState::Unknown,
            BusState::Unknown,
            BusState::Energized,
            BusState::Unknown,
            BusState::Unknown,
        ],
        teams: vec![
            TeamState { time: 0, index: 0 },
            TeamState { time: 0, index: 3 },
        ],
    };
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![0, 4]];
    check_sets(&actions, &expected_actions);

    let state = State {
        buses: vec![
            BusState::Damaged,
            BusState::Unknown,
            BusState::Unknown,
            BusState::Damaged,
            BusState::Unknown,
            BusState::Unknown,
        ],
        teams: vec![
            TeamState { time: 0, index: 0 },
            TeamState { time: 0, index: 3 },
        ],
    };
    let actions: Vec<_> = iter.all_actions_in_state(&state, &graph);
    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![0, 3]];
    check_sets(&actions, &expected_actions);
}
