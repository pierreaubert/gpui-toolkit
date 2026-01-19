//! Set operations for arrays
//!
//! Provides functions for computing set differences, intersections, and unions.

use std::collections::HashSet;
use std::hash::Hash;

/// Returns elements that are in `a` but not in `b`.
///
/// # Example
///
/// ```
/// use d3rs::array::difference;
///
/// let a = vec![1, 2, 3, 4, 5];
/// let b = vec![3, 4, 5, 6, 7];
/// assert_eq!(difference(&a, &b), vec![1, 2]);
/// ```
pub fn difference<T: Clone + Eq + Hash>(a: &[T], b: &[T]) -> Vec<T> {
    let b_set: HashSet<&T> = b.iter().collect();
    a.iter().filter(|x| !b_set.contains(x)).cloned().collect()
}

/// Returns elements that are in both `a` and `b`.
///
/// # Example
///
/// ```
/// use d3rs::array::intersection;
///
/// let a = vec![1, 2, 3, 4, 5];
/// let b = vec![3, 4, 5, 6, 7];
/// assert_eq!(intersection(&a, &b), vec![3, 4, 5]);
/// ```
pub fn intersection<T: Clone + Eq + Hash>(a: &[T], b: &[T]) -> Vec<T> {
    let b_set: HashSet<&T> = b.iter().collect();
    a.iter().filter(|x| b_set.contains(x)).cloned().collect()
}

/// Returns elements that are in either `a` or `b` (or both).
///
/// The order is: all elements from `a`, then elements from `b` not in `a`.
///
/// # Example
///
/// ```
/// use d3rs::array::union;
///
/// let a = vec![1, 2, 3];
/// let b = vec![3, 4, 5];
/// assert_eq!(union(&a, &b), vec![1, 2, 3, 4, 5]);
/// ```
pub fn union<T: Clone + Eq + Hash>(a: &[T], b: &[T]) -> Vec<T> {
    let mut seen: HashSet<T> = HashSet::new();
    let mut result: Vec<T> = Vec::new();

    for item in a {
        if seen.insert(item.clone()) {
            result.push(item.clone());
        }
    }

    for item in b {
        if seen.insert(item.clone()) {
            result.push(item.clone());
        }
    }

    result
}

/// Returns the symmetric difference: elements in `a` or `b` but not both.
///
/// # Example
///
/// ```
/// use d3rs::array::symmetric_difference;
///
/// let a = vec![1, 2, 3, 4];
/// let b = vec![3, 4, 5, 6];
/// let result = symmetric_difference(&a, &b);
/// assert!(result.contains(&1));
/// assert!(result.contains(&2));
/// assert!(result.contains(&5));
/// assert!(result.contains(&6));
/// assert!(!result.contains(&3));
/// assert!(!result.contains(&4));
/// ```
pub fn symmetric_difference<T: Clone + Eq + Hash>(a: &[T], b: &[T]) -> Vec<T> {
    let a_set: HashSet<&T> = a.iter().collect();
    let b_set: HashSet<&T> = b.iter().collect();

    let mut result: Vec<T> = Vec::new();

    // Elements in a but not b
    for item in a {
        if !b_set.contains(item) {
            result.push(item.clone());
        }
    }

    // Elements in b but not a
    for item in b {
        if !a_set.contains(item) {
            result.push(item.clone());
        }
    }

    result
}

/// Returns true if `a` is a subset of `b` (all elements of `a` are in `b`).
///
/// # Example
///
/// ```
/// use d3rs::array::is_subset;
///
/// let a = vec![1, 2, 3];
/// let b = vec![1, 2, 3, 4, 5];
/// assert!(is_subset(&a, &b));
/// assert!(!is_subset(&b, &a));
/// ```
pub fn is_subset<T: Eq + Hash>(a: &[T], b: &[T]) -> bool {
    let b_set: HashSet<&T> = b.iter().collect();
    a.iter().all(|x| b_set.contains(x))
}

/// Returns true if `a` is a superset of `b` (all elements of `b` are in `a`).
///
/// # Example
///
/// ```
/// use d3rs::array::is_superset;
///
/// let a = vec![1, 2, 3, 4, 5];
/// let b = vec![1, 2, 3];
/// assert!(is_superset(&a, &b));
/// ```
pub fn is_superset<T: Eq + Hash>(a: &[T], b: &[T]) -> bool {
    is_subset(b, a)
}

/// Returns true if `a` and `b` have no elements in common.
///
/// # Example
///
/// ```
/// use d3rs::array::is_disjoint;
///
/// let a = vec![1, 2, 3];
/// let b = vec![4, 5, 6];
/// let c = vec![3, 4, 5];
/// assert!(is_disjoint(&a, &b));
/// assert!(!is_disjoint(&a, &c));
/// ```
pub fn is_disjoint<T: Eq + Hash>(a: &[T], b: &[T]) -> bool {
    let b_set: HashSet<&T> = b.iter().collect();
    !a.iter().any(|x| b_set.contains(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difference() {
        let a = vec![1, 2, 3, 4, 5];
        let b = vec![3, 4, 5, 6, 7];
        assert_eq!(difference(&a, &b), vec![1, 2]);
    }

    #[test]
    fn test_intersection() {
        let a = vec![1, 2, 3, 4, 5];
        let b = vec![3, 4, 5, 6, 7];
        assert_eq!(intersection(&a, &b), vec![3, 4, 5]);
    }

    #[test]
    fn test_union() {
        let a = vec![1, 2, 3];
        let b = vec![3, 4, 5];
        assert_eq!(union(&a, &b), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_symmetric_difference() {
        let a = vec![1, 2, 3, 4];
        let b = vec![3, 4, 5, 6];
        let result = symmetric_difference(&a, &b);
        assert_eq!(result.len(), 4);
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        assert!(result.contains(&5));
        assert!(result.contains(&6));
    }

    #[test]
    fn test_subset_superset() {
        let a = vec![1, 2, 3];
        let b = vec![1, 2, 3, 4, 5];
        assert!(is_subset(&a, &b));
        assert!(!is_subset(&b, &a));
        assert!(is_superset(&b, &a));
    }

    #[test]
    fn test_disjoint() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5, 6];
        let c = vec![3, 4, 5];
        assert!(is_disjoint(&a, &b));
        assert!(!is_disjoint(&a, &c));
    }
}
