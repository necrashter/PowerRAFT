use std::time::Instant;

use super::*;

/// This module contains different implementations of structs related to field-teams
/// restoration problem and solution.
///
/// Structs in this module usually have different Serialize and Deserialize implementations
/// than their counterparts in other modules.
/// Some have different internal representation to make the save file smaller.
mod saveable {
    use crate::{teams::state::StateCompressor, Index, Time};
    use bitvec::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub enum TeamState {
        OnBus(Index),
        EnRoute(Index, Index, Time),
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

        pub compressor: StateCompressor,
        pub state_count: usize,
        pub states: BitVec,
        pub transitions: Vec<Vec<Vec<T>>>,

        pub values: Vec<Vec<f64>>,
        pub policy: Vec<usize>,
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

            let team_node_count = team_nodes.shape()[0];
            let team_nodes = team_nodes.into_raw_vec();
            let travel_times = travel_times.into_raw_vec();

            let bus_count = states.shape()[1];
            let team_count = teams.shape()[1];
            let max_time = *travel_times
                .iter()
                .max()
                .expect("Cannot get max travel time");
            let compressor = StateCompressor::new(bus_count, team_count, max_time);

            let states = compressor.compress(states, teams);

            TeamSolution {
                total_time,
                generation_time,
                max_memory,
                team_node_count,
                team_nodes,
                travel_times,
                compressor,
                state_count: transitions.len(),
                states,
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
                compressor,
                state_count,
                states,
                transitions,
                values,
                policy,
                horizon,
            } = $a;

            let (states, teams) = compressor.decompress(states, state_count);

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
                states,
                teams,
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
        pub successor: usize,
        pub p: f64,
        pub cost: f64,
    }

    #[derive(Serialize, Deserialize)]
    pub struct TimedTransition {
        pub successor: usize,
        pub p: f64,
        pub cost: f64,
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
