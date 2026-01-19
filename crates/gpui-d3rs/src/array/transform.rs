//! Data transformation functions
//!
//! Provides functions for grouping, sorting, and transforming arrays.

use std::collections::HashMap;
use std::hash::Hash;

/// Groups elements by a key function.
///
/// Returns a HashMap where each key maps to a Vec of elements with that key.
///
/// # Example
///
/// ```
/// use d3rs::array::group;
///
/// let data = vec!["apple", "apricot", "banana", "blueberry", "cherry"];
/// let grouped = group(&data, |s| s.chars().next().unwrap());
///
/// assert_eq!(grouped[&'a'].len(), 2);
/// assert_eq!(grouped[&'b'].len(), 2);
/// assert_eq!(grouped[&'c'].len(), 1);
/// ```
pub fn group<'a, T, K, F>(data: &'a [T], key: F) -> HashMap<K, Vec<&'a T>>
where
    K: Eq + Hash,
    F: Fn(&T) -> K,
{
    let mut result: HashMap<K, Vec<&'a T>> = HashMap::new();
    for item in data {
        let k = key(item);
        result.entry(k).or_default().push(item);
    }
    result
}

/// Groups elements and applies a reducer to each group.
///
/// # Example
///
/// ```
/// use d3rs::array::rollup;
///
/// let data = vec![
///     ("A", 10),
///     ("B", 20),
///     ("A", 30),
///     ("B", 40),
/// ];
///
/// let sums = rollup(&data, |item| item.0, |group| {
///     group.iter().map(|item| item.1).sum::<i32>()
/// });
///
/// assert_eq!(sums[&"A"], 40);
/// assert_eq!(sums[&"B"], 60);
/// ```
pub fn rollup<T, K, V, F, R>(data: &[T], key: F, reduce: R) -> HashMap<K, V>
where
    K: Eq + Hash,
    F: Fn(&T) -> K,
    R: Fn(&[&T]) -> V,
{
    let groups = group(data, key);
    groups.into_iter().map(|(k, v)| (k, reduce(&v))).collect()
}

/// Creates an index from key to element.
///
/// If multiple elements have the same key, the last one wins.
///
/// # Example
///
/// ```
/// use d3rs::array::index;
///
/// let data = vec![
///     ("id1", "Alice"),
///     ("id2", "Bob"),
///     ("id3", "Charlie"),
/// ];
///
/// let indexed = index(&data, |item| item.0);
/// assert_eq!(indexed[&"id2"].1, "Bob");
/// ```
pub fn index<'a, T, K, F>(data: &'a [T], key: F) -> HashMap<K, &'a T>
where
    K: Eq + Hash,
    F: Fn(&T) -> K,
{
    let mut result: HashMap<K, &'a T> = HashMap::new();
    for item in data {
        let k = key(item);
        result.insert(k, item);
    }
    result
}

/// Sorts a slice in place using a key accessor.
///
/// # Example
///
/// ```
/// use d3rs::array::sort_by;
///
/// let mut data = vec![("c", 3), ("a", 1), ("b", 2)];
/// sort_by(&mut data, |item| item.1);
/// assert_eq!(data, vec![("a", 1), ("b", 2), ("c", 3)]);
/// ```
pub fn sort_by<T, K, F>(data: &mut [T], key: F)
where
    K: Ord,
    F: Fn(&T) -> K,
{
    data.sort_by_key(|a| key(a));
}

/// Sorts a slice in place in descending order using a key accessor.
///
/// # Example
///
/// ```
/// use d3rs::array::sort_by_desc;
///
/// let mut data = vec![("c", 3), ("a", 1), ("b", 2)];
/// sort_by_desc(&mut data, |item| item.1);
/// assert_eq!(data, vec![("c", 3), ("b", 2), ("a", 1)]);
/// ```
pub fn sort_by_desc<T, K, F>(data: &mut [T], key: F)
where
    K: Ord,
    F: Fn(&T) -> K + Copy,
{
    data.sort_by_key(|item| std::cmp::Reverse(key(item)));
}

/// Randomly shuffles a slice using the Fisher-Yates algorithm.
///
/// # Example
///
/// ```
/// use d3rs::array::shuffle;
///
/// let mut data = vec![1, 2, 3, 4, 5];
/// shuffle(&mut data);
/// // data is now shuffled
/// ```
pub fn shuffle<T>(data: &mut [T]) {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Simple LCG random number generator
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    for i in (1..data.len()).rev() {
        // Generate random index in [0, i]
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = ((seed >> 33) as usize) % (i + 1);
        data.swap(i, j);
    }
}

/// Randomly shuffles a slice using a seeded random number generator.
///
/// # Example
///
/// ```
/// use d3rs::array::shuffle_seeded;
///
/// let mut data1 = vec![1, 2, 3, 4, 5];
/// let mut data2 = vec![1, 2, 3, 4, 5];
///
/// shuffle_seeded(&mut data1, 12345);
/// shuffle_seeded(&mut data2, 12345);
///
/// assert_eq!(data1, data2); // Same seed produces same shuffle
/// ```
pub fn shuffle_seeded<T>(data: &mut [T], seed: u64) {
    let mut rng_seed = seed;

    for i in (1..data.len()).rev() {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let j = ((rng_seed >> 33) as usize) % (i + 1);
        data.swap(i, j);
    }
}

/// Reverses the order of elements in a slice.
///
/// # Example
///
/// ```
/// use d3rs::array::reverse;
///
/// let mut data = vec![1, 2, 3, 4, 5];
/// reverse(&mut data);
/// assert_eq!(data, vec![5, 4, 3, 2, 1]);
/// ```
pub fn reverse<T>(data: &mut [T]) {
    data.reverse();
}

/// Returns a new Vec containing only unique elements (first occurrence).
///
/// # Example
///
/// ```
/// use d3rs::array::unique;
///
/// let data = vec![1, 2, 2, 3, 1, 4, 3];
/// assert_eq!(unique(&data), vec![1, 2, 3, 4]);
/// ```
pub fn unique<T: Clone + Eq + Hash>(data: &[T]) -> Vec<T> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for item in data {
        if seen.insert(item.clone()) {
            result.push(item.clone());
        }
    }
    result
}

/// Returns pairs of consecutive elements.
///
/// # Example
///
/// ```
/// use d3rs::array::pairs;
///
/// let data = vec![1, 2, 3, 4, 5];
/// let result = pairs(&data);
/// assert_eq!(result, vec![(&1, &2), (&2, &3), (&3, &4), (&4, &5)]);
/// ```
pub fn pairs<T>(data: &[T]) -> Vec<(&T, &T)> {
    data.windows(2).map(|w| (&w[0], &w[1])).collect()
}

/// Returns cross product of two slices.
///
/// # Example
///
/// ```
/// use d3rs::array::cross;
///
/// let a = vec![1, 2];
/// let b = vec!["a", "b"];
/// let result = cross(&a, &b);
/// assert_eq!(result, vec![(&1, &"a"), (&1, &"b"), (&2, &"a"), (&2, &"b")]);
/// ```
pub fn cross<'a, 'b, T, U>(a: &'a [T], b: &'b [U]) -> Vec<(&'a T, &'b U)> {
    let mut result = Vec::with_capacity(a.len() * b.len());
    for item_a in a {
        for item_b in b {
            result.push((item_a, item_b));
        }
    }
    result
}

/// Merges multiple sorted slices into a single sorted Vec.
///
/// # Example
///
/// ```
/// use d3rs::array::merge_sorted;
///
/// let a = vec![1, 3, 5];
/// let b = vec![2, 4, 6];
/// assert_eq!(merge_sorted(&[&a[..], &b[..]]), vec![1, 2, 3, 4, 5, 6]);
/// ```
pub fn merge_sorted<T: Ord + Clone>(slices: &[&[T]]) -> Vec<T> {
    let total_len: usize = slices.iter().map(|s| s.len()).sum();
    let mut result = Vec::with_capacity(total_len);

    // Simple k-way merge using indices
    let mut indices: Vec<usize> = vec![0; slices.len()];

    loop {
        // Find the minimum element among all non-exhausted slices
        let mut min_val: Option<&T> = None;
        let mut min_idx = 0;

        for (i, slice) in slices.iter().enumerate() {
            if indices[i] < slice.len() {
                let val = &slice[indices[i]];
                if min_val.is_none() || val < min_val.unwrap() {
                    min_val = Some(val);
                    min_idx = i;
                }
            }
        }

        match min_val {
            Some(val) => {
                result.push(val.clone());
                indices[min_idx] += 1;
            }
            None => break,
        }
    }

    result
}

/// Filters elements by a predicate.
///
/// # Example
///
/// ```
/// use d3rs::array::filter;
///
/// let data = vec![1, 2, 3, 4, 5, 6];
/// let evens = filter(&data, |x| x % 2 == 0);
/// assert_eq!(evens, vec![&2, &4, &6]);
/// ```
pub fn filter<T, F>(data: &[T], predicate: F) -> Vec<&T>
where
    F: Fn(&T) -> bool,
{
    data.iter().filter(|x| predicate(x)).collect()
}

/// Maps elements using a function.
///
/// # Example
///
/// ```
/// use d3rs::array::map;
///
/// let data = vec![1, 2, 3, 4, 5];
/// let squares = map(&data, |x| x * x);
/// assert_eq!(squares, vec![1, 4, 9, 16, 25]);
/// ```
pub fn map<T, U, F>(data: &[T], f: F) -> Vec<U>
where
    F: Fn(&T) -> U,
{
    data.iter().map(f).collect()
}

/// Reduces elements to a single value.
///
/// # Example
///
/// ```
/// use d3rs::array::reduce;
///
/// let data = vec![1, 2, 3, 4, 5];
/// let sum = reduce(&data, 0, |acc, x| acc + x);
/// assert_eq!(sum, 15);
/// ```
pub fn reduce<T, U, F>(data: &[T], initial: U, f: F) -> U
where
    F: Fn(U, &T) -> U,
{
    data.iter().fold(initial, f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group() {
        let data = vec!["apple", "apricot", "banana", "blueberry", "cherry"];
        let grouped = group(&data, |s| s.chars().next().unwrap());

        assert_eq!(grouped[&'a'].len(), 2);
        assert_eq!(grouped[&'b'].len(), 2);
        assert_eq!(grouped[&'c'].len(), 1);
    }

    #[test]
    fn test_rollup() {
        let data = vec![("A", 10), ("B", 20), ("A", 30), ("B", 40)];

        let sums = rollup(
            &data,
            |item| item.0,
            |group| group.iter().map(|item| item.1).sum::<i32>(),
        );

        assert_eq!(sums[&"A"], 40);
        assert_eq!(sums[&"B"], 60);
    }

    #[test]
    fn test_sort_by() {
        let mut data = vec![("c", 3), ("a", 1), ("b", 2)];
        sort_by(&mut data, |item| item.1);
        assert_eq!(data, vec![("a", 1), ("b", 2), ("c", 3)]);
    }

    #[test]
    fn test_shuffle_seeded() {
        let mut data1 = vec![1, 2, 3, 4, 5];
        let mut data2 = vec![1, 2, 3, 4, 5];

        shuffle_seeded(&mut data1, 12345);
        shuffle_seeded(&mut data2, 12345);

        assert_eq!(data1, data2);
    }

    #[test]
    fn test_unique() {
        let data = vec![1, 2, 2, 3, 1, 4, 3];
        assert_eq!(unique(&data), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_pairs() {
        let data = vec![1, 2, 3, 4, 5];
        let result = pairs(&data);
        assert_eq!(result, vec![(&1, &2), (&2, &3), (&3, &4), (&4, &5)]);
    }

    #[test]
    fn test_cross() {
        let a = vec![1, 2];
        let b = vec!["a", "b"];
        let result = cross(&a, &b);
        assert_eq!(result, vec![(&1, &"a"), (&1, &"b"), (&2, &"a"), (&2, &"b")]);
    }

    #[test]
    fn test_merge_sorted() {
        let a = vec![1, 3, 5];
        let b = vec![2, 4, 6];
        assert_eq!(merge_sorted(&[&a[..], &b[..]]), vec![1, 2, 3, 4, 5, 6]);
    }
}
