use super::*;

fn get_distance_matrix(size: usize) -> Array2<Time> {
    let mut a = ndarray::Array2::<Time>::zeros((size, size));
    for ((x, y), v) in a.indexed_iter_mut() {
        *v = x.abs_diff(y) as Time;
    }
    a
}

#[test]
fn test_min_time_until_arrival() {
    let graph = Graph {
        travel_times: get_distance_matrix(20),
        branches: vec![],
        connected: vec![],
        pfs: ndarray::arr1(&[]),
        team_nodes: Array2::default((0, 0)),
        partitions: None,
    };

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 }
            ],
            &[1, 2, 3],
        ),
        Some(1)
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 }
            ],
            &[1, 2, 3],
            1,
        ),
        vec![
            TeamState { time: 0, index: 1 },
            TeamState { index: 2, time: 1 },
            TeamState { index: 3, time: 2 }
        ],
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 }
            ],
            &[5, 8, 4],
        ),
        Some(4)
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 }
            ],
            &[5, 8, 4],
            1,
        ),
        vec![
            TeamState { index: 5, time: 4 },
            TeamState { index: 8, time: 7 },
            TeamState { index: 4, time: 3 }
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 }
            ],
            &[5, 8, 4],
            4,
        ),
        vec![
            TeamState { index: 5, time: 1 },
            TeamState { index: 8, time: 4 },
            TeamState { time: 0, index: 4 }
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 }
            ],
            &[5, 8, 4],
            5,
        ),
        vec![
            TeamState { time: 0, index: 5 },
            TeamState { index: 8, time: 3 },
            TeamState { time: 0, index: 4 }
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 }
            ],
            &[5, 8, 4],
            30,
        ),
        vec![
            TeamState { time: 0, index: 5 },
            TeamState { time: 0, index: 8 },
            TeamState { time: 0, index: 4 }
        ],
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 }
            ],
            &[0, 19, 0],
        ),
        Some(19)
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 1 }
            ],
            &[5, 0, 4],
        ),
        Some(3)
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { index: 4, time: 1 }
            ],
            &[5, 8, 4],
        ),
        Some(1)
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { index: 4, time: 1 }
            ],
            &[0, 8, 4],
        ),
        Some(1)
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { index: 4, time: 1 }
            ],
            &[0, 8, 4],
            1
        ),
        vec![
            TeamState { time: 0, index: 0 },
            TeamState { index: 8, time: 7 },
            TeamState { time: 0, index: 4 }
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { index: 4, time: 1 }
            ],
            &[0, 8, 4],
            3
        ),
        vec![
            TeamState { time: 0, index: 0 },
            TeamState { index: 8, time: 5 },
            TeamState { time: 0, index: 4 }
        ],
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState { index: 4, time: 1 },
                TeamState {
                    index: 15,
                    time: 12
                }
            ],
            &[4, 15],
        ),
        Some(1)
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { index: 4, time: 1 },
                TeamState {
                    index: 15,
                    time: 12
                },
            ],
            &[0, 4, 15],
        ),
        Some(1)
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { index: 4, time: 1 },
                TeamState {
                    index: 15,
                    time: 12
                },
            ],
            &[0, 4, 15],
            1
        ),
        vec![
            TeamState { time: 0, index: 0 },
            TeamState { time: 0, index: 4 },
            TeamState {
                index: 15,
                time: 11
            },
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { index: 4, time: 1 },
                TeamState {
                    index: 15,
                    time: 12
                },
            ],
            &[0, 4, 15],
            12
        ),
        vec![
            TeamState { time: 0, index: 0 },
            TeamState { time: 0, index: 4 },
            TeamState { time: 0, index: 15 },
        ],
    );

    // All waiting
    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 1 }
            ],
            &[0, 0, 1],
        ),
        None
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 0 },
                TeamState { time: 0, index: 1 }
            ],
            &[0, 0, 1],
            20
        ),
        vec![
            TeamState { time: 0, index: 0 },
            TeamState { time: 0, index: 0 },
            TeamState { time: 0, index: 1 }
        ],
    );
}

/// 10 bus system, everything in on a line
fn ten_bus_linear_system() -> (Graph, Vec<BusState>) {
    let graph = Graph {
        travel_times: get_distance_matrix(10),
        branches: vec![
            vec![1],
            vec![2],
            vec![3],
            vec![4],
            vec![5],
            vec![6],
            vec![7],
            vec![8],
            vec![9],
            vec![],
        ],
        connected: vec![
            true, false, false, false, false, false, false, false, false, false,
        ],
        pfs: ndarray::arr1(&[0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
        team_nodes: Array2::default((0, 0)),
        partitions: None,
    };

    let bus_state = vec![
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
    ];

    (graph, bus_state)
}

#[test]
fn test_time_until_energization() {
    let (graph, bus_state) = ten_bus_linear_system();

    assert_eq!(
        TimeUntilArrival::get_time_state(
            &graph,
            State {
                buses: bus_state.clone(),
                teams: vec![
                    TeamState { time: 0, index: 3 },
                    TeamState { index: 2, time: 1 },
                    TeamState { index: 0, time: 4 },
                ],
            },
            &[0, 2, 0]
        ),
        1
    );

    assert_eq!(
        TimeUntilEnergization::get_time_state(
            &graph,
            State {
                buses: bus_state.clone(),
                teams: vec![
                    TeamState { time: 0, index: 3 },
                    TeamState { index: 2, time: 1 },
                    TeamState { index: 0, time: 4 },
                ],
            },
            &[0, 2, 0]
        ),
        3
    );

    assert_eq!(
        TimeUntilEnergization::get_time_state(
            &graph,
            State {
                buses: bus_state.clone(),
                teams: vec![
                    TeamState { time: 0, index: 3 },
                    TeamState { index: 2, time: 1 },
                    TeamState { index: 0, time: 2 },
                ],
            },
            &[0, 2, 0]
        ),
        2
    );

    assert_eq!(
        TimeUntilArrival::get_time_state(
            &graph,
            State {
                buses: bus_state.clone(),
                teams: vec![
                    TeamState { time: 0, index: 3 },
                    TeamState { index: 0, time: 2 },
                ],
            },
            &[0, 0]
        ),
        2
    );

    assert_eq!(
        ConstantTime::get_time_state(
            &graph,
            State {
                buses: bus_state,
                teams: vec![
                    TeamState { time: 0, index: 3 },
                    TeamState { index: 0, time: 2 },
                ],
            },
            &[0, 0]
        ),
        1
    );
}

#[test]
fn test_time_until_arrival_progress() {
    let (graph, bus_state) = ten_bus_linear_system();
    assert_eq!(
        TimeUntilArrival::get_time_state(
            &graph,
            State {
                buses: bus_state,
                teams: vec![
                    TeamState { time: 0, index: 3 },
                    TeamState { index: 2, time: 1 },
                    TeamState { index: 6, time: 2 },
                ],
            },
            &[1, 2, 6]
        ),
        1
    );
}

#[test]
#[should_panic(expected = "progress condition")]
fn test_time_until_energization_progress() {
    let (graph, bus_state) = ten_bus_linear_system();
    let _time = TimeUntilEnergization::get_time_state(
        &graph,
        State {
            buses: bus_state,
            teams: vec![
                TeamState { time: 0, index: 3 },
                TeamState { index: 2, time: 1 },
                TeamState { index: 6, time: 2 },
            ],
        },
        &[1, 2, 6],
    );
}
