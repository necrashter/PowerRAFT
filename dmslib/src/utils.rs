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

#[cfg(test)]
mod tests {
    use super::sorted_intersects;

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
}
