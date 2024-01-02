use std::time::Instant;

use super::*;

/// This module contains different implementations of structs related to field-teams
/// restoration problem and solution.
///
/// Structs in this module usually have different Serialize and Deserialize implementations
/// than their counterparts in other modules.
/// Some have different internal representation to make the save file smaller.
mod saveable {
    use crate::types::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct TeamState {
        pub time: Time,
        pub index: BusIndex,
    }

    #[derive(Serialize, Deserialize)]
    pub enum BusState {
        Unknown = 0,
        Damaged = 1,
        Energized = 2,
    }

    #[derive(Serialize, Deserialize)]
    pub struct TeamSolution<T> {
        pub total_time: f64,
        pub generation_time: f64,
        pub max_memory: usize,

        pub team_node_count: usize,
        pub team_nodes: Vec<f64>,
        pub travel_times: Vec<Time>,

        pub states: Vec<BusState>,
        pub teams: Vec<TeamState>,
        pub transitions: Vec<Vec<Vec<T>>>,

        pub values: Vec<Vec<Value>>,
        pub policy: Vec<ActionIndex>,
        pub horizon: usize,
    }

    macro_rules! super_to_saveable {
        ($a:expr) => {{
            let super::TeamSolution {
                total_time,
                generation_time,
                max_memory,
                team_nodes,
                travel_times,
                states,
                teams,
                transitions,
                values,
                policy,
                horizon,
            } = $a;
            TeamSolution {
                total_time,
                generation_time,
                max_memory,
                team_node_count: team_nodes.shape()[0],
                team_nodes: team_nodes.into_raw_vec(),
                travel_times: travel_times.into_raw_vec(),
                states: unsafe { std::mem::transmute(states.into_raw_vec()) },
                teams: unsafe { std::mem::transmute(teams.into_raw_vec()) },
                transitions: unsafe { std::mem::transmute(transitions) },
                values,
                policy,
                horizon,
            }
        }};
    }

    impl From<super::TeamSolution<super::RegularTransition>> for TeamSolution<RegularTransition> {
        fn from(value: super::TeamSolution<super::RegularTransition>) -> Self {
            super_to_saveable!(value)
        }
    }

    impl From<super::TeamSolution<super::TimedTransition>> for TeamSolution<TimedTransition> {
        fn from(value: super::TeamSolution<super::TimedTransition>) -> Self {
            super_to_saveable!(value)
        }
    }

    macro_rules! saveable_to_super {
        ($a:expr) => {{
            let TeamSolution {
                total_time,
                generation_time,
                max_memory,
                team_node_count,
                team_nodes,
                travel_times,
                states,
                teams,
                transitions,
                values,
                policy,
                horizon,
            } = $a;
            let state_count = transitions.len();
            let bus_count = states.len() / state_count;
            let team_count = teams.len() / state_count;
            super::TeamSolution {
                total_time,
                generation_time,
                max_memory,
                team_nodes: ndarray::Array::from_vec(team_nodes)
                    .into_shape((team_node_count, 2))
                    .unwrap(),
                travel_times: ndarray::Array::from_vec(travel_times)
                    .into_shape((team_node_count, team_node_count))
                    .unwrap(),
                states: ndarray::Array::from_vec(unsafe {
                    std::mem::transmute::<Vec<BusState>, Vec<super::BusState>>(states)
                })
                .into_shape((state_count, bus_count))
                .unwrap(),
                teams: ndarray::Array::from_vec(unsafe {
                    std::mem::transmute::<Vec<TeamState>, Vec<super::TeamState>>(teams)
                })
                .into_shape((state_count, team_count))
                .unwrap(),
                transitions: unsafe { std::mem::transmute(transitions) },
                values,
                policy,
                horizon,
            }
        }};
    }

    impl From<TeamSolution<RegularTransition>> for super::TeamSolution<super::RegularTransition> {
        fn from(value: TeamSolution<RegularTransition>) -> Self {
            saveable_to_super!(value)
        }
    }

    impl From<TeamSolution<TimedTransition>> for super::TeamSolution<super::TimedTransition> {
        fn from(value: TeamSolution<TimedTransition>) -> Self {
            saveable_to_super!(value)
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct RegularTransition {
        pub successor: StateIndex,
        pub p: Probability,
        pub cost: Cost,
    }

    #[derive(Serialize, Deserialize)]
    pub struct TimedTransition {
        pub successor: StateIndex,
        pub p: Probability,
        pub cost: Cost,
        pub time: Time,
    }

    #[derive(Serialize, Deserialize)]
    pub enum GenericTeamSolution {
        Timed(TeamSolution<TimedTransition>),
        Regular(TeamSolution<RegularTransition>),
    }

    #[derive(Serialize, Deserialize)]
    pub enum TimeFunc {
        DirectDistance {
            multiplier: Option<f64>,
            divider: Option<f64>,
        },
        Constant {
            constant: Time,
        },
    }

    #[derive(Serialize, Deserialize)]
    pub struct TeamProblem {
        pub name: Option<String>,
        pub graph: super::Graph,
        pub teams: Vec<super::Team>,
        pub horizon: Option<usize>,
        pub pfo: Option<f64>,
        pub time_func: TimeFunc,
    }

    impl From<TeamProblem> for super::TeamProblem {
        fn from(value: TeamProblem) -> Self {
            unsafe { std::mem::transmute(value) }
        }
    }

    impl From<super::TeamProblem> for TeamProblem {
        fn from(value: super::TeamProblem) -> Self {
            unsafe { std::mem::transmute(value) }
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct SaveFile {
        pub problem: TeamProblem,
        pub solution: GenericTeamSolution,
    }
}

impl From<GenericTeamSolution> for saveable::GenericTeamSolution {
    fn from(value: GenericTeamSolution) -> Self {
        match value {
            GenericTeamSolution::Timed(a) => saveable::GenericTeamSolution::Timed(a.into()),
            GenericTeamSolution::Regular(a) => saveable::GenericTeamSolution::Regular(a.into()),
        }
    }
}

impl From<saveable::GenericTeamSolution> for GenericTeamSolution {
    fn from(value: saveable::GenericTeamSolution) -> Self {
        match value {
            saveable::GenericTeamSolution::Timed(a) => GenericTeamSolution::Timed(a.into()),
            saveable::GenericTeamSolution::Regular(a) => GenericTeamSolution::Regular(a.into()),
        }
    }
}

impl From<TeamSolution<TimedTransition>> for saveable::GenericTeamSolution {
    fn from(value: TeamSolution<TimedTransition>) -> Self {
        saveable::GenericTeamSolution::Timed(value.into())
    }
}

impl From<TeamSolution<RegularTransition>> for saveable::GenericTeamSolution {
    fn from(value: TeamSolution<RegularTransition>) -> Self {
        saveable::GenericTeamSolution::Regular(value.into())
    }
}

/// Struct that represents the contents of a save file.
pub struct SaveFile {
    pub problem: TeamProblem,
    pub solution: GenericTeamSolution,
}

use bincode::Options;

macro_rules! bincode_options {
    () => {{
        bincode::DefaultOptions::new()
    }};
}

/// Save the field-teams restoration problem and solution to the given file.
pub fn save_solution<P: AsRef<Path>, S: Into<saveable::GenericTeamSolution>>(
    problem: TeamProblem,
    solution: S,
    path: P,
) -> std::io::Result<()> {
    let start_time = Instant::now();

    let file_content = saveable::SaveFile {
        problem: problem.into(),
        solution: solution.into(),
    };

    let encoded: Vec<u8> = match bincode_options!().serialize(&file_content) {
        Ok(v) => v,
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    };

    let mut file = std::fs::File::create(&path)?;
    file.write_all(&encoded[..])?;

    log::info!(
        "Saved {} bytes to {} in {:.4} seconds.",
        encoded.len(),
        path.as_ref().to_string_lossy().to_string(),
        start_time.elapsed().as_secs_f64()
    );

    Ok(())
}

/// Load the field-teams restoration problem and solution from the given file.
pub fn load_solution<P: AsRef<Path>>(path: P) -> std::io::Result<SaveFile> {
    let start_time = Instant::now();

    let mut file = std::fs::File::open(&path)?;
    let mut encoded: Vec<u8> = Vec::new();
    file.read_to_end(&mut encoded)?;

    let decoded: saveable::SaveFile = match bincode_options!().deserialize(&encoded[..]) {
        Ok(v) => v,
        Err(e) => {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    };

    let saveable::SaveFile { problem, solution } = decoded;

    let output = SaveFile {
        problem: problem.into(),
        solution: solution.into(),
    };

    log::info!(
        "Loaded {} bytes from {} in {:.4} seconds.",
        encoded.len(),
        path.as_ref().to_string_lossy().to_string(),
        start_time.elapsed().as_secs_f64()
    );

    Ok(output)
}

#[cfg(test)]
mod tests {
    use crate::teams::{solve_custom_regular, solve_custom_timed};

    use super::*;

    const TEST_GRAPH: &str = include_str!("../../../../graphs/FieldTeams/paperE0.json");

    fn encode_decode_save_file(save_file: SaveFile) {
        let file_content = saveable::SaveFile {
            problem: save_file.problem.clone().into(),
            solution: save_file.solution.clone().into(),
        };

        let encoded = bincode_options!().serialize(&file_content).unwrap();
        let decoded: saveable::SaveFile = bincode_options!().deserialize(&encoded[..]).unwrap();

        let reconstructed = SaveFile {
            problem: decoded.problem.into(),
            solution: decoded.solution.into(),
        };

        assert_eq!(save_file.problem, reconstructed.problem);
        assert_eq!(save_file.solution, reconstructed.solution);
    }

    #[test]
    fn solution_binary_save_test() {
        let input_graph: Graph = serde_json::from_str(TEST_GRAPH).unwrap();
        let teams = vec![Team {
            index: Some(0),
            latlng: None,
        }];
        let (problem, config) = input_graph
            .clone()
            .to_teams_problem(teams.clone(), Some(30))
            .unwrap();
        let team_problem = TeamProblem {
            name: Some("Save Test".to_string()),
            graph: input_graph,
            teams,
            horizon: Some(30),
            pfo: None,
            time_func: TimeFunc::default(),
        };

        let solution = solve_custom_timed(
            &problem.graph,
            problem.initial_teams.clone(),
            &config,
            "NaiveStateIndexer",
            "NaiveActions",
            "TimedActionApplier<TimeUntilEnergization>",
            "NaiveExplorer",
        )
        .unwrap();

        encode_decode_save_file(SaveFile {
            problem: team_problem.clone(),
            solution: GenericTeamSolution::Timed(solution.into_io(&problem.graph)),
        });

        let solution = solve_custom_regular(
            &problem.graph,
            problem.initial_teams.clone(),
            &config,
            "NaiveStateIndexer",
            "NaiveActions",
            "NaiveExplorer",
        )
        .unwrap();
        encode_decode_save_file(SaveFile {
            problem: team_problem.clone(),
            solution: GenericTeamSolution::Regular(solution.into_io(&problem.graph)),
        });
    }
}
