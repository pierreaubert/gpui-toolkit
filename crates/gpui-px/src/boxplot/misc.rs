/// Calculate percentile using linear interpolation on a sorted slice.
pub(super) fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let n = sorted.len();
    let index = p * (n - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let frac = index - lower as f64;

    if lower == upper || upper >= n {
        sorted[lower.min(n - 1)]
    } else {
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}

/// Calculate percentile using linear interpolation without requiring the input
/// to be fully sorted. Uses `select_nth_unstable` to find the one or two order
/// statistics needed, giving expected O(n) time instead of O(n log n).
#[cfg(test)]
pub(super) fn percentile_unsorted(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return values[0];
    }

    let n = values.len();
    let index = p * (n - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    let mut scratch = values.to_vec();

    if lower == upper || upper >= n {
        let (_, kth, _) = scratch.select_nth_unstable_by(lower.min(n - 1), |a, b| a.total_cmp(b));
        *kth
    } else {
        let (_, kth_lower, right) = scratch.select_nth_unstable_by(lower, |a, b| a.total_cmp(b));
        // `upper` is `lower + 1` when `index` is non-integer, so the upper
        // statistic is the smallest element of the right partition.
        let (_, kth_upper, _) = right.select_nth_unstable_by(0, |a, b| a.total_cmp(b));
        let frac = index - lower as f64;
        *kth_lower * (1.0 - frac) + *kth_upper * frac
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_unsorted_matches_sorted() {
        let values = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        for p in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!(
                (percentile(&sorted, p) - percentile_unsorted(&values, p)).abs() < 1e-10,
                "mismatch at p={}",
                p
            );
        }
    }

    #[test]
    fn test_percentile_unsorted_single_value() {
        let values = vec![42.0];
        assert_eq!(percentile_unsorted(&values, 0.5), 42.0);
    }

    #[test]
    fn test_percentile_unsorted_empty() {
        let values: Vec<f64> = vec![];
        assert_eq!(percentile_unsorted(&values, 0.5), 0.0);
    }

    #[test]
    fn test_percentile_endpoints() {
        let sorted = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&sorted, 0.0), 1.0);
        assert_eq!(percentile(&sorted, 1.0), 5.0);
    }

    #[test]
    fn test_percentile_two_values() {
        let sorted = vec![10.0, 20.0];
        assert_eq!(percentile(&sorted, 0.25), 12.5);
        assert_eq!(percentile(&sorted, 0.5), 15.0);
        assert_eq!(percentile(&sorted, 0.75), 17.5);
    }

    #[test]
    fn test_percentile_duplicates() {
        let sorted = vec![1.0, 1.0, 1.0, 1.0];
        assert_eq!(percentile(&sorted, 0.5), 1.0);
    }

    #[test]
    fn test_percentile_single_value() {
        let sorted = vec![42.0];
        assert_eq!(percentile(&sorted, 0.5), 42.0);
    }

    #[test]
    fn test_percentile_unsorted_duplicates() {
        let values = vec![3.0, 1.0, 2.0, 1.0, 3.0];
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for p in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!(
                (percentile(&sorted, p) - percentile_unsorted(&values, p)).abs() < 1e-10,
                "mismatch at p={}",
                p
            );
        }
    }
}
