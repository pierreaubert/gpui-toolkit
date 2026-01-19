//! Statistical functions for data arrays
//!
//! Provides functions for computing summary statistics like min, max, mean,
//! median, variance, and quantiles.

use std::cmp::Ordering;
use std::iter::Sum;

/// Returns the minimum value in the slice.
///
/// Returns `None` if the slice is empty.
///
/// # Example
///
/// ```
/// use d3rs::array::min;
///
/// let data = vec![3, 1, 4, 1, 5, 9];
/// assert_eq!(min(&data), Some(&1));
/// assert_eq!(min::<i32>(&[]), None);
/// ```
pub fn min<T: Ord>(data: &[T]) -> Option<&T> {
    data.iter().min()
}

/// Returns the minimum value in the slice using a custom comparator.
///
/// # Example
///
/// ```
/// use d3rs::array::min_by;
///
/// let data = vec![3.0, 1.0, f64::NAN, 4.0];
/// let result = min_by(&data, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
/// assert_eq!(result, Some(&1.0));
/// ```
pub fn min_by<T, F>(data: &[T], compare: F) -> Option<&T>
where
    F: Fn(&T, &T) -> Ordering,
{
    data.iter().min_by(|a, b| compare(a, b))
}

/// Returns the maximum value in the slice.
///
/// Returns `None` if the slice is empty.
///
/// # Example
///
/// ```
/// use d3rs::array::max;
///
/// let data = vec![3, 1, 4, 1, 5, 9];
/// assert_eq!(max(&data), Some(&9));
/// ```
pub fn max<T: Ord>(data: &[T]) -> Option<&T> {
    data.iter().max()
}

/// Returns the maximum value in the slice using a custom comparator.
pub fn max_by<T, F>(data: &[T], compare: F) -> Option<&T>
where
    F: Fn(&T, &T) -> Ordering,
{
    data.iter().max_by(|a, b| compare(a, b))
}

/// Returns the minimum and maximum values in the slice.
///
/// Returns `None` if the slice is empty.
///
/// # Example
///
/// ```
/// use d3rs::array::extent;
///
/// let data = vec![3, 1, 4, 1, 5, 9];
/// assert_eq!(extent(&data), Some((&1, &9)));
/// ```
pub fn extent<T: Ord>(data: &[T]) -> Option<(&T, &T)> {
    if data.is_empty() {
        return None;
    }
    let min = data.iter().min()?;
    let max = data.iter().max()?;
    Some((min, max))
}

/// Returns the minimum and maximum values using a custom comparator.
pub fn extent_by<T, F>(data: &[T], compare: F) -> Option<(&T, &T)>
where
    F: Fn(&T, &T) -> Ordering + Copy,
{
    if data.is_empty() {
        return None;
    }
    let min = data.iter().min_by(|a, b| compare(a, b))?;
    let max = data.iter().max_by(|a, b| compare(a, b))?;
    Some((min, max))
}

/// Returns the sum of values in the slice.
///
/// # Example
///
/// ```
/// use d3rs::array::sum;
///
/// let data = vec![1, 2, 3, 4, 5];
/// assert_eq!(sum(&data), 15);
/// ```
pub fn sum<T>(data: &[T]) -> T
where
    T: Sum + Clone,
{
    data.iter().cloned().sum()
}

/// Returns the arithmetic mean of values in the slice.
///
/// Returns `None` if the slice is empty.
///
/// # Example
///
/// ```
/// use d3rs::array::mean;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// assert_eq!(mean(&data), Some(3.0));
/// ```
pub fn mean(data: &[f64]) -> Option<f64> {
    if data.is_empty() {
        return None;
    }
    Some(data.iter().sum::<f64>() / data.len() as f64)
}

/// Returns the arithmetic mean using an accessor function.
///
/// # Example
///
/// ```
/// use d3rs::array::mean_by;
///
/// #[derive(Debug)]
/// struct Point { x: f64, y: f64 }
///
/// let data = vec![
///     Point { x: 1.0, y: 2.0 },
///     Point { x: 3.0, y: 4.0 },
/// ];
/// assert_eq!(mean_by(&data, |p| p.x), Some(2.0));
/// ```
pub fn mean_by<T, F>(data: &[T], accessor: F) -> Option<f64>
where
    F: Fn(&T) -> f64,
{
    if data.is_empty() {
        return None;
    }
    Some(data.iter().map(&accessor).sum::<f64>() / data.len() as f64)
}

/// Returns the median of values in a mutable slice.
///
/// The slice will be partially reordered.
/// Returns `None` if the slice is empty.
///
/// # Example
///
/// ```
/// use d3rs::array::median;
///
/// let mut data = vec![3.0, 1.0, 4.0, 1.0, 5.0];
/// assert_eq!(median(&mut data), Some(3.0));
/// ```
pub fn median(data: &mut [f64]) -> Option<f64> {
    if data.is_empty() {
        return None;
    }
    // Filter out NaN values
    let mut valid: Vec<f64> = data.iter().copied().filter(|x| !x.is_nan()).collect();
    if valid.is_empty() {
        return None;
    }
    valid.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = valid.len();
    if n.is_multiple_of(2) {
        Some((valid[n / 2 - 1] + valid[n / 2]) / 2.0)
    } else {
        Some(valid[n / 2])
    }
}

/// Returns the p-quantile of values in a sorted slice.
///
/// The slice must be sorted. The parameter `p` should be in `[0, 1]`.
///
/// # Example
///
/// ```
/// use d3rs::array::quantile_sorted;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// assert_eq!(quantile_sorted(&data, 0.5), Some(3.0));
/// assert_eq!(quantile_sorted(&data, 0.25), Some(2.0));
/// ```
pub fn quantile_sorted(data: &[f64], p: f64) -> Option<f64> {
    if data.is_empty() || !(0.0..=1.0).contains(&p) {
        return None;
    }
    if data.len() == 1 {
        return Some(data[0]);
    }
    let n = data.len();
    let i = (n - 1) as f64 * p;
    let i0 = i.floor() as usize;
    let i1 = i.ceil() as usize;
    let v0 = data[i0];
    let v1 = data[i1];
    Some(v0 + (v1 - v0) * (i - i0 as f64))
}

/// Returns the p-quantile of values in a mutable slice.
///
/// The slice will be partially reordered.
///
/// # Example
///
/// ```
/// use d3rs::array::quantile;
///
/// let mut data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
/// assert_eq!(quantile(&mut data, 0.5), Some(3.5));
/// ```
pub fn quantile(data: &mut [f64], p: f64) -> Option<f64> {
    if data.is_empty() || !(0.0..=1.0).contains(&p) {
        return None;
    }
    // Filter out NaN values
    let mut valid: Vec<f64> = data.iter().copied().filter(|x| !x.is_nan()).collect();
    if valid.is_empty() {
        return None;
    }
    valid.sort_by(|a, b| a.partial_cmp(b).unwrap());
    quantile_sorted(&valid, p)
}

/// Returns the sample variance of values in the slice.
///
/// Uses Bessel's correction (divides by n-1).
/// Returns `None` if the slice has fewer than 2 elements.
///
/// # Example
///
/// ```
/// use d3rs::array::variance;
///
/// let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
/// let var = variance(&data).unwrap();
/// assert!((var - 4.571428571428571).abs() < 1e-10);
/// ```
pub fn variance(data: &[f64]) -> Option<f64> {
    if data.len() < 2 {
        return None;
    }
    let m = mean(data)?;
    let sum_sq: f64 = data.iter().map(|x| (x - m).powi(2)).sum();
    Some(sum_sq / (data.len() - 1) as f64)
}

/// Returns the sample standard deviation of values in the slice.
///
/// Uses Bessel's correction (divides by n-1).
/// Returns `None` if the slice has fewer than 2 elements.
///
/// # Example
///
/// ```
/// use d3rs::array::deviation;
///
/// let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
/// let dev = deviation(&data).unwrap();
/// assert!((dev - 2.138089935299395).abs() < 1e-10);
/// ```
pub fn deviation(data: &[f64]) -> Option<f64> {
    variance(data).map(|v| v.sqrt())
}

/// Returns the cumulative sum of values in the slice.
///
/// # Example
///
/// ```
/// use d3rs::array::cumsum;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0];
/// assert_eq!(cumsum(&data), vec![1.0, 3.0, 6.0, 10.0]);
/// ```
pub fn cumsum(data: &[f64]) -> Vec<f64> {
    let mut result = Vec::with_capacity(data.len());
    let mut acc = 0.0;
    for &x in data {
        acc += x;
        result.push(acc);
    }
    result
}

/// Returns the index of the minimum value in the slice.
///
/// # Example
///
/// ```
/// use d3rs::array::min_index;
///
/// let data = vec![3, 1, 4, 1, 5, 9];
/// assert_eq!(min_index(&data), Some(1));
/// ```
pub fn min_index<T: Ord>(data: &[T]) -> Option<usize> {
    if data.is_empty() {
        return None;
    }
    let mut min_idx = 0;
    for (i, v) in data.iter().enumerate().skip(1) {
        if v < &data[min_idx] {
            min_idx = i;
        }
    }
    Some(min_idx)
}

/// Returns the index of the maximum value in the slice.
///
/// # Example
///
/// ```
/// use d3rs::array::max_index;
///
/// let data = vec![3, 1, 4, 1, 5, 9];
/// assert_eq!(max_index(&data), Some(5));
/// ```
pub fn max_index<T: Ord>(data: &[T]) -> Option<usize> {
    if data.is_empty() {
        return None;
    }
    let mut max_idx = 0;
    for (i, v) in data.iter().enumerate().skip(1) {
        if v > &data[max_idx] {
            max_idx = i;
        }
    }
    Some(max_idx)
}

/// Returns the count of values in the slice that satisfy the predicate.
///
/// # Example
///
/// ```
/// use d3rs::array::count;
///
/// let data = vec![1, 2, 3, 4, 5, 6];
/// assert_eq!(count(&data, |x| x % 2 == 0), 3);
/// ```
pub fn count<T, F>(data: &[T], predicate: F) -> usize
where
    F: Fn(&T) -> bool,
{
    data.iter().filter(|x| predicate(x)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_max() {
        let data = vec![3, 1, 4, 1, 5, 9, 2, 6];
        assert_eq!(min(&data), Some(&1));
        assert_eq!(max(&data), Some(&9));
    }

    #[test]
    fn test_extent() {
        let data = vec![3, 1, 4, 1, 5, 9, 2, 6];
        assert_eq!(extent(&data), Some((&1, &9)));
        assert_eq!(extent::<i32>(&[]), None);
    }

    #[test]
    fn test_sum() {
        let data = vec![1, 2, 3, 4, 5];
        assert_eq!(sum(&data), 15);
    }

    #[test]
    fn test_mean() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(mean(&data), Some(3.0));
        assert_eq!(mean(&[]), None);
    }

    #[test]
    fn test_median() {
        let mut odd = vec![3.0, 1.0, 4.0, 1.0, 5.0];
        assert_eq!(median(&mut odd), Some(3.0));

        let mut even = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0];
        assert_eq!(median(&mut even), Some(3.5));
    }

    #[test]
    fn test_quantile() {
        let mut data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(quantile(&mut data, 0.0), Some(1.0));
        assert_eq!(quantile(&mut data, 0.5), Some(3.0));
        assert_eq!(quantile(&mut data, 1.0), Some(5.0));
    }

    #[test]
    fn test_variance_deviation() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let var = variance(&data).unwrap();
        let dev = deviation(&data).unwrap();
        assert!((var - 4.571428571428571).abs() < 1e-10);
        assert!((dev - 2.138089935299395).abs() < 1e-10);
    }

    #[test]
    fn test_cumsum() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(cumsum(&data), vec![1.0, 3.0, 6.0, 10.0]);
    }

    #[test]
    fn test_min_max_index() {
        let data = vec![3, 1, 4, 1, 5, 9, 2, 6];
        assert_eq!(min_index(&data), Some(1));
        assert_eq!(max_index(&data), Some(5));
    }

    #[test]
    fn test_count() {
        let data = vec![1, 2, 3, 4, 5, 6];
        assert_eq!(count(&data, |x| x % 2 == 0), 3);
        assert_eq!(count(&data, |x| *x > 10), 0);
    }
}
