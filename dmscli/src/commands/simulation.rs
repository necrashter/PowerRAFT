/// Loading solutions and simulating the restoration process.
use dmslib::io::fs::SaveFile;
use std::time::Instant;

use super::*;

impl Load {
    pub fn run(self) {
        // let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        let Load { path } = self;

        let save_file = match dmslib::io::fs::load_solution(path) {
            Ok(s) => s,
            Err(e) => fatal_error!(1, "Error while loading the solution: {}", e),
        };

        let SaveFile { problem, solution } = save_file;

        let start_time = Instant::now();

        let pfs: Vec<f64> = if let Some(pfo) = problem.pfo {
            vec![pfo; problem.graph.nodes.len()]
        } else {
            problem.graph.nodes.iter().map(|node| node.pf).collect()
        };

        let mut transition_count: usize = 0;

        if let dmslib::io::GenericTeamSolution::Timed(mut solution) = solution {
            let mut transitions = Vec::new();
            std::mem::swap(&mut solution.transitions, &mut transitions);
            for (i, state_actions) in transitions.into_iter().enumerate() {
                for action in state_actions {
                    for transition in action {
                        let state = solution.get_state(i);
                        let successor = solution.get_state(transition.successor);

                        state.get_cost();
                        // assert_eq!(state.get_cost(), transition.cost);
                        assert_eq!(state.get_probability(&successor, &pfs), transition.p);
                        // assert_eq!(state.get_time)

                        transition_count += 1;
                    }
                }
            }
        }

        log::info!(
            "Recomputed {} transitions in {:.4} seconds",
            transition_count,
            start_time.elapsed().as_secs_f64()
        );
    }
}
