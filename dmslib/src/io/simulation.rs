use std::time::Instant;

use crate::teams::state::State;

use super::*;

/// Result of taking all possible paths to terminal states in an MDP.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RestorationSimulationResult {
    /// For each bus, energization probability.
    pub bus_energization_p: Vec<f64>,
    /// For each bus, average time until energization (in all paths that energize it).
    pub bus_avg_time: Vec<f64>,
    /// For all buses, energization probability
    pub energization_p: f64,
    /// For all buses, average time until energization (in all paths that energize it).
    pub avg_time: f64,
    /// Number of transitions simulated.
    pub simulated_transitions: usize,
    /// Execution time in seconds.
    pub runtime: f64,
}

impl<T: Transition> TeamSolution<T> {
    /// Simulate a all possible restoration processes starting from the inital state.
    pub fn simulate_all(&self) -> RestorationSimulationResult {
        let start_time = Instant::now();

        let bus_count: usize = self.states.shape()[1];

        let mut result = RestorationSimulationResult {
            bus_energization_p: vec![0.0; bus_count],
            bus_avg_time: vec![0.0; bus_count],
            energization_p: 0.0,
            avg_time: 0.0,
            simulated_transitions: 0,
            runtime: 0.0,
        };

        fn visit<T: Transition>(
            state: State,
            index: usize,
            p: f64,
            time: usize,
            solution: &TeamSolution<T>,
            result: &mut RestorationSimulationResult,
        ) {
            let action_index = solution.policy[index] as usize;
            let action = &solution.transitions[index][action_index];
            if action.len() == 1 && action[0].get_successor() as usize == index {
                // Terminal state
                return;
            }
            for transition in action {
                let successor_index = transition.get_successor() as usize;
                let successor_state = solution.get_state(successor_index);
                let p = p * (transition.get_probability() as f64);
                // This is because costless transition still has time = 1.
                let time = if transition.get_cost() == (0 as Cost) {
                    time
                } else {
                    time + (transition.get_time() as usize)
                };

                for (i, (&a, &b)) in state
                    .buses
                    .iter()
                    .zip(successor_state.buses.iter())
                    .enumerate()
                {
                    if a != b && b == BusState::Energized {
                        result.bus_energization_p[i] += p;
                        result.bus_avg_time[i] += p * (time as f64);
                    }
                }

                visit(successor_state, successor_index, p, time, solution, result);
                result.simulated_transitions += 1;
            }
        }

        visit(self.get_state(0), 0, 1.0, 0, self, &mut result);

        result.energization_p = result.bus_energization_p.iter().sum::<f64>() / (bus_count as f64);
        result.avg_time = result.bus_avg_time.iter().sum::<f64>() / (bus_count as f64);

        result.runtime = start_time.elapsed().as_secs_f64();

        log::info!(
            "Simulated {} transitions in {:.4} seconds",
            result.simulated_transitions,
            result.runtime,
        );

        result
    }
}

impl GenericTeamSolution {
    /// Simulate a all possible restoration processes starting from the inital state.
    pub fn simulate_all(&self) -> RestorationSimulationResult {
        match self {
            GenericTeamSolution::Timed(solution) => solution.simulate_all(),
            GenericTeamSolution::Regular(solution) => solution.simulate_all(),
        }
    }
}
