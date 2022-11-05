use super::*;

fn get_distance_matrix(size: usize) -> Array2<Time> {
    let mut a = ndarray::Array2::<Time>::zeros((size, size));
    for ((x, y), v) in a.indexed_iter_mut() {
        *v = x.abs_diff(y);
    }
    return a;
}

#[test]
fn test_team_times() {
    let graph = Graph {
        travel_times: get_distance_matrix(20),
        branches: vec![],
        connected: vec![],
        pfs: ndarray::arr1(&[]),
        team_nodes: Array2::default((0, 0)),
    };

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(0)
            ],
            &[1, 2, 3],
        ),
        Some(1)
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(0)
            ],
            &[1, 2, 3],
            1,
        ),
        vec![
            TeamState::OnBus(1),
            TeamState::EnRoute(0, 2, 1),
            TeamState::EnRoute(0, 3, 1)
        ],
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(0)
            ],
            &[5, 8, 4],
        ),
        Some(4)
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(0)
            ],
            &[5, 8, 4],
            1,
        ),
        vec![
            TeamState::EnRoute(0, 5, 1),
            TeamState::EnRoute(0, 8, 1),
            TeamState::EnRoute(0, 4, 1)
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(0)
            ],
            &[5, 8, 4],
            4,
        ),
        vec![
            TeamState::EnRoute(0, 5, 4),
            TeamState::EnRoute(0, 8, 4),
            TeamState::OnBus(4)
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(0)
            ],
            &[5, 8, 4],
            5,
        ),
        vec![
            TeamState::OnBus(5),
            TeamState::EnRoute(0, 8, 5),
            TeamState::OnBus(4)
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(0)
            ],
            &[5, 8, 4],
            30,
        ),
        vec![
            TeamState::OnBus(5),
            TeamState::OnBus(8),
            TeamState::OnBus(4)
        ],
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(0)
            ],
            &[0, 19, 0],
        ),
        Some(19)
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(1)
            ],
            &[5, 0, 4],
        ),
        Some(3)
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::EnRoute(1, 4, 2)
            ],
            &[5, 8, 4],
        ),
        Some(1)
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::EnRoute(1, 4, 2)
            ],
            &[0, 8, 4],
        ),
        Some(1)
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::EnRoute(1, 4, 2)
            ],
            &[0, 8, 4],
            1
        ),
        vec![
            TeamState::OnBus(0),
            TeamState::EnRoute(0, 8, 1),
            TeamState::OnBus(4)
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::EnRoute(1, 4, 2)
            ],
            &[0, 8, 4],
            3
        ),
        vec![
            TeamState::OnBus(0),
            TeamState::EnRoute(0, 8, 3),
            TeamState::OnBus(4)
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::EnRoute(1, 4, 2)
            ],
            &[0, 8, 4],
            8
        ),
        vec![
            TeamState::OnBus(0),
            TeamState::OnBus(8),
            TeamState::OnBus(4)
        ],
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[TeamState::EnRoute(1, 4, 2), TeamState::EnRoute(1, 15, 2)],
            &[4, 15],
        ),
        Some(1)
    );

    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::EnRoute(1, 4, 2),
                TeamState::EnRoute(1, 15, 2)
            ],
            &[0, 4, 15],
        ),
        Some(1)
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::EnRoute(1, 4, 2),
                TeamState::EnRoute(1, 15, 2)
            ],
            &[0, 4, 15],
            1
        ),
        vec![
            TeamState::OnBus(0),
            TeamState::OnBus(4),
            TeamState::EnRoute(1, 15, 3)
        ],
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::EnRoute(1, 4, 2),
                TeamState::EnRoute(1, 15, 2)
            ],
            &[0, 4, 15],
            12
        ),
        vec![
            TeamState::OnBus(0),
            TeamState::OnBus(4),
            TeamState::OnBus(15)
        ],
    );

    // All waiting
    assert_eq!(
        min_time_until_arrival(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(1)
            ],
            &[0, 0, 1],
        ),
        None
    );
    assert_eq!(
        advance_time_for_teams(
            &graph,
            &[
                TeamState::OnBus(0),
                TeamState::OnBus(0),
                TeamState::OnBus(1)
            ],
            &[0, 0, 1],
            20
        ),
        vec![
            TeamState::OnBus(0),
            TeamState::OnBus(0),
            TeamState::OnBus(1)
        ],
    );
}
