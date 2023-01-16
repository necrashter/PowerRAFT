//! Various utility functions.

use ndarray::Array2;
use num_traits::{ToPrimitive, Unsigned};

/// Given 2 sorted iterators, returns true if at least one element is common.
pub fn sorted_intersects<'a, T, IT>(mut a: IT, mut b: IT) -> bool
where
    T: Ord + 'a,
    IT: 'a + Iterator<Item = &'a T>,
{
    let mut x: &T = if let Some(value) = a.next() {
        value
    } else {
        return false;
    };
    let mut y: &T = if let Some(value) = b.next() {
        value
    } else {
        return false;
    };
    loop {
        match x.cmp(y) {
            std::cmp::Ordering::Less => {
                if let Some(value) = a.next() {
                    x = value;
                } else {
                    return false;
                }
            }
            std::cmp::Ordering::Equal => {
                return true;
            }
            std::cmp::Ordering::Greater => {
                if let Some(value) = b.next() {
                    y = value;
                } else {
                    return false;
                }
            }
        }
    }
}

/// Given 2 sorted vectors, returns a vector of common elements in sorted order.
pub fn sorted_intersection<T: Ord + Clone>(a: &Vec<T>, b: &Vec<T>) -> Vec<T> {
    let mut output: Vec<T> = Vec::new();
    output.reserve_exact(std::cmp::min(a.len(), b.len()));
    let mut a = a.iter().cloned();
    let mut b = b.iter().cloned();
    let mut x: T = if let Some(value) = a.next() {
        value
    } else {
        return output;
    };
    let mut y: T = if let Some(value) = b.next() {
        value
    } else {
        return output;
    };
    loop {
        match x.cmp(&y) {
            std::cmp::Ordering::Less => {
                if let Some(value) = a.next() {
                    x = value;
                } else {
                    break;
                }
            }
            std::cmp::Ordering::Equal => {
                output.push(x);
                if let Some(value) = a.next() {
                    x = value;
                } else {
                    break;
                }
                if let Some(value) = b.next() {
                    y = value;
                } else {
                    break;
                }
            }
            std::cmp::Ordering::Greater => {
                if let Some(value) = b.next() {
                    y = value;
                } else {
                    break;
                }
            }
        }
    }
    output
}

/// Detect if a cycle exists in the given directed graph with DFS.
/// `edges` is a **lexicographically sorted** list of edges, for example `[(0,1), (0,2), (1,2)]`.
pub fn is_graph_cyclic(vertex_count: usize, edges: &Vec<(usize, usize)>) -> bool {
    #[derive(Clone, PartialEq, Eq)]
    enum VisitStatus {
        Unvisited,
        Visiting,
        Visited,
    }
    let mut status = vec![VisitStatus::Unvisited; vertex_count];
    fn visit(edges: &Vec<(usize, usize)>, status: &mut Vec<VisitStatus>, i: usize) -> bool {
        match status[i] {
            VisitStatus::Unvisited => {
                // Visit node
                status[i] = VisitStatus::Visiting;
                let mut edge_i = {
                    // Binary search to find where the edges of this vertex start
                    // Equivalent to C++ lower bound
                    let mut first = 0;
                    let mut count = edges.len();
                    while count > 0 {
                        let step: usize = count / 2;
                        if edges[first + step].0 < i {
                            first += step + 1;
                            count -= step + 1;
                        } else {
                            count = step;
                        }
                    }
                    first
                };
                while edge_i < edges.len() {
                    let edge = edges[edge_i];
                    if edge.0 != i {
                        break;
                    }
                    if visit(edges, status, edge.1) {
                        return true;
                    }
                    edge_i += 1;
                }
                status[i] = VisitStatus::Visited;
                false
            }
            VisitStatus::Visiting => true,
            VisitStatus::Visited => false,
        }
    }
    for i in 0..vertex_count {
        if visit(edges, &mut status, i) {
            return true;
        }
    }
    false
}

/// Given a vector and ordered list of indices for that vector, checks whether all elements in the
/// given indices are sorted in ascending order (equality accepted).
///
/// Panics if an invalid index is given.
pub fn are_indices_sorted<T: Ord>(v: &[T], indices: &Vec<usize>) -> bool {
    if indices.len() <= 1 {
        return true;
    }
    let mut last = &v[indices[0]];
    for &index in indices.iter().skip(1) {
        let current = &v[index];
        if current < last {
            return false;
        }
        last = current;
    }
    true
}

/// Returns the indices of elements that are repeating in a continuous sequence.
/// For example, `1,1,1` is considered but `1,2,1` is ignored.
pub fn get_repeating_indices<T: PartialEq>(v: &[T]) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    if v.len() <= 1 {
        return out;
    }
    out.reserve_exact(v.len());
    let mut last_added = false;
    let mut last = &v[0];
    for (i, current) in v.iter().enumerate().skip(1) {
        if current == last {
            if !last_added {
                out.push(i - 1);
            }
            out.push(i);
            last_added = true;
        } else {
            last_added = false;
        }
        last = current;
    }
    out
}

/// For a distance matrix, return the average value, excluding diagonal entries.
pub fn distance_matrix_average<T>(matrix: &Array2<T>) -> f64
where
    T: Clone + num_traits::identities::Zero + num_traits::cast::AsPrimitive<f64>,
{
    let size = matrix.shape()[0];
    debug_assert_eq!(size, matrix.shape()[1]);
    let sum = matrix.sum();
    let n_nondiag_elements = size * (size - 1);
    sum.as_() / n_nondiag_elements as f64
}

/// Get the distances between neighbors in graph.
pub fn neighbor_distances<T, U: Unsigned + ToPrimitive>(
    matrix: &Array2<T>,
    adj: &[Vec<U>],
) -> Vec<T>
where
    T: Copy,
{
    let mut out = Vec::new();
    for (i, adj) in adj.iter().enumerate() {
        for j in adj.iter() {
            let j: usize = (*j).to_usize().unwrap();
            out.push(matrix[(i, j)])
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::bool_assert_comparison)]
mod tests {
    use super::*;

    #[test]
    fn sorted_intersects_test() {
        assert_eq!(
            sorted_intersects(vec![1, 2, 3].iter(), vec![].iter()),
            false
        );
        assert_eq!(
            sorted_intersects(vec![1, 2, 3].iter(), vec![3].iter()),
            true
        );
        assert_eq!(
            sorted_intersects(vec![3].iter(), vec![1, 2, 3].iter()),
            true
        );
        assert_eq!(
            sorted_intersects(vec![666].iter(), vec![1, 2, 3].iter()),
            false
        );
        assert_eq!(
            sorted_intersects(vec![2, 3].iter(), vec![2, 3].iter()),
            true
        );
        assert_eq!(
            sorted_intersects(vec![1, 2, 3, 15].iter(), vec![11, 12, 13, 15].iter()),
            true
        );
        assert_eq!(
            sorted_intersects(Vec::<i32>::new().iter(), vec![].iter()),
            false
        );
    }

    #[test]
    fn sorted_intersection_test() {
        assert_eq!(
            sorted_intersection(&vec![1, 2, 3], &vec![]),
            Vec::<i32>::new()
        );
        assert_eq!(sorted_intersection(&vec![1, 2, 3], &vec![3]), vec![3]);
        assert_eq!(sorted_intersection(&vec![3], &vec![1, 2, 3]), vec![3]);
        assert_eq!(
            sorted_intersection(&vec![666], &vec![1, 2, 3]),
            Vec::<i32>::new()
        );
        assert_eq!(sorted_intersection(&vec![2, 3], &vec![2, 3]), vec![2, 3]);
        assert_eq!(
            sorted_intersection(&vec![1, 2, 3, 15], &vec![11, 12, 13, 15]),
            vec![15]
        );
        assert_eq!(
            sorted_intersection(&Vec::<i32>::new(), &vec![]),
            Vec::<i32>::new()
        );
    }

    #[test]
    fn is_graph_cyclic_test() {
        assert_eq!(is_graph_cyclic(2, &vec![(0, 1), (1, 0)]), true);
        assert_eq!(is_graph_cyclic(3, &vec![(0, 1), (1, 0)]), true);
        assert_eq!(is_graph_cyclic(3, &vec![(0, 1), (1, 2)]), false);
        assert_eq!(is_graph_cyclic(3, &vec![(0, 1), (1, 2), (2, 1)]), true);
        assert_eq!(is_graph_cyclic(3, &vec![(0, 1), (1, 2), (2, 2)]), true);
        assert_eq!(is_graph_cyclic(4, &vec![(0, 1), (1, 2), (2, 3)]), false);
        assert_eq!(is_graph_cyclic(3, &vec![(0, 0)]), true);
        assert_eq!(is_graph_cyclic(3, &vec![]), false);
    }

    #[test]
    fn are_indices_sorted_test() {
        assert_eq!(are_indices_sorted(&[0], &vec![0]), true,);
        assert_eq!(are_indices_sorted(&[900, 1, 2, 0, 3], &vec![1, 2, 4]), true,);
        assert_eq!(are_indices_sorted(&[900, 1, 2, 0, 3], &vec![0, 1]), false,);
        assert_eq!(
            are_indices_sorted(&[900, 1, 2, 0, 3], &vec![1, 2, 3]),
            false,
        );
    }

    #[test]
    fn get_repeating_indices_test() {
        assert_eq!(
            get_repeating_indices(&Vec::<usize>::new()),
            Vec::<usize>::new(),
        );
        assert_eq!(get_repeating_indices(&[1]), Vec::<usize>::new(),);
        assert_eq!(
            get_repeating_indices(&[1, 2, 3, 4, 5, 4]),
            Vec::<usize>::new(),
        );
        assert_eq!(get_repeating_indices(&[1, 2, 3, 3, 3, 4]), vec![2, 3, 4],);
        assert_eq!(get_repeating_indices(&[0, 0, 0, 0]), vec![0, 1, 2, 3],);
        assert_eq!(
            get_repeating_indices(&[1, 2, 3, 3, 3, 4, 1, 1, 1, 1]),
            vec![2, 3, 4, 6, 7, 8, 9],
        );
    }

    #[test]
    fn test_distance_matrix_average() {
        let a: Array2<usize> = ndarray::arr2(&[[0, 2], [1, 0]]);
        assert_eq!(distance_matrix_average(&a), 1.5);
        let a: Array2<f64> = ndarray::arr2(&[[0.0, 3.5], [1.5, 0.0]]);
        assert_eq!(distance_matrix_average(&a), 2.5);
    }
}
