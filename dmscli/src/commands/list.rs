/// Printing distances, travel times, optimization combinations, etc.
use super::*;

fn print_distances(out: &mut StandardStream, mut problem: TeamProblem, precision: usize) {
    let name = problem.name.take().unwrap_or_else(|| "-".to_string());
    let distances = match problem.get_distances() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Problem Name:     ").unwrap();
    out.reset().unwrap();
    writeln!(out, "{}", name).unwrap();

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Average Distance: ").unwrap();
    out.reset().unwrap();
    let avg: f64 = dmslib::utils::distance_matrix_average(&distances);
    writeln!(out, "{}", avg).unwrap();

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Maximum Distance: ").unwrap();
    out.reset().unwrap();
    writeln!(
        out,
        "{}",
        distances
            .iter()
            .max_by(|a, b| {
                a.partial_cmp(b)
                    .expect("Distance values must be comparable (not NaN)")
            })
            .unwrap()
    )
    .unwrap();

    let (problem, _config) = match problem.prepare() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };
    let neighbor_dists = dmslib::utils::neighbor_distances(&distances, &problem.graph.branches);

    if !neighbor_dists.is_empty() {
        out.set_color(ColorSpec::new().set_bold(true)).unwrap();
        writeln!(out, "Neighbor Distances:").unwrap();
        out.reset().unwrap();

        let min = neighbor_dists
            .iter()
            .min_by(|x, y| x.partial_cmp(y).expect("Distances cannot be compared"))
            .unwrap();
        out.set_color(ColorSpec::new().set_bold(true)).unwrap();
        write!(out, "         Minimum: ").unwrap();
        out.reset().unwrap();
        writeln!(out, "{}", min).unwrap();

        let avg: f64 = neighbor_dists.iter().sum::<f64>() / (neighbor_dists.len() as f64);
        out.set_color(ColorSpec::new().set_bold(true)).unwrap();
        write!(out, "         Average: ").unwrap();
        out.reset().unwrap();
        writeln!(out, "{}", avg).unwrap();

        let max = neighbor_dists
            .iter()
            .max_by(|x, y| x.partial_cmp(y).expect("Distances cannot be compared"))
            .unwrap();
        out.set_color(ColorSpec::new().set_bold(true)).unwrap();
        write!(out, "         Maximum: ").unwrap();
        out.reset().unwrap();
        writeln!(out, "{}", max).unwrap();
    }

    println!("{:.1$}", &distances, precision);
}

fn print_travel_times(out: &mut StandardStream, mut problem: TeamProblem) {
    let name = problem.name.take().unwrap_or_else(|| "-".to_string());
    let (problem, _config) = match problem.prepare() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };
    let travel_times = problem.graph.travel_times;

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Problem Name: ").unwrap();
    out.reset().unwrap();
    writeln!(out, "{}", name).unwrap();

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Average Time: ").unwrap();
    out.reset().unwrap();
    let avg: f64 = dmslib::utils::distance_matrix_average(&travel_times);
    writeln!(out, "{}", avg).unwrap();

    out.set_color(ColorSpec::new().set_bold(true)).unwrap();
    write!(out, "Maximum Time: ").unwrap();
    out.reset().unwrap();
    writeln!(out, "{}", travel_times.iter().max().unwrap()).unwrap();

    println!("{}", &travel_times);
}

pub fn list_all_opt() {
    let result = teams::all_optimizations();
    let serialized = match serde_json::to_string_pretty(&result) {
        Ok(s) => s,
        Err(e) => fatal_error!(1, "Error while serializing results: {}", e),
    };
    println!("{}", serialized);
}

impl TravelTimes {
    pub fn run(self) {
        let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        let TravelTimes { path } = self;
        let problems = match read_problems_from_file(path) {
            Ok(x) => x,
            Err(err) => fatal_error!(1, "Cannot read team problem(s): {}", err),
        };
        for problem in problems {
            print_travel_times(&mut stderr, problem);
        }
    }
}

impl Distances {
    pub fn run(self) {
        let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        let Distances { path, precision } = self;
        let problems = match read_problems_from_file(path) {
            Ok(x) => x,
            Err(err) => fatal_error!(1, "Cannot read team problem(s): {}", err),
        };
        for problem in problems {
            print_distances(&mut stderr, problem, precision);
        }
    }
}
