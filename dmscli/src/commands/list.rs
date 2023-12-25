/// Printing distances, travel times, optimization combinations, etc.
use super::*;

fn print_distances(mut problem: TeamProblem, precision: usize) {
    let name = problem.name.take().unwrap_or_else(|| "-".to_string());
    let distances = match problem.get_distances() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };

    println!("{:18}{}", "Problem Name:".bold(), name);

    let avg: f64 = dmslib::utils::distance_matrix_average(&distances);
    println!("{:18}{}", "Average Distance:".bold(), avg);

    println!(
        "{:18}{}",
        "Maximum Distance:".bold(),
        distances
            .iter()
            .max_by(|a, b| {
                a.partial_cmp(b)
                    .expect("Distance values must be comparable (not NaN)")
            })
            .unwrap()
    );

    let (problem, _config) = match problem.prepare() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };
    let neighbor_dists = dmslib::utils::neighbor_distances(&distances, &problem.graph.branches);

    if !neighbor_dists.is_empty() {
        println!("{}", "Neighbor Distances:".bold());

        let min = neighbor_dists
            .iter()
            .min_by(|x, y| x.partial_cmp(y).expect("Distances cannot be compared"))
            .unwrap();
        println!("{:>18}{}", "Minimum: ".bold(), min);

        let avg: f64 = neighbor_dists.iter().sum::<f64>() / (neighbor_dists.len() as f64);
        println!("{:>18}{}", "Average: ".bold(), avg);

        let max = neighbor_dists
            .iter()
            .max_by(|x, y| x.partial_cmp(y).expect("Distances cannot be compared"))
            .unwrap();
        println!("{:>18}{}", "Maximum: ".bold(), max);
    }

    println!("{:.1$}", &distances, precision);
}

fn print_travel_times(mut problem: TeamProblem) {
    let name = problem.name.take().unwrap_or_else(|| "-".to_string());
    let (problem, _config) = match problem.prepare() {
        Ok(x) => x,
        Err(err) => fatal_error!(1, "Error while parsing team problem: {}", err),
    };
    let travel_times = problem.graph.travel_times;

    println!("{:14}{}", "Problem Name:".bold(), name);

    let avg: f64 = dmslib::utils::distance_matrix_average(&travel_times);
    println!("{:14}{}", "Average Time:".bold(), avg);

    println!(
        "{:14}{}",
        "Maximum Time:".bold(),
        travel_times.iter().max().unwrap()
    );

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
        let TravelTimes { path } = self;

        let problems = match read_problems_from_file(path) {
            Ok(x) => x,
            Err(err) => fatal_error!(1, "Cannot read team problem(s): {}", err),
        };
        for problem in problems {
            print_travel_times(problem);
        }
    }
}

impl Distances {
    pub fn run(self) {
        let Distances { path, precision } = self;

        let problems = match read_problems_from_file(path) {
            Ok(x) => x,
            Err(err) => fatal_error!(1, "Cannot read team problem(s): {}", err),
        };
        for problem in problems {
            print_distances(problem, precision);
        }
    }
}
