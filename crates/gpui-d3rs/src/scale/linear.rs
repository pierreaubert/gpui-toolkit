//! Linear scale implementation

use super::{Scale, generate_linear_ticks, nice_number};

/// A linear scale maps a continuous domain to a continuous range using linear interpolation
///
/// # Example
///
/// ```
/// use d3rs::scale::{LinearScale, Scale};
///
/// let scale = LinearScale::new()
///     .domain(0.0, 100.0)
///     .range(0.0, 500.0);
///
/// assert_eq!(scale.scale(0.0), 0.0);
/// assert_eq!(scale.scale(50.0), 250.0);
/// assert_eq!(scale.scale(100.0), 500.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearScale {
    domain_min: f64,
    domain_max: f64,
    range_min: f64,
    range_max: f64,
    clamped: bool,
}

impl Default for LinearScale {
    fn default() -> Self {
        Self::new()
    }
}

impl LinearScale {
    /// Create a new linear scale with default domain [0, 1] and range [0, 1]
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::LinearScale;
    ///
    /// let scale = LinearScale::new();
    /// ```
    pub fn new() -> Self {
        Self {
            domain_min: 0.0,
            domain_max: 1.0,
            range_min: 0.0,
            range_max: 1.0,
            clamped: false,
        }
    }

    /// Set the domain (input extent)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::LinearScale;
    ///
    /// let scale = LinearScale::new().domain(0.0, 100.0);
    /// ```
    pub fn domain(mut self, min: f64, max: f64) -> Self {
        self.domain_min = min;
        self.domain_max = max;
        self
    }

    /// Set the range (output extent)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::LinearScale;
    ///
    /// let scale = LinearScale::new().range(0.0, 500.0);
    /// ```
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.range_min = min;
        self.range_max = max;
        self
    }

    /// Convenience method to set range from 0.0 to max (for normalized coordinates)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::LinearScale;
    ///
    /// let scale = LinearScale::new().range_normalized(1.0);
    /// ```
    pub fn range_normalized(self, max: f64) -> Self {
        self.range(0.0, max)
    }

    /// Clamp values to the domain
    ///
    /// When enabled, values outside the domain will be clamped to the domain extent.
    /// When disabled (default), extrapolation occurs for out-of-domain values.
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::{LinearScale, Scale};
    ///
    /// let scale = LinearScale::new()
    ///     .domain(0.0, 100.0)
    ///     .range(0.0, 500.0)
    ///     .clamp(true);
    ///
    /// // Values are clamped to domain
    /// assert_eq!(scale.scale(-50.0), 0.0);
    /// assert_eq!(scale.scale(150.0), 500.0);
    /// ```
    pub fn clamp(mut self, enabled: bool) -> Self {
        self.clamped = enabled;
        self
    }

    /// Adjust the domain to nice, round values
    ///
    /// This extends the domain to start and end on nice round values.
    /// The optional count parameter specifies the number of ticks to use
    /// for determining the step size.
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::LinearScale;
    ///
    /// let scale = LinearScale::new()
    ///     .domain(0.123, 0.987)
    ///     .nice(None);
    ///
    /// // Domain is now extended to nice values like [0.0, 1.0]
    /// let (min, max) = (scale.domain_min(), scale.domain_max());
    /// assert!(min <= 0.123);
    /// assert!(max >= 0.987);
    /// ```
    pub fn nice(mut self, count: Option<usize>) -> Self {
        let count = count.unwrap_or(10);
        let range = self.domain_max - self.domain_min;
        if range == 0.0 {
            return self;
        }

        let step = nice_number(range / (count as f64), true);
        self.domain_min = (self.domain_min / step).floor() * step;
        self.domain_max = (self.domain_max / step).ceil() * step;
        self
    }

    /// Create a copy of this scale
    pub fn copy(&self) -> Self {
        *self
    }

    /// Get the domain minimum
    pub fn domain_min(&self) -> f64 {
        self.domain_min
    }

    /// Get the domain maximum
    pub fn domain_max(&self) -> f64 {
        self.domain_max
    }

    /// Check if clamping is enabled
    pub fn is_clamped(&self) -> bool {
        self.clamped
    }
}

impl Scale<f64, f64> for LinearScale {
    fn scale(&self, value: f64) -> f64 {
        let value = if self.clamped {
            value.clamp(
                self.domain_min.min(self.domain_max),
                self.domain_min.max(self.domain_max),
            )
        } else {
            value
        };
        let t = (value - self.domain_min) / (self.domain_max - self.domain_min);
        self.range_min + t * (self.range_max - self.range_min)
    }

    fn invert(&self, value: f64) -> Option<f64> {
        let value = if self.clamped {
            value.clamp(
                self.range_min.min(self.range_max),
                self.range_min.max(self.range_max),
            )
        } else {
            value
        };
        let t = (value - self.range_min) / (self.range_max - self.range_min);
        Some(self.domain_min + t * (self.domain_max - self.domain_min))
    }

    fn ticks(&self, count: usize) -> Vec<f64> {
        generate_linear_ticks(self.domain_min, self.domain_max, count)
    }

    fn domain(&self) -> (f64, f64) {
        (self.domain_min, self.domain_max)
    }

    fn range(&self) -> (f64, f64) {
        (self.range_min, self.range_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_linear_scale_basic() {
        let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);

        assert_relative_eq!(scale.scale(0.0), 0.0);
        assert_relative_eq!(scale.scale(50.0), 250.0);
        assert_relative_eq!(scale.scale(100.0), 500.0);
    }

    #[test]
    fn test_linear_scale_inverted_range() {
        let scale = LinearScale::new().domain(0.0, 100.0).range(500.0, 0.0); // Inverted

        assert_relative_eq!(scale.scale(0.0), 500.0);
        assert_relative_eq!(scale.scale(50.0), 250.0);
        assert_relative_eq!(scale.scale(100.0), 0.0);
    }

    #[test]
    fn test_linear_scale_invert() {
        let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);

        assert_relative_eq!(scale.invert(0.0).unwrap(), 0.0);
        assert_relative_eq!(scale.invert(250.0).unwrap(), 50.0);
        assert_relative_eq!(scale.invert(500.0).unwrap(), 100.0);
    }

    #[test]
    fn test_linear_scale_extrapolation() {
        let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);

        // Values outside domain should extrapolate
        assert_relative_eq!(scale.scale(-50.0), -250.0);
        assert_relative_eq!(scale.scale(150.0), 750.0);
    }

    #[test]
    fn test_linear_scale_negative_domain() {
        let scale = LinearScale::new().domain(-100.0, 100.0).range(0.0, 1.0);

        assert_relative_eq!(scale.scale(-100.0), 0.0);
        assert_relative_eq!(scale.scale(0.0), 0.5);
        assert_relative_eq!(scale.scale(100.0), 1.0);
    }

    #[test]
    fn test_linear_scale_normalized() {
        let scale = LinearScale::new().domain(0.0, 100.0).range_normalized(1.0);

        assert_relative_eq!(scale.scale(0.0), 0.0);
        assert_relative_eq!(scale.scale(50.0), 0.5);
        assert_relative_eq!(scale.scale(100.0), 1.0);
    }

    #[test]
    fn test_linear_scale_roundtrip() {
        let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);

        for value in [0.0, 25.0, 50.0, 75.0, 100.0] {
            let scaled = scale.scale(value);
            let inverted = scale.invert(scaled).unwrap();
            assert_relative_eq!(inverted, value, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_linear_scale_clamped() {
        let scale = LinearScale::new()
            .domain(0.0, 100.0)
            .range(0.0, 500.0)
            .clamp(true);

        // Values within domain should work normally
        assert_relative_eq!(scale.scale(50.0), 250.0);

        // Values outside domain should be clamped
        assert_relative_eq!(scale.scale(-50.0), 0.0);
        assert_relative_eq!(scale.scale(150.0), 500.0);

        // Check clamped flag
        assert!(scale.is_clamped());
    }

    #[test]
    fn test_linear_scale_clamped_inverted_range() {
        let scale = LinearScale::new()
            .domain(0.0, 100.0)
            .range(500.0, 0.0)
            .clamp(true);

        // Clamping should still work with inverted range
        assert_relative_eq!(scale.scale(-50.0), 500.0);
        assert_relative_eq!(scale.scale(150.0), 0.0);
    }

    #[test]
    fn test_linear_scale_nice() {
        let scale = LinearScale::new().domain(0.123, 0.987).nice(None);

        // Domain should be extended to nice values
        // D3.js produces [0.1, 1] for domain [0.123, 0.987]
        assert!(scale.domain_min() <= 0.123);
        assert!(scale.domain_max() >= 0.987);
        // Should be nice round numbers
        assert_relative_eq!(scale.domain_min(), 0.1, epsilon = 1e-10);
        assert_relative_eq!(scale.domain_max(), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_linear_scale_nice_with_count() {
        let scale = LinearScale::new().domain(1.0, 99.0).nice(Some(5));

        // Domain should be extended to nice values
        assert!(scale.domain_min() <= 1.0);
        assert!(scale.domain_max() >= 99.0);
    }

    #[test]
    fn test_linear_scale_copy() {
        let scale = LinearScale::new()
            .domain(0.0, 100.0)
            .range(0.0, 500.0)
            .clamp(true);

        let copy = scale.copy();
        assert_eq!(Scale::domain(&scale), Scale::domain(&copy));
        assert_eq!(Scale::range(&scale), Scale::range(&copy));
        assert_eq!(scale.is_clamped(), copy.is_clamped());
    }
}
