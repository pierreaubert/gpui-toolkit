//! Threshold calculation functions
//!
//! Provides various methods for calculating optimal thresholds for contour generation.

/// Calculate thresholds using Sturges' formula.
///
/// Returns n evenly spaced thresholds where n = ceil(log2(count) + 1).
///
/// # Example
///
/// ```
/// use d3rs::contour::threshold_sturges;
///
/// let thresholds = threshold_sturges(0.0, 100.0, 100);
/// assert!(thresholds.len() >= 7);
/// ```
pub fn threshold_sturges(min: f64, max: f64, count: usize) -> Vec<f64> {
    if count == 0 || min >= max {
        return vec![];
    }

    let n = (count as f64).log2().ceil() as usize + 1;
    linspace(min, max, n)
}

/// Calculate thresholds using Scott's rule.
///
/// Based on the normal reference rule: h = 3.5 * std / n^(1/3).
///
/// # Example
///
/// ```
/// use d3rs::contour::threshold_scott;
///
/// let values: Vec<f64> = (0..100).map(|i| i as f64).collect();
/// let thresholds = threshold_scott(&values, 0.0, 100.0);
/// ```
pub fn threshold_scott(values: &[f64], min: f64, max: f64) -> Vec<f64> {
    if values.is_empty() || min >= max {
        return vec![];
    }

    let n = values.len() as f64;
    let std = standard_deviation(values);

    if std <= 0.0 {
        return vec![(min + max) / 2.0];
    }

    let h = 3.5 * std / n.powf(1.0 / 3.0);
    let num_bins = ((max - min) / h).ceil() as usize;

    linspace(min, max, num_bins.max(1))
}

/// Calculate thresholds using the Freedman-Diaconis rule.
///
/// Based on: h = 2 * IQR / n^(1/3).
///
/// # Example
///
/// ```
/// use d3rs::contour::threshold_freedman_diaconis;
///
/// let values: Vec<f64> = (0..100).map(|i| i as f64).collect();
/// let thresholds = threshold_freedman_diaconis(&values, 0.0, 100.0);
/// ```
pub fn threshold_freedman_diaconis(values: &[f64], min: f64, max: f64) -> Vec<f64> {
    if values.is_empty() || min >= max {
        return vec![];
    }

    let n = values.len() as f64;
    let iqr = interquartile_range(values);

    if iqr <= 0.0 {
        return vec![(min + max) / 2.0];
    }

    let h = 2.0 * iqr / n.powf(1.0 / 3.0);
    let num_bins = ((max - min) / h).ceil() as usize;

    linspace(min, max, num_bins.max(1))
}

/// Generate n evenly spaced values between min and max.
fn linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(min + max) / 2.0];
    }

    (0..n)
        .map(|i| min + (max - min) * (i as f64) / ((n - 1) as f64))
        .collect()
}

/// Calculate the standard deviation of a slice of values.
fn standard_deviation(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    variance.sqrt()
}

/// Calculate the interquartile range (Q3 - Q1).
fn interquartile_range(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let q1 = quantile(&sorted, 0.25);
    let q3 = quantile(&sorted, 0.75);
    q3 - q1
}

/// Calculate a quantile from a sorted slice.
fn quantile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let n = sorted.len() as f64;
    let idx = p * (n - 1.0);
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    let frac = idx - lo as f64;

    if hi >= sorted.len() {
        sorted[sorted.len() - 1]
    } else {
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_sturges() {
        let thresholds = threshold_sturges(0.0, 100.0, 100);
        assert!(!thresholds.is_empty());
        assert!((thresholds[0] - 0.0).abs() < 0.001);
        assert!((thresholds[thresholds.len() - 1] - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_threshold_scott() {
        let values: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let thresholds = threshold_scott(&values, 0.0, 100.0);
        assert!(!thresholds.is_empty());
    }

    #[test]
    fn test_threshold_freedman_diaconis() {
        let values: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let thresholds = threshold_freedman_diaconis(&values, 0.0, 100.0);
        assert!(!thresholds.is_empty());
    }

    #[test]
    fn test_linspace() {
        let values = linspace(0.0, 10.0, 5);
        assert_eq!(values.len(), 5);
        assert!((values[0] - 0.0).abs() < 0.001);
        assert!((values[2] - 5.0).abs() < 0.001);
        assert!((values[4] - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_standard_deviation() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let std = standard_deviation(&values);
        assert!((std - 2.138).abs() < 0.01);
    }

    #[test]
    fn test_interquartile_range() {
        let values: Vec<f64> = (1..=10).map(|i| i as f64).collect();
        let iqr = interquartile_range(&values);
        assert!(iqr > 0.0);
    }
}
