use dmslib::io::TeamProblem;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap();
    let action_set = args.next().unwrap();

    println!("Solving team problem: {}", path);
    let problem = match TeamProblem::read_from_file(&path) {
        Ok(x) => x,
        Err(err) => {
            eprintln!("Cannot read team problem: {}", err);
            return;
        }
    };
    let solution = match problem.solve_custom(&action_set) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("Cannot solve team problem: {}", err);
            return;
        }
    };
    println!("Number of states: {}", solution.transitions.len());
    println!("Generation time: {}", solution.generation_time);
    println!("Total time: {}", solution.total_time);
}
