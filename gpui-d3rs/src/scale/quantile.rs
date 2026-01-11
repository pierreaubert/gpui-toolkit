//! Quantile scale implementation
//!
//! Quantile scales map a sampled input domain to a discrete range by
//! dividing the domain into equal-sized groups based on sample quantiles.

use super::Scale;

/// A quantile scale divides domain samples into equal-sized groups
///
/// Unlike quantize scales which divide the domain uniformly, quantile scales
/// divide based on the actual distribution of sample data. Each range value
/// corresponds to roughly the same number of domain samples.
///
/// # Example
///
/// ```
/// use d3rs::scale::{QuantileScale, Scale};
///
/// let scale = QuantileScale::new()
///     .domain(vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0])
///     .range(vec!["low", "high"]);
///
/// // First 3 values map to "low", last 3 to "high"
/// assert_eq!(scale.scale(2.0), "low");
/// assert_eq!(scale.scale(100.0), "high");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct QuantileScale<R: Clone> {
    domain_samples: Vec<f64>,
    range_values: Vec<R>,
    thresholds_cache: Vec<f64>,
}

impl<R: Clone + Default> Default for QuantileScale<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Clone + Default> QuantileScale<R> {
    /// Create a new quantile scale
    pub fn new() -> Self {
        Self {
            domain_samples: vec![],
            range_values: vec![],
            thresholds_cache: vec![],
        }
    }
}

impl<R: Clone> QuantileScale<R> {
    /// Create a new quantile scale with specified range
    pub fn with_range(range: Vec<R>) -> Self {
        Self {
            domain_samples: vec![],
            range_values: range,
            thresholds_cache: vec![],
        }
    }

    /// Set the domain samples
    ///
    /// The samples are sorted internally to compute quantiles.
    pub fn domain(mut self, mut samples: Vec<f64>) -> Self {
        // Remove NaN values and sort
        samples.retain(|x| !x.is_nan());
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        self.domain_samples = samples;
        self.recompute_thresholds();
        self
    }

    /// Set the range (discrete output values)
    pub fn range(mut self, values: Vec<R>) -> Self {
        self.range_values = values;
        self.recompute_thresholds();
        self
    }

    /// Get the domain samples
    pub fn domain_samples(&self) -> &[f64] {
        &self.domain_samples
    }

    /// Get the range values
    pub fn range_values(&self) -> &[R] {
        &self.range_values
    }

    /// Get the quantile thresholds
    pub fn quantiles(&self) -> &[f64] {
        &self.thresholds_cache
    }

    /// Recompute thresholds when domain or range changes
    fn recompute_thresholds(&mut self) {
        let n = self.range_values.len();
        if n == 0 || self.domain_samples.is_empty() {
            self.thresholds_cache = vec![];
            return;
        }

        // Compute n-1 threshold values (quantile boundaries)
        self.thresholds_cache = (1..n)
            .map(|i| {
                let p = i as f64 / n as f64;
                self.compute_quantile(p)
            })
            .collect();
    }

    /// Compute the quantile value for a given probability p in [0, 1]
    fn compute_quantile(&self, p: f64) -> f64 {
        let n = self.domain_samples.len();
        if n == 0 {
            return 0.0;
        }

        let index = p * (n - 1) as f64;
        let lower = index.floor() as usize;
        let upper = index.ceil() as usize;
        let t = index - lower as f64;

        if lower == upper || upper >= n {
            self.domain_samples[lower.min(n - 1)]
        } else {
            self.domain_samples[lower] * (1.0 - t) + self.domain_samples[upper] * t
        }
    }

    /// Get the extent of domain values that map to a specific range index
    pub fn invert_extent(&self, index: usize) -> Option<(f64, f64)> {
        let n = self.range_values.len();
        if index >= n || self.domain_samples.is_empty() {
            return None;
        }

        let min = if index == 0 {
            self.domain_samples[0]
        } else {
            self.thresholds_cache[index - 1]
        };

        let max = if index == n - 1 {
            *self.domain_samples.last().unwrap()
        } else {
            self.thresholds_cache[index]
        };

        Some((min, max))
    }

    /// Create a copy of this scale
    pub fn copy(&self) -> Self {
        Self {
            domain_samples: self.domain_samples.clone(),
            range_values: self.range_values.clone(),
            thresholds_cache: self.thresholds_cache.clone(),
        }
    }
}

impl<R: Clone> Scale<f64, R> for QuantileScale<R> {
    fn scale(&self, value: f64) -> R {
        let n = self.range_values.len();
        if n == 0 {
            panic!("QuantileScale requires at least one range value");
        }

        // Find which bucket the value falls into
        let index = self
            .thresholds_cache
            .iter()
            .position(|&t| value < t)
            .unwrap_or(n - 1);

        self.range_values[index].clone()
    }

    fn invert(&self, _value: R) -> Option<f64> {
        // Cannot invert a quantile scale to a single value
        None
    }

    fn ticks(&self, _count: usize) -> Vec<f64> {
        self.thresholds_cache.clone()
    }

    fn domain(&self) -> (f64, f64) {
        if self.domain_samples.is_empty() {
            (0.0, 1.0)
        } else {
            (self.domain_samples[0], *self.domain_samples.last().unwrap())
        }
    }

    fn range(&self) -> (R, R) {
        if self.range_values.is_empty() {
            panic!("QuantileScale requires at least one range value");
        }
        (
            self.range_values.first().unwrap().clone(),
            self.range_values.last().unwrap().clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantile_scale_basic() {
        let scale = QuantileScale::with_range(vec!["a", "b"]).domain(vec![1.0, 2.0, 3.0, 4.0]);

        // First half maps to "a", second half to "b"
        assert_eq!(scale.scale(1.0), "a");
        assert_eq!(scale.scale(2.0), "a");
        assert_eq!(scale.scale(3.0), "b");
        assert_eq!(scale.scale(4.0), "b");
    }

    #[test]
    fn test_quantile_scale_uneven() {
        // Domain with outlier
        let scale = QuantileScale::with_range(vec!["low", "high"])
            .domain(vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0]);

        // Should split at median, not at 50
        assert_eq!(scale.scale(3.0), "low");
        assert_eq!(scale.scale(4.0), "high");
    }

    #[test]
    fn test_quantile_scale_quantiles() {
        let scale = QuantileScale::with_range(vec!["a", "b", "c", "d"])
            .domain(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);

        let quantiles = scale.quantiles();
        assert_eq!(quantiles.len(), 3); // n-1 thresholds for n buckets
    }

    #[test]
    fn test_quantile_scale_three_buckets() {
        let scale = QuantileScale::with_range(vec!["low", "mid", "high"])
            .domain(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);

        // Each bucket should get ~3 samples
        assert_eq!(scale.scale(1.0), "low");
        assert_eq!(scale.scale(3.0), "low");
        assert_eq!(scale.scale(5.0), "mid");
        assert_eq!(scale.scale(7.0), "high");
        assert_eq!(scale.scale(9.0), "high");
    }

    #[test]
    fn test_quantile_scale_invert_extent() {
        let scale = QuantileScale::with_range(vec!["a", "b", "c"])
            .domain(vec![0.0, 3.0, 6.0, 9.0, 12.0, 15.0]);

        let extent = scale.invert_extent(0);
        assert!(extent.is_some());
        let (min, _max) = extent.unwrap();
        assert!((min - 0.0).abs() < 1e-6);
        // Middle extent should match quantile threshold
    }

    #[test]
    fn test_quantile_scale_numeric_range() {
        let scale: QuantileScale<f64> = QuantileScale::with_range(vec![0.0, 0.5, 1.0])
            .domain(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        assert!((scale.scale(1.0) - 0.0).abs() < 1e-6);
        assert!((scale.scale(3.0) - 0.5).abs() < 1e-6);
        assert!((scale.scale(6.0) - 1.0).abs() < 1e-6);
    }
}
