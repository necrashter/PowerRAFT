use super::*;

#[test]
fn linear_system_energize() {
    // 4 bus system, everything in on a line
    let graph = Graph {
        // Energize does not really care about this
        branches: vec![vec![1], vec![2], vec![3], vec![4]],
        connected: vec![true, false, false, false],
        pfs: ndarray::arr1(&[0.5, 0.75, 0.5, 0.5]),
    };

    let bus_state = vec![
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
        BusState::Unknown,
    ];

    assert_eq!(
        energize(&graph, bus_state.clone(), &[0]),
        vec![
            (
                0.5,
                vec![
                    BusState::Energized,
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Unknown,
                ]
            ),
            (
                0.5,
                vec![
                    BusState::Damaged,
                    BusState::Unknown,
                    BusState::Unknown,
                    BusState::Unknown,
                ]
            ),
        ],
    );

    assert_eq!(
        energize(&graph, bus_state.clone(), &[0, 1]),
        vec![
            (
                0.5 * 0.25,
                vec![
                    BusState::Energized,
                    BusState::Energized,
                    BusState::Unknown,
                    BusState::Unknown,
                ]
            ),
            (
                0.5 * 0.25,
                vec![
                    BusState::Damaged,
                    BusState::Energized,
                    BusState::Unknown,
                    BusState::Unknown,
                ]
            ),
            (
                0.5 * 0.75,
                vec![
                    BusState::Energized,
                    BusState::Damaged,
                    BusState::Unknown,
                    BusState::Unknown,
                ]
            ),
            (
                0.5 * 0.75,
                vec![
                    BusState::Damaged,
                    BusState::Damaged,
                    BusState::Unknown,
                    BusState::Unknown,
                ]
            ),
        ],
    );
}
