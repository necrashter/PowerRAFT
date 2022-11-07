use dmslib::io::TeamProblem;

fn main() {
    for path in std::env::args().skip(1) {
        println!();
        println!("Solving team problem: {}", path);
        let problem = match TeamProblem::read_from_file(&path) {
            Ok(x) => x,
            Err(err) => {
                eprintln!("Cannot read team problem: {}", err);
                continue;
            }
        };
        let solution = match problem.solve_naive() {
            Ok(s) => s,
            Err(err) => {
                eprintln!("Cannot solve team problem: {}", err);
                continue;
            }
        };
        println!("Number of states: {}", solution.transitions.len());
        println!("Generation time: {}", solution.generation_time);
        println!("Total time: {}", solution.total_time);
    }
}
