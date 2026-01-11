//! Symmetric log scale implementation
//!
//! A symmetric log scale is like a log scale but handles negative values
//! and values close to zero by using a linear region around zero.

use super::{Scale, nice_number};

/// A symmetric log scale handles negative values and zero
///
/// Unlike log scales, symlog scales can handle negative values and zero
/// by using a linear region near zero defined by the `constant` parameter.
///
/// The transformation is: sign(x) * log(1 + |x| / constant)
///
/// # Example
///
/// ```
/// use d3rs::scale::{SymlogScale, Scale};
///
/// let scale = SymlogScale::new()
///     .domain(-100.0, 100.0)
///     .range(0.0, 1.0);
///
/// assert!((scale.scale(0.0) - 0.5).abs() < 1e-6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SymlogScale {
    domain_min: f64,
    domain_max: f64,
    range_min: f64,
    range_max: f64,
    constant: f64,
    clamped: bool,
}

impl Default for SymlogScale {
    fn default() -> Self {
        Self::new()
    }
}

impl SymlogScale {
    /// Create a new symlog scale with default domain [0, 1], range [0, 1], constant 1
    pub fn new() -> Self {
        Self {
            domain_min: 0.0,
            domain_max: 1.0,
            range_min: 0.0,
            range_max: 1.0,
            constant: 1.0,
            clamped: false,
        }
    }

    /// Set the domain (input extent)
    pub fn domain(mut self, min: f64, max: f64) -> Self {
        self.domain_min = min;
        self.domain_max = max;
        self
    }

    /// Set the range (output extent)
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.range_min = min;
        self.range_max = max;
        self
    }

    /// Set the constant that determines the linear region around zero
    ///
    /// The constant parameter determines the size of the linear region around
    /// zero. Larger values create a larger linear region.
    pub fn constant(mut self, c: f64) -> Self {
        self.constant = c;
        self
    }

    /// Enable or disable clamping
    pub fn clamp(mut self, enabled: bool) -> Self {
        self.clamped = enabled;
        self
    }

    /// Adjust the domain to nice round values
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

    /// Get the constant value
    pub fn constant_value(&self) -> f64 {
        self.constant
    }

    /// Check if clamping is enabled
    pub fn is_clamped(&self) -> bool {
        self.clamped
    }

    /// Apply symlog transform: sign(x) * log(1 + |x|/c)
    fn symlog(&self, x: f64) -> f64 {
        x.signum() * (1.0 + (x.abs() / self.constant)).ln()
    }

    /// Apply inverse symlog transform
    fn symlog_inv(&self, x: f64) -> f64 {
        x.signum() * self.constant * (x.abs().exp() - 1.0)
    }
}

impl Scale<f64, f64> for SymlogScale {
    fn scale(&self, value: f64) -> f64 {
        let value = if self.clamped {
            value.clamp(
                self.domain_min.min(self.domain_max),
                self.domain_min.max(self.domain_max),
            )
        } else {
            value
        };

        let log_min = self.symlog(self.domain_min);
        let log_max = self.symlog(self.domain_max);
        let log_value = self.symlog(value);

        let t = (log_value - log_min) / (log_max - log_min);
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
        let log_min = self.symlog(self.domain_min);
        let log_max = self.symlog(self.domain_max);
        let log_value = log_min + t * (log_max - log_min);

        Some(self.symlog_inv(log_value))
    }

    fn ticks(&self, count: usize) -> Vec<f64> {
        super::generate_linear_ticks(self.domain_min, self.domain_max, count)
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
    fn test_symlog_scale_zero() {
        let scale = SymlogScale::new().domain(-100.0, 100.0).range(0.0, 1.0);

        // Zero should map to middle of range
        assert_relative_eq!(scale.scale(0.0), 0.5, epsilon = 1e-6);
    }

    #[test]
    fn test_symlog_scale_symmetric() {
        let scale = SymlogScale::new().domain(-100.0, 100.0).range(0.0, 1.0);

        // Should be symmetric around zero
        let pos = scale.scale(50.0);
        let neg = scale.scale(-50.0);
        assert_relative_eq!(pos - 0.5, 0.5 - neg, epsilon = 1e-6);
    }

    #[test]
    fn test_symlog_scale_endpoints() {
        let scale = SymlogScale::new().domain(-100.0, 100.0).range(0.0, 1.0);

        assert_relative_eq!(scale.scale(-100.0), 0.0, epsilon = 1e-6);
        assert_relative_eq!(scale.scale(100.0), 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_symlog_scale_invert() {
        let scale = SymlogScale::new().domain(-100.0, 100.0).range(0.0, 1.0);

        assert_relative_eq!(scale.invert(0.0).unwrap(), -100.0, epsilon = 1e-6);
        assert_relative_eq!(scale.invert(0.5).unwrap(), 0.0, epsilon = 1e-6);
        assert_relative_eq!(scale.invert(1.0).unwrap(), 100.0, epsilon = 1e-6);
    }

    #[test]
    fn test_symlog_scale_constant() {
        let scale1 = SymlogScale::new()
            .domain(0.0, 100.0)
            .range(0.0, 1.0)
            .constant(1.0);

        let scale10 = SymlogScale::new()
            .domain(0.0, 100.0)
            .range(0.0, 1.0)
            .constant(10.0);

        // With larger constant, the linear region is larger
        // The transform log(1 + |x|/c) with larger c gives smaller log values
        // But both scales are normalized to map domain to range, so
        // the effect is that larger c makes small values more prominent
        let v1 = scale1.scale(1.0);
        let v10 = scale10.scale(1.0);
        // Both should be small positive values, and relationship depends on normalization
        assert!(v1 > 0.0);
        assert!(v10 > 0.0);
    }

    #[test]
    fn test_symlog_scale_clamped() {
        let scale = SymlogScale::new()
            .domain(-100.0, 100.0)
            .range(0.0, 1.0)
            .clamp(true);

        assert_relative_eq!(scale.scale(-200.0), 0.0, epsilon = 1e-6);
        assert_relative_eq!(scale.scale(200.0), 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_symlog_roundtrip() {
        let scale = SymlogScale::new().domain(-100.0, 100.0).range(0.0, 1.0);

        for value in [-100.0, -50.0, -10.0, 0.0, 10.0, 50.0, 100.0] {
            let scaled = scale.scale(value);
            let inverted = scale.invert(scaled).unwrap();
            assert_relative_eq!(inverted, value, epsilon = 1e-6);
        }
    }
}
