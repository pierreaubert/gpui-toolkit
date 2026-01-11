//! Power scale implementation (including sqrt scale)
//!
//! Power scales are similar to linear scales, but apply an exponential
//! transform to the input domain before computing the output range.

use super::{Scale, nice_number};

/// A power scale applies an exponential transform before linear interpolation
///
/// Power scales are useful for data that follows a power law distribution.
/// The `exponent` parameter controls the shape of the transformation.
///
/// # Example
///
/// ```
/// use d3rs::scale::{PowScale, Scale};
///
/// let scale = PowScale::new()
///     .domain(0.0, 100.0)
///     .range(0.0, 10.0)
///     .exponent(0.5); // sqrt scale
///
/// // sqrt(0) * 10 / sqrt(100) = 0
/// assert!((scale.scale(0.0) - 0.0).abs() < 1e-6);
/// // sqrt(100) * 10 / sqrt(100) = 10
/// assert!((scale.scale(100.0) - 10.0).abs() < 1e-6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowScale {
    domain_min: f64,
    domain_max: f64,
    range_min: f64,
    range_max: f64,
    exponent: f64,
    clamped: bool,
}

impl Default for PowScale {
    fn default() -> Self {
        Self::new()
    }
}

impl PowScale {
    /// Create a new power scale with default domain [0, 1], range [0, 1], and exponent 1
    pub fn new() -> Self {
        Self {
            domain_min: 0.0,
            domain_max: 1.0,
            range_min: 0.0,
            range_max: 1.0,
            exponent: 1.0,
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

    /// Set the exponent for the power transform
    ///
    /// Common values:
    /// - 1.0: linear (default)
    /// - 0.5: square root
    /// - 2.0: quadratic
    pub fn exponent(mut self, exp: f64) -> Self {
        self.exponent = exp;
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

    /// Get the exponent
    pub fn exponent_value(&self) -> f64 {
        self.exponent
    }

    /// Check if clamping is enabled
    pub fn is_clamped(&self) -> bool {
        self.clamped
    }

    /// Apply power transform (handles negative values)
    fn pow(&self, x: f64) -> f64 {
        if x < 0.0 {
            -(-x).powf(self.exponent)
        } else {
            x.powf(self.exponent)
        }
    }

    /// Apply inverse power transform (handles negative values)
    fn pow_inv(&self, x: f64) -> f64 {
        if x < 0.0 {
            -(-x).powf(1.0 / self.exponent)
        } else {
            x.powf(1.0 / self.exponent)
        }
    }
}

impl Scale<f64, f64> for PowScale {
    fn scale(&self, value: f64) -> f64 {
        let value = if self.clamped {
            value.clamp(
                self.domain_min.min(self.domain_max),
                self.domain_min.max(self.domain_max),
            )
        } else {
            value
        };

        let pow_min = self.pow(self.domain_min);
        let pow_max = self.pow(self.domain_max);
        let pow_value = self.pow(value);

        let t = (pow_value - pow_min) / (pow_max - pow_min);
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
        let pow_min = self.pow(self.domain_min);
        let pow_max = self.pow(self.domain_max);
        let pow_value = pow_min + t * (pow_max - pow_min);

        Some(self.pow_inv(pow_value))
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

/// Type alias for sqrt scale (power scale with exponent 0.5)
///
/// Sqrt scales are useful for sizing circles by area, since area is proportional
/// to the square of the radius.
///
/// Use [`sqrt_scale()`] to create a properly configured sqrt scale with exponent 0.5.
///
/// # Example
///
/// ```
/// use d3rs::scale::{sqrt_scale, Scale};
///
/// let scale = sqrt_scale()
///     .domain(0.0, 100.0)
///     .range(0.0, 10.0);
///
/// // sqrt(25) / sqrt(100) * 10 = 5
/// assert!((scale.scale(25.0) - 5.0).abs() < 1e-6);
/// ```
pub type SqrtScale = PowScale;

/// Create a new sqrt scale
pub fn sqrt_scale() -> PowScale {
    PowScale::new().exponent(0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_pow_scale_linear() {
        // With exponent 1, should behave like linear scale
        let scale = PowScale::new()
            .domain(0.0, 100.0)
            .range(0.0, 500.0)
            .exponent(1.0);

        assert_relative_eq!(scale.scale(0.0), 0.0);
        assert_relative_eq!(scale.scale(50.0), 250.0);
        assert_relative_eq!(scale.scale(100.0), 500.0);
    }

    #[test]
    fn test_pow_scale_sqrt() {
        let scale = PowScale::new()
            .domain(0.0, 100.0)
            .range(0.0, 10.0)
            .exponent(0.5);

        assert_relative_eq!(scale.scale(0.0), 0.0);
        assert_relative_eq!(scale.scale(25.0), 5.0);
        assert_relative_eq!(scale.scale(100.0), 10.0);
    }

    #[test]
    fn test_pow_scale_quadratic() {
        let scale = PowScale::new()
            .domain(0.0, 10.0)
            .range(0.0, 100.0)
            .exponent(2.0);

        // pow(5, 2) / pow(10, 2) = 25/100 = 0.25 -> range 25
        assert_relative_eq!(scale.scale(0.0), 0.0);
        assert_relative_eq!(scale.scale(5.0), 25.0);
        assert_relative_eq!(scale.scale(10.0), 100.0);
    }

    #[test]
    fn test_pow_scale_invert() {
        let scale = PowScale::new()
            .domain(0.0, 100.0)
            .range(0.0, 10.0)
            .exponent(0.5);

        assert_relative_eq!(scale.invert(0.0).unwrap(), 0.0);
        assert_relative_eq!(scale.invert(5.0).unwrap(), 25.0);
        assert_relative_eq!(scale.invert(10.0).unwrap(), 100.0);
    }

    #[test]
    fn test_pow_scale_clamped() {
        let scale = PowScale::new()
            .domain(0.0, 100.0)
            .range(0.0, 10.0)
            .exponent(0.5)
            .clamp(true);

        assert_relative_eq!(scale.scale(-10.0), 0.0);
        assert_relative_eq!(scale.scale(200.0), 10.0);
    }

    #[test]
    fn test_sqrt_scale() {
        let scale = sqrt_scale().domain(0.0, 100.0).range(0.0, 10.0);

        assert_relative_eq!(scale.scale(25.0), 5.0);
        assert_relative_eq!(scale.exponent_value(), 0.5);
    }

    #[test]
    fn test_pow_scale_negative_domain() {
        let scale = PowScale::new()
            .domain(-100.0, 100.0)
            .range(0.0, 1.0)
            .exponent(0.5);

        // Should handle negative values symmetrically
        let neg = scale.scale(-100.0);
        let pos = scale.scale(100.0);
        assert_relative_eq!(pos, 1.0);
        assert_relative_eq!(neg, 0.0);
    }
}
