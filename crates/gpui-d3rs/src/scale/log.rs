//! Logarithmic scale implementation

use super::{Scale, generate_log_ticks};

/// A logarithmic scale maps a continuous domain to a continuous range using logarithmic transformation
///
/// # Example
///
/// ```
/// use d3rs::scale::{LogScale, Scale};
///
/// let scale = LogScale::new()
///     .domain(1.0, 1000.0)
///     .range(0.0, 500.0);
///
/// assert!((scale.scale(1.0) - 0.0).abs() < 0.01);
/// assert!((scale.scale(1000.0) - 500.0).abs() < 0.01);
/// ```
///
/// # Panics
///
/// The domain must contain only positive values. Setting a non-positive domain
/// will panic in debug mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogScale {
    domain_min: f64,
    domain_max: f64,
    range_min: f64,
    range_max: f64,
    base: f64,
}

impl Default for LogScale {
    fn default() -> Self {
        Self::new()
    }
}

impl LogScale {
    /// Create a new logarithmic scale with default domain [1, 10] and range [0, 1], base 10
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::LogScale;
    ///
    /// let scale = LogScale::new();
    /// ```
    pub fn new() -> Self {
        Self {
            domain_min: 1.0,
            domain_max: 10.0,
            range_min: 0.0,
            range_max: 1.0,
            base: 10.0,
        }
    }

    /// Set the domain (input extent)
    ///
    /// Both values must be positive for logarithmic scaling.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if min or max are not positive.
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::LogScale;
    ///
    /// let scale = LogScale::new().domain(20.0, 20000.0);
    /// ```
    pub fn domain(mut self, min: f64, max: f64) -> Self {
        debug_assert!(
            min > 0.0,
            "Log scale domain minimum must be positive, got {}",
            min
        );
        debug_assert!(
            max > 0.0,
            "Log scale domain maximum must be positive, got {}",
            max
        );
        self.domain_min = min;
        self.domain_max = max;
        self
    }

    /// Set the range (output extent)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::LogScale;
    ///
    /// let scale = LogScale::new().range(0.0, 500.0);
    /// ```
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.range_min = min;
        self.range_max = max;
        self
    }

    /// Set the logarithmic base (default is 10)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::LogScale;
    ///
    /// let scale = LogScale::new().base(2.0); // Binary logarithm
    /// ```
    pub fn base(mut self, base: f64) -> Self {
        debug_assert!(
            base > 0.0 && base != 1.0,
            "Log scale base must be positive and not 1.0"
        );
        self.base = base;
        self
    }

    /// Convenience method to set range from 0.0 to max (for normalized coordinates)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::scale::LogScale;
    ///
    /// let scale = LogScale::new().range_normalized(1.0);
    /// ```
    pub fn range_normalized(self, max: f64) -> Self {
        self.range(0.0, max)
    }
}

impl Scale<f64, f64> for LogScale {
    fn scale(&self, value: f64) -> f64 {
        let log_min = self.domain_min.log(self.base);
        let log_max = self.domain_max.log(self.base);
        let log_val = value.clamp(self.domain_min, self.domain_max).log(self.base);

        let t = (log_val - log_min) / (log_max - log_min);
        self.range_min + t * (self.range_max - self.range_min)
    }

    fn invert(&self, value: f64) -> Option<f64> {
        let log_min = self.domain_min.log(self.base);
        let log_max = self.domain_max.log(self.base);

        let t = (value - self.range_min) / (self.range_max - self.range_min);
        let log_val = log_min + t * (log_max - log_min);

        Some(self.base.powf(log_val))
    }

    fn ticks(&self, count: usize) -> Vec<f64> {
        // Use subdivisions if count is large
        let subdivisions = count > 10;
        generate_log_ticks(self.domain_min, self.domain_max, self.base, subdivisions)
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
    fn test_log_scale_basic() {
        let scale = LogScale::new().domain(1.0, 1000.0).range(0.0, 1.0);

        assert_relative_eq!(scale.scale(1.0), 0.0, epsilon = 1e-10);
        assert_relative_eq!(scale.scale(10.0), 1.0 / 3.0, epsilon = 1e-10);
        assert_relative_eq!(scale.scale(100.0), 2.0 / 3.0, epsilon = 1e-10);
        assert_relative_eq!(scale.scale(1000.0), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_log_scale_frequency() {
        let scale = LogScale::new().domain(20.0, 20000.0).range(0.0, 1.0);

        assert_relative_eq!(scale.scale(20.0), 0.0, epsilon = 1e-10);
        assert_relative_eq!(scale.scale(20000.0), 1.0, epsilon = 1e-10);

        // Geometric mean should be near 0.5
        let geometric_mean = (20.0_f64 * 20000.0_f64).sqrt();
        assert_relative_eq!(scale.scale(geometric_mean), 0.5, epsilon = 1e-6);
    }

    #[test]
    fn test_log_scale_invert() {
        let scale = LogScale::new().domain(1.0, 1000.0).range(0.0, 500.0);

        assert_relative_eq!(scale.invert(0.0).unwrap(), 1.0, epsilon = 1e-6);
        assert_relative_eq!(scale.invert(500.0).unwrap(), 1000.0, epsilon = 1e-6);

        // Middle value
        let mid_val = scale.invert(250.0).unwrap();
        assert_relative_eq!(mid_val, (1.0_f64 * 1000.0_f64).sqrt(), epsilon = 1e-6);
    }

    #[test]
    fn test_log_scale_base_2() {
        let scale = LogScale::new().domain(1.0, 16.0).range(0.0, 1.0).base(2.0);

        assert_relative_eq!(scale.scale(1.0), 0.0, epsilon = 1e-10);
        assert_relative_eq!(scale.scale(2.0), 0.25, epsilon = 1e-10);
        assert_relative_eq!(scale.scale(4.0), 0.5, epsilon = 1e-10);
        assert_relative_eq!(scale.scale(8.0), 0.75, epsilon = 1e-10);
        assert_relative_eq!(scale.scale(16.0), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_log_scale_roundtrip() {
        let scale = LogScale::new().domain(1.0, 1000.0).range(0.0, 500.0);

        for value in [1.0, 10.0, 100.0, 1000.0] {
            let scaled = scale.scale(value);
            let inverted = scale.invert(scaled).unwrap();
            assert_relative_eq!(inverted, value, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_log_scale_inverted_range() {
        let scale = LogScale::new().domain(1.0, 1000.0).range(500.0, 0.0); // Inverted range

        assert_relative_eq!(scale.scale(1.0), 500.0, epsilon = 1e-10);
        assert_relative_eq!(scale.scale(1000.0), 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_log_scale_clamping() {
        let scale = LogScale::new().domain(10.0, 100.0).range(0.0, 1.0);

        // Values outside domain should be clamped
        assert_relative_eq!(scale.scale(5.0), 0.0, epsilon = 1e-10); // Clamped to 10
        assert_relative_eq!(scale.scale(200.0), 1.0, epsilon = 1e-10); // Clamped to 100
    }

    #[test]
    fn test_log_scale_ticks() {
        let scale = LogScale::new().domain(1.0, 1000.0).range(0.0, 1.0);

        let ticks = scale.ticks(5);

        // Should include powers of 10
        assert!(ticks.contains(&1.0));
        assert!(ticks.contains(&10.0));
        assert!(ticks.contains(&100.0));
        assert!(ticks.contains(&1000.0));
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn test_log_scale_negative_domain() {
        LogScale::new().domain(-10.0, 10.0);
    }
}
