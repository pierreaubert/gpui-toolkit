//! Threshold scale implementation
//!
//! Threshold scales map arbitrary input values to discrete output based
//! on explicit threshold boundaries.

use super::Scale;

/// A threshold scale maps values based on explicit thresholds
///
/// Unlike quantize scales which divide the domain uniformly, threshold scales
/// use explicitly specified threshold values to determine the output.
///
/// # Example
///
/// ```
/// use d3rs::scale::{ThresholdScale, Scale};
///
/// let scale = ThresholdScale::new()
///     .domain(vec![0.0, 50.0, 100.0])  // thresholds
///     .range(vec!["very low", "low", "medium", "high"]);  // n+1 outputs
///
/// assert_eq!(scale.scale(-10.0), "very low");
/// assert_eq!(scale.scale(25.0), "low");
/// assert_eq!(scale.scale(75.0), "medium");
/// assert_eq!(scale.scale(150.0), "high");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdScale<R: Clone> {
    thresholds: Vec<f64>,
    range_values: Vec<R>,
}

impl<R: Clone + Default> Default for ThresholdScale<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Clone + Default> ThresholdScale<R> {
    /// Create a new threshold scale
    pub fn new() -> Self {
        Self {
            thresholds: vec![],
            range_values: vec![],
        }
    }
}

impl<R: Clone> ThresholdScale<R> {
    /// Create a new threshold scale with specified range
    pub fn with_range(range: Vec<R>) -> Self {
        Self {
            thresholds: vec![],
            range_values: range,
        }
    }

    /// Set the domain thresholds
    ///
    /// The thresholds divide the domain into n+1 regions,
    /// where n is the number of thresholds.
    pub fn domain(mut self, thresholds: Vec<f64>) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// Set the range (discrete output values)
    ///
    /// The number of range values should be one more than the number
    /// of thresholds.
    pub fn range(mut self, values: Vec<R>) -> Self {
        self.range_values = values;
        self
    }

    /// Get the thresholds
    pub fn thresholds(&self) -> &[f64] {
        &self.thresholds
    }

    /// Get the range values
    pub fn range_values(&self) -> &[R] {
        &self.range_values
    }

    /// Get the extent of domain values that map to a specific range index
    pub fn invert_extent(&self, index: usize) -> Option<(f64, f64)> {
        let n = self.range_values.len();
        if index >= n {
            return None;
        }

        let min = if index == 0 {
            f64::NEG_INFINITY
        } else {
            self.thresholds[index - 1]
        };

        let max = if index >= self.thresholds.len() {
            f64::INFINITY
        } else {
            self.thresholds[index]
        };

        Some((min, max))
    }

    /// Create a copy of this scale
    pub fn copy(&self) -> Self {
        Self {
            thresholds: self.thresholds.clone(),
            range_values: self.range_values.clone(),
        }
    }
}

impl<R: Clone> Scale<f64, R> for ThresholdScale<R> {
    fn scale(&self, value: f64) -> R {
        if self.range_values.is_empty() {
            panic!("ThresholdScale requires at least one range value");
        }

        // Find which bucket the value falls into
        let index = self
            .thresholds
            .iter()
            .position(|&t| value < t)
            .unwrap_or(self.thresholds.len());

        // Clamp to valid range index
        let index = index.min(self.range_values.len() - 1);
        self.range_values[index].clone()
    }

    fn invert(&self, _value: R) -> Option<f64> {
        // Cannot invert a threshold scale to a single value
        None
    }

    fn ticks(&self, _count: usize) -> Vec<f64> {
        self.thresholds.clone()
    }

    fn domain(&self) -> (f64, f64) {
        if self.thresholds.is_empty() {
            (0.0, 1.0)
        } else {
            (self.thresholds[0], *self.thresholds.last().unwrap())
        }
    }

    fn range(&self) -> (R, R) {
        if self.range_values.is_empty() {
            panic!("ThresholdScale requires at least one range value");
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
    fn test_threshold_scale_basic() {
        let scale = ThresholdScale::with_range(vec!["a", "b", "c"]).domain(vec![0.0, 1.0]);

        assert_eq!(scale.scale(-0.5), "a");
        assert_eq!(scale.scale(0.5), "b");
        assert_eq!(scale.scale(1.5), "c");
    }

    #[test]
    fn test_threshold_scale_boundaries() {
        let scale = ThresholdScale::with_range(vec!["low", "mid", "high"]).domain(vec![33.0, 66.0]);

        // Values exactly at threshold should go to next bucket
        assert_eq!(scale.scale(32.9), "low");
        assert_eq!(scale.scale(33.0), "mid");
        assert_eq!(scale.scale(65.9), "mid");
        assert_eq!(scale.scale(66.0), "high");
    }

    #[test]
    fn test_threshold_scale_multiple() {
        let scale = ThresholdScale::with_range(vec!["F", "D", "C", "B", "A"])
            .domain(vec![60.0, 70.0, 80.0, 90.0]);

        assert_eq!(scale.scale(55.0), "F");
        assert_eq!(scale.scale(65.0), "D");
        assert_eq!(scale.scale(75.0), "C");
        assert_eq!(scale.scale(85.0), "B");
        assert_eq!(scale.scale(95.0), "A");
    }

    #[test]
    fn test_threshold_scale_invert_extent() {
        let scale = ThresholdScale::with_range(vec!["a", "b", "c"]).domain(vec![0.0, 1.0]);

        let extent = scale.invert_extent(0).unwrap();
        assert!(extent.0.is_infinite() && extent.0 < 0.0);
        assert!((extent.1 - 0.0).abs() < 1e-6);

        let extent = scale.invert_extent(1).unwrap();
        assert!((extent.0 - 0.0).abs() < 1e-6);
        assert!((extent.1 - 1.0).abs() < 1e-6);

        let extent = scale.invert_extent(2).unwrap();
        assert!((extent.0 - 1.0).abs() < 1e-6);
        assert!(extent.1.is_infinite() && extent.1 > 0.0);
    }

    #[test]
    fn test_threshold_scale_numeric_range() {
        let scale: ThresholdScale<f64> =
            ThresholdScale::with_range(vec![0.0, 0.5, 1.0]).domain(vec![33.0, 66.0]);

        assert!((scale.scale(0.0) - 0.0).abs() < 1e-6);
        assert!((scale.scale(50.0) - 0.5).abs() < 1e-6);
        assert!((scale.scale(100.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_threshold_scale_single_threshold() {
        let scale = ThresholdScale::with_range(vec!["negative", "positive"]).domain(vec![0.0]);

        assert_eq!(scale.scale(-1.0), "negative");
        assert_eq!(scale.scale(0.0), "positive");
        assert_eq!(scale.scale(1.0), "positive");
    }
}
