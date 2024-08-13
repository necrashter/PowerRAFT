//! Integration tests
//!
//! Test MDP construction and policy synthesis as a whole.

use super::*;

const SYSTEM_PAPER_EXAMPLE_0: &str = include_str!("../../../graphs/FieldTeams/paperE0.json");
const WSCC: &str = include_str!("../../../graphs/wscc.json");

#[test]
fn pe0_nonteam() {
    let input_graph: io::Graph = serde_json::from_str(SYSTEM_PAPER_EXAMPLE_0).unwrap();
    let team_problem = crate::io::TeamProblem {
        name: None,
        graph: input_graph,
        teams: vec![],
        pfo: None,
        horizon: Some(30),
        time_func: io::TimeFunc::default(),
    };
    let (graph, config) = team_problem.prepare_nonteam().unwrap();
    const OPTIMAL_VALUE: Value = unsafe { std::mem::transmute(0x43074980_u32) };

    let solution = solve_naive(&graph, &config).unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    assert_eq!(solution.transitions.len(), 126);
}

#[test]
fn wscc_nonteam() {
    let input_graph: io::Graph = serde_json::from_str(WSCC).unwrap();
    let team_problem = crate::io::TeamProblem {
        name: None,
        graph: input_graph,
        teams: vec![],
        pfo: None,
        horizon: Some(30),
        time_func: io::TimeFunc::default(),
    };
    let (graph, config) = team_problem.prepare_nonteam().unwrap();
    const OPTIMAL_VALUE: Value = unsafe { std::mem::transmute(0x42bf3a4a_u32) };

    let solution = solve_naive(&graph, &config).unwrap();
    assert_eq!(solution.get_min_value(), OPTIMAL_VALUE);
    // After team representations were updated, this reduced from 645 to 593
    assert_eq!(solution.transitions.len(), 409);
}
