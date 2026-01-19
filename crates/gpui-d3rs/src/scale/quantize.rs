//! Quantize scale implementation
//!
//! Quantize scales map a continuous domain to a discrete range by
//! dividing the domain into uniform segments.

use super::Scale;

/// A quantize scale divides a continuous domain into uniform segments
///
/// Unlike linear scales, quantize scales map continuous input to discrete
/// output values. The domain is divided into n uniform segments where n
/// is the number of values in the range.
///
/// # Example
///
/// ```
/// use d3rs::scale::{QuantizeScale, Scale};
///
/// let scale = QuantizeScale::new()
///     .domain(0.0, 100.0)
///     .range(vec!["low", "medium", "high"]);
///
/// assert_eq!(scale.scale(10.0), "low");      // 0-33.3
/// assert_eq!(scale.scale(50.0), "medium");   // 33.3-66.6
/// assert_eq!(scale.scale(90.0), "high");     // 66.6-100
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct QuantizeScale<R: Clone> {
    domain_min: f64,
    domain_max: f64,
    range_values: Vec<R>,
}

impl<R: Clone + Default> Default for QuantizeScale<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Clone + Default> QuantizeScale<R> {
    /// Create a new quantize scale with default domain [0, 1]
    pub fn new() -> Self {
        Self {
            domain_min: 0.0,
            domain_max: 1.0,
            range_values: vec![],
        }
    }
}

impl<R: Clone> QuantizeScale<R> {
    /// Create a new quantize scale with specified range
    pub fn with_range(range: Vec<R>) -> Self {
        Self {
            domain_min: 0.0,
            domain_max: 1.0,
            range_values: range,
        }
    }

    /// Set the domain (input extent)
    pub fn domain(mut self, min: f64, max: f64) -> Self {
        self.domain_min = min;
        self.domain_max = max;
        self
    }

    /// Set the range (discrete output values)
    pub fn range(mut self, values: Vec<R>) -> Self {
        self.range_values = values;
        self
    }

    /// Get the domain minimum
    pub fn domain_min(&self) -> f64 {
        self.domain_min
    }

    /// Get the domain maximum
    pub fn domain_max(&self) -> f64 {
        self.domain_max
    }

    /// Get the range values
    pub fn range_values(&self) -> &[R] {
        &self.range_values
    }

    /// Get the thresholds that divide the domain
    pub fn thresholds(&self) -> Vec<f64> {
        let n = self.range_values.len();
        if n == 0 {
            return vec![];
        }

        let step = (self.domain_max - self.domain_min) / n as f64;
        (1..n).map(|i| self.domain_min + step * i as f64).collect()
    }

    /// Get the extent of domain values that map to a specific range value
    pub fn invert_extent(&self, index: usize) -> Option<(f64, f64)> {
        let n = self.range_values.len();
        if index >= n {
            return None;
        }

        let step = (self.domain_max - self.domain_min) / n as f64;
        let min = self.domain_min + step * index as f64;
        let max = min + step;
        Some((min, max))
    }

    /// Create a copy of this scale
    pub fn copy(&self) -> Self {
        Self {
            domain_min: self.domain_min,
            domain_max: self.domain_max,
            range_values: self.range_values.clone(),
        }
    }
}

impl<R: Clone> Scale<f64, R> for QuantizeScale<R> {
    fn scale(&self, value: f64) -> R {
        let n = self.range_values.len();
        if n == 0 {
            panic!("QuantizeScale requires at least one range value");
        }

        // Clamp and normalize to [0, 1]
        let t = ((value - self.domain_min) / (self.domain_max - self.domain_min)).clamp(0.0, 1.0);

        // Map to discrete index
        let index = (t * n as f64).floor() as usize;
        let index = index.min(n - 1);

        self.range_values[index].clone()
    }

    fn invert(&self, _value: R) -> Option<f64> {
        // Cannot invert a quantize scale to a single value
        None
    }

    fn ticks(&self, _count: usize) -> Vec<f64> {
        self.thresholds()
    }

    fn domain(&self) -> (f64, f64) {
        (self.domain_min, self.domain_max)
    }

    fn range(&self) -> (R, R) {
        if self.range_values.is_empty() {
            panic!("QuantizeScale requires at least one range value");
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
    fn test_quantize_scale_basic() {
        let scale = QuantizeScale::with_range(vec!["a", "b", "c"]).domain(0.0, 1.0);

        assert_eq!(scale.scale(0.0), "a");
        assert_eq!(scale.scale(0.33), "a");
        assert_eq!(scale.scale(0.34), "b");
        assert_eq!(scale.scale(0.66), "b");
        assert_eq!(scale.scale(0.67), "c");
        assert_eq!(scale.scale(1.0), "c");
    }

    #[test]
    fn test_quantize_scale_clamping() {
        let scale = QuantizeScale::with_range(vec![1, 2, 3]).domain(0.0, 100.0);

        // Values outside domain should clamp
        assert_eq!(scale.scale(-50.0), 1);
        assert_eq!(scale.scale(150.0), 3);
    }

    #[test]
    fn test_quantize_scale_thresholds() {
        let scale = QuantizeScale::with_range(vec!["a", "b", "c", "d"]).domain(0.0, 100.0);

        let thresholds = scale.thresholds();
        assert_eq!(thresholds.len(), 3);
        assert!((thresholds[0] - 25.0).abs() < 1e-6);
        assert!((thresholds[1] - 50.0).abs() < 1e-6);
        assert!((thresholds[2] - 75.0).abs() < 1e-6);
    }

    #[test]
    fn test_quantize_scale_invert_extent() {
        let scale = QuantizeScale::with_range(vec!["a", "b", "c", "d"]).domain(0.0, 100.0);

        let extent = scale.invert_extent(1).unwrap();
        assert!((extent.0 - 25.0).abs() < 1e-6);
        assert!((extent.1 - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_quantize_scale_numeric_range() {
        let scale: QuantizeScale<f64> =
            QuantizeScale::with_range(vec![0.0, 0.5, 1.0]).domain(0.0, 100.0);

        assert!((scale.scale(10.0) - 0.0).abs() < 1e-6);
        assert!((scale.scale(50.0) - 0.5).abs() < 1e-6);
        assert!((scale.scale(90.0) - 1.0).abs() < 1e-6);
    }
}
