use super::*;

fn get_paper_example_graph() -> Graph {
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
    let graph = get_paper_example_graph();
    let buses: Vec<BusState> = vec![
        BusState::Energized,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Energized,
        BusState::Damaged,
        BusState::Unknown,
    ];
    let teams: Vec<TeamState> = vec![TeamState::OnBus(0), TeamState::EnRoute(4, 2, 1)];
    let state = State { buses, teams };

    let cost = state.get_cost();
    assert_eq!(cost, 4.0);

    let mut iter = NaiveIterator::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();

    assert_eq!(actions, vec![vec![1, -2]]);

    let expected_team_outcome: Vec<TeamState> = vec![TeamState::OnBus(1), TeamState::OnBus(2)];
    let expected_outcomes: Vec<(f64, State)> = vec![
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
                teams: expected_team_outcome.clone(),
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
    let outcomes: Vec<(f64, State)> = NaiveActionApplier::apply(&state, cost, &graph, &actions[0])
        .into_iter()
        .map(|(transition, state)| {
            assert_eq!(transition.cost, cost);
            (transition.p, state)
        })
        .collect();
    check_sets(&outcomes, &expected_outcomes);
}

#[test]
fn on_energized_bus_actions() {
    let graph = get_paper_example_graph();
    let buses: Vec<BusState> = vec![
        BusState::Energized,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Energized,
        BusState::Unknown,
        BusState::Unknown,
    ];
    let teams: Vec<TeamState> = vec![TeamState::OnBus(0), TeamState::OnBus(3)];
    let state = State { buses, teams };

    assert_eq!(state.get_cost(), 4.0);

    let mut iter = NaiveIterator::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
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

    let mut iter = WaitMovingIterator::<NaiveIterator>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
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

    let mut iter = OnWayIterator::<NaiveIterator>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let mut iter = WaitMovingIterator::<OnWayIterator<NaiveIterator>>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
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

    let mut iter = PermutationalIterator::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let mut iter = WaitMovingIterator::<PermutationalIterator>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let expected_actions: Vec<Vec<TeamAction>> =
        vec![vec![1, 1], vec![1, 2], vec![1, 4], vec![4, 4], vec![5, 4]];

    let mut iter = OnWayIterator::<PermutationalIterator>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let mut iter = WaitMovingIterator::<OnWayIterator<PermutationalIterator>>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);
}

#[test]
fn wait_moving_elimination() {
    let graph = get_paper_example_graph();
    let buses: Vec<BusState> = vec![
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Energized,
        BusState::Energized,
        BusState::Energized,
    ];
    let teams: Vec<TeamState> = vec![TeamState::OnBus(2), TeamState::EnRoute(5, 0, 1)];
    let state = State { buses, teams };

    assert_eq!(state.get_cost(), 3.0);

    let expected_actions: Vec<Vec<TeamAction>> = vec![
        vec![WAIT_ACTION, CONTINUE_ACTION],
        vec![0, CONTINUE_ACTION],
        vec![1, CONTINUE_ACTION],
    ];

    let mut iter = NaiveIterator::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let mut iter = OnWayIterator::<NaiveIterator>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let mut iter = WaitMovingIterator::<NaiveIterator>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &vec![vec![WAIT_ACTION, CONTINUE_ACTION]]);

    let mut iter = OnWayIterator::<WaitMovingIterator<NaiveIterator>>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &vec![vec![WAIT_ACTION, CONTINUE_ACTION]]);
}

#[test]
fn beta_values_on_paper_example() {
    let graph = get_paper_example_graph();
    let dummy_teams = vec![TeamState::OnBus(0)];

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
    assert_eq!(state.minbetas(&graph), vec![0, 1, 2, 0, 0, usize::MAX]);

    let state = State {
        buses: vec![BusState::Unknown; 6],
        teams: dummy_teams.clone(),
    };
    assert_eq!(state.minbetas(&graph), vec![1, 2, 3, 1, 2, 3]);

    let state = State {
        buses: vec![
            BusState::Damaged,
            BusState::Unknown,
            BusState::Unknown,
            BusState::Damaged,
            BusState::Unknown,
            BusState::Unknown,
        ],
        teams: dummy_teams.clone(),
    };
    assert_eq!(
        state.minbetas(&graph),
        vec![0, usize::MAX, usize::MAX, 0, usize::MAX, usize::MAX,]
    );
}

#[test]
fn minimal_nonopt_permutations() {
    let graph = Graph {
        travel_times: ndarray::arr2(&[[0, 1, 1, 2], [1, 0, 2, 1], [1, 2, 0, 1], [2, 1, 1, 0]]),
        branches: vec![vec![], vec![]],
        connected: vec![true, true],
        pfs: ndarray::arr1(&[0.5, 0.5]),
    };

    let state = State {
        buses: vec![BusState::Unknown, BusState::Unknown],
        teams: vec![TeamState::OnBus(2), TeamState::OnBus(3)],
    };

    assert_eq!(state.minbetas(&graph), vec![1, 1]);

    let expected_actions: Vec<Vec<TeamAction>> =
        vec![vec![0, 0], vec![1, 0], vec![0, 1], vec![1, 1]];
    let mut iter = NaiveIterator::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let mut iter = WaitMovingIterator::<NaiveIterator>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![0, 1]];
    let mut iter = OnWayIterator::<NaiveIterator>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let mut iter = OnWayIterator::<WaitMovingIterator<NaiveIterator>>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![0, 0], vec![0, 1], vec![1, 1]];
    let mut iter = PermutationalIterator::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let mut iter = WaitMovingIterator::<PermutationalIterator>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let expected_actions: Vec<Vec<TeamAction>> = vec![vec![0, 1]];
    let mut iter = OnWayIterator::<PermutationalIterator>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);

    let mut iter = OnWayIterator::<WaitMovingIterator<PermutationalIterator>>::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    check_sets(&actions, &expected_actions);
}

#[test]
fn eliminating_cycle_permutations() {
    let graph = get_paper_example_graph();
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
        TeamState::OnBus(0),
        TeamState::OnBus(2),
        TeamState::OnBus(5),
    ];
    let state = State { buses, teams };

    let eliminated_action = vec![1, 5, 2];

    let mut iter = NaiveIterator::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    assert!(actions.contains(&eliminated_action));
    for action in &actions {
        // Ensure that no team is sent to the bus on which it's standing
        assert_ne!(action[1], 2);
        assert_ne!(action[2], 5);
    }

    let mut iter = PermutationalIterator::setup(&graph);
    let actions: Vec<_> = iter.prepare_from_state(&state, &graph).collect();
    assert!(!actions.contains(&eliminated_action));
    for action in &actions {
        // Ensure that no team is sent to the bus on which it's standing
        assert_ne!(action[1], 2);
        assert_ne!(action[2], 5);
    }
}
