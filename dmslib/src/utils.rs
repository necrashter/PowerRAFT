//! Various utility functions.

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

#[cfg(test)]
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
}
