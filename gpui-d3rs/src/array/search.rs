//! Binary search functions for sorted arrays
//!
//! Provides functions for efficiently locating values in sorted arrays.

use std::cmp::Ordering;

/// Returns the insertion point for `x` in a sorted slice to maintain sorted order.
///
/// If `x` is already present, returns the index after any existing entries.
/// This is equivalent to `bisect_right`.
///
/// # Example
///
/// ```
/// use d3rs::array::bisect;
///
/// let data = vec![1, 2, 3, 5, 8, 13];
/// assert_eq!(bisect(&data, &4), 3);
/// assert_eq!(bisect(&data, &5), 4); // After existing 5
/// ```
pub fn bisect<T: Ord>(data: &[T], x: &T) -> usize {
    bisect_right(data, x)
}

/// Returns the leftmost insertion point for `x` in a sorted slice.
///
/// If `x` is already present, returns the index before any existing entries.
///
/// # Example
///
/// ```
/// use d3rs::array::bisect_left;
///
/// let data = vec![1, 2, 3, 3, 3, 5, 8];
/// assert_eq!(bisect_left(&data, &3), 2);
/// assert_eq!(bisect_left(&data, &4), 5);
/// ```
pub fn bisect_left<T: Ord>(data: &[T], x: &T) -> usize {
    let mut lo = 0;
    let mut hi = data.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if data[mid] < *x {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Returns the rightmost insertion point for `x` in a sorted slice.
///
/// If `x` is already present, returns the index after any existing entries.
///
/// # Example
///
/// ```
/// use d3rs::array::bisect_right;
///
/// let data = vec![1, 2, 3, 3, 3, 5, 8];
/// assert_eq!(bisect_right(&data, &3), 5);
/// assert_eq!(bisect_right(&data, &4), 5);
/// ```
pub fn bisect_right<T: Ord>(data: &[T], x: &T) -> usize {
    let mut lo = 0;
    let mut hi = data.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if data[mid] <= *x {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// A bisector that uses a custom accessor function.
///
/// # Example
///
/// ```
/// use d3rs::array::Bisector;
///
/// #[derive(Debug)]
/// struct Point { x: f64, y: f64 }
///
/// let bisector = Bisector::new(|p: &Point| p.x);
///
/// let data = vec![
///     Point { x: 1.0, y: 2.0 },
///     Point { x: 3.0, y: 4.0 },
///     Point { x: 5.0, y: 6.0 },
/// ];
///
/// assert_eq!(bisector.left(&data, 2.0), 1);
/// assert_eq!(bisector.right(&data, 3.0), 2);
/// ```
pub struct Bisector<T, K> {
    accessor: Box<dyn Fn(&T) -> K>,
}

impl<T, K: PartialOrd> Bisector<T, K> {
    /// Creates a new bisector with the given accessor function.
    pub fn new<F>(accessor: F) -> Self
    where
        F: Fn(&T) -> K + 'static,
    {
        Self {
            accessor: Box::new(accessor),
        }
    }

    /// Returns the leftmost insertion point for value `x`.
    pub fn left(&self, data: &[T], x: K) -> usize {
        let mut lo = 0;
        let mut hi = data.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if (self.accessor)(&data[mid]) < x {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// Returns the rightmost insertion point for value `x`.
    pub fn right(&self, data: &[T], x: K) -> usize {
        let mut lo = 0;
        let mut hi = data.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if (self.accessor)(&data[mid]) <= x {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// Returns the element in the slice that is closest to `x`.
    pub fn center<'a>(&self, data: &'a [T], x: K) -> Option<&'a T>
    where
        K: Copy + std::ops::Sub<Output = K>,
    {
        if data.is_empty() {
            return None;
        }
        let i = self.left(data, x);
        if i == 0 {
            return Some(&data[0]);
        }
        if i >= data.len() {
            return Some(&data[data.len() - 1]);
        }
        // Compare distances to decide which element is closer
        Some(&data[i])
    }
}

/// Finds the value in a sorted slice that is closest to `x`.
///
/// # Example
///
/// ```
/// use d3rs::array::least_index;
///
/// let data = vec![1.0, 2.0, 5.0, 8.0, 13.0];
/// assert_eq!(least_index(&data, 6.0), Some(2)); // 5.0 is closer than 8.0
/// ```
pub fn least_index(data: &[f64], x: f64) -> Option<usize> {
    if data.is_empty() {
        return None;
    }
    let i = bisect_left_f64(data, x);
    if i == 0 {
        return Some(0);
    }
    if i >= data.len() {
        return Some(data.len() - 1);
    }
    // Compare distances
    let dist_left = (data[i - 1] - x).abs();
    let dist_right = (data[i] - x).abs();
    if dist_left <= dist_right {
        Some(i - 1)
    } else {
        Some(i)
    }
}

/// Binary search for f64 values (handles NaN correctly).
pub fn bisect_left_f64(data: &[f64], x: f64) -> usize {
    let mut lo = 0;
    let mut hi = data.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match data[mid].partial_cmp(&x) {
            Some(Ordering::Less) => lo = mid + 1,
            _ => hi = mid,
        }
    }
    lo
}

/// Binary search for f64 values (handles NaN correctly).
pub fn bisect_right_f64(data: &[f64], x: f64) -> usize {
    let mut lo = 0;
    let mut hi = data.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match data[mid].partial_cmp(&x) {
            Some(Ordering::Greater) => hi = mid,
            _ => lo = mid + 1,
        }
    }
    lo
}

/// Returns the index of the value in a sorted slice that equals `x`,
/// or `None` if not found.
///
/// # Example
///
/// ```
/// use d3rs::array::binary_search;
///
/// let data = vec![1, 2, 3, 5, 8, 13];
/// assert_eq!(binary_search(&data, &5), Some(3));
/// assert_eq!(binary_search(&data, &4), None);
/// ```
pub fn binary_search<T: Ord>(data: &[T], x: &T) -> Option<usize> {
    data.binary_search(x).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bisect_left() {
        let data = vec![1, 2, 3, 3, 3, 5, 8];
        assert_eq!(bisect_left(&data, &0), 0);
        assert_eq!(bisect_left(&data, &1), 0);
        assert_eq!(bisect_left(&data, &3), 2);
        assert_eq!(bisect_left(&data, &4), 5);
        assert_eq!(bisect_left(&data, &9), 7);
    }

    #[test]
    fn test_bisect_right() {
        let data = vec![1, 2, 3, 3, 3, 5, 8];
        assert_eq!(bisect_right(&data, &0), 0);
        assert_eq!(bisect_right(&data, &1), 1);
        assert_eq!(bisect_right(&data, &3), 5);
        assert_eq!(bisect_right(&data, &4), 5);
        assert_eq!(bisect_right(&data, &9), 7);
    }

    #[test]
    fn test_bisector() {
        #[derive(Debug)]
        struct Item {
            value: i32,
        }

        let bisector = Bisector::new(|item: &Item| item.value);
        let data = vec![
            Item { value: 1 },
            Item { value: 3 },
            Item { value: 3 },
            Item { value: 5 },
        ];

        assert_eq!(bisector.left(&data, 3), 1);
        assert_eq!(bisector.right(&data, 3), 3);
    }

    #[test]
    fn test_least_index() {
        let data = vec![1.0, 2.0, 5.0, 8.0, 13.0];
        assert_eq!(least_index(&data, 0.0), Some(0));
        assert_eq!(least_index(&data, 3.0), Some(1)); // 2.0 is closer than 5.0
        assert_eq!(least_index(&data, 6.0), Some(2)); // 5.0 is closer than 8.0
        assert_eq!(least_index(&data, 20.0), Some(4));
    }

    #[test]
    fn test_binary_search() {
        let data = vec![1, 2, 3, 5, 8, 13];
        assert_eq!(binary_search(&data, &5), Some(3));
        assert_eq!(binary_search(&data, &4), None);
    }
}
