//! Shared value scaling utilities for audio UI components
//!
//! Provides linear and logarithmic scaling for parameters like
//! frequency (Hz), gain (dB), Q factor, etc.

/// Scale type for value mapping between UI position and actual value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Scale {
    /// Linear scale (default) - equal increments
    #[default]
    Linear,
    /// Logarithmic scale - for frequency, etc.
    /// Values must be positive (min > 0)
    Logarithmic,
}

impl Scale {
    /// Convert a value to normalized position [0, 1] based on scale type
    pub fn value_to_normalized(self, value: f64, min: f64, max: f64) -> f64 {
        match self {
            Scale::Linear => {
                if max > min {
                    ((value - min) / (max - min)).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }
            Scale::Logarithmic => {
                // For log scale, min must be > 0
                let min = min.max(1e-10);
                let max = max.max(min + 1e-10);
                let value = value.clamp(min, max);
                let log_min = min.ln();
                let log_max = max.ln();
                ((value.ln() - log_min) / (log_max - log_min)).clamp(0.0, 1.0)
            }
        }
    }

    /// Convert a normalized position [0, 1] to a value based on scale type
    pub fn normalized_to_value(self, normalized: f64, min: f64, max: f64) -> f64 {
        match self {
            Scale::Linear => min + normalized * (max - min),
            Scale::Logarithmic => {
                // For log scale, min must be > 0
                let min = min.max(1e-10);
                let max = max.max(min + 1e-10);
                let log_min = min.ln();
                let log_max = max.ln();
                (log_min + normalized * (log_max - log_min)).exp()
            }
        }
    }

    /// Compute new value after stepping in normalized space
    /// `direction`: 1.0 for increase, -1.0 for decrease
    /// `step_percent`: step size as fraction (e.g., 0.05 for 5%)
    pub fn step_value(
        self,
        current: f64,
        min: f64,
        max: f64,
        direction: f64,
        step_percent: f64,
    ) -> f64 {
        let current_norm = self.value_to_normalized(current, min, max);
        let new_norm = (current_norm + step_percent * direction).clamp(0.0, 1.0);
        self.normalized_to_value(new_norm, min, max)
    }
}

/// Default step sizes for scroll/keyboard adjustments
pub mod step_sizes {
    /// Normal scroll/keyboard step (5% of range)
    pub const NORMAL: f64 = 0.05;
    /// Fine step when Shift is held (0.5% of range)
    pub const FINE: f64 = 0.005;
    /// Large step when Ctrl/Cmd is held (10% of range)
    pub const LARGE: f64 = 0.1;
}

#[cfg(test)]
mod tests {
    use super::Scale;

    #[test]
    fn linear_value_to_normalized_is_bounded() {
        assert_eq!(Scale::Linear.value_to_normalized(0.0, 0.0, 100.0), 0.0);
        assert_eq!(Scale::Linear.value_to_normalized(50.0, 0.0, 100.0), 0.5);
        assert_eq!(Scale::Linear.value_to_normalized(100.0, 0.0, 100.0), 1.0);
        assert_eq!(Scale::Linear.value_to_normalized(-10.0, 0.0, 100.0), 0.0);
        assert_eq!(Scale::Linear.value_to_normalized(200.0, 0.0, 100.0), 1.0);
    }

    #[test]
    fn linear_value_to_normalized_handles_bad_range() {
        assert_eq!(Scale::Linear.value_to_normalized(5.0, 10.0, 10.0), 0.0);
        assert_eq!(Scale::Linear.value_to_normalized(5.0, 10.0, 0.0), 0.0);
    }

    #[test]
    fn linear_normalized_to_value_round_trips() {
        let scale = Scale::Linear;
        for norm in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let value = scale.normalized_to_value(norm, -10.0, 30.0);
            let back = scale.value_to_normalized(value, -10.0, 30.0);
            assert!((back - norm).abs() < 1e-9);
        }
    }

    #[test]
    fn logarithmic_conversion_round_trips() {
        let scale = Scale::Logarithmic;
        let min = 20.0;
        let max = 20_000.0;
        for value in [20.0, 100.0, 1000.0, 20_000.0] {
            let norm = scale.value_to_normalized(value, min, max);
            let back = scale.normalized_to_value(norm, min, max);
            assert!((back - value).abs() < 1e-6 * value.max(1.0));
        }
    }

    #[test]
    fn logarithmic_conversion_sanitizes_non_positive_range() {
        let scale = Scale::Logarithmic;
        assert_eq!(scale.value_to_normalized(-5.0, 0.0, 100.0), 0.0);
        assert_eq!(scale.value_to_normalized(50.0, -10.0, 0.0), 1.0);
        let value = scale.normalized_to_value(0.5, 0.0, -10.0);
        assert!(value > 0.0 && value.is_finite());
    }

    #[test]
    fn step_value_respects_bounds_and_direction() {
        let scale = Scale::Linear;
        assert!((scale.step_value(50.0, 0.0, 100.0, 1.0, 0.05) - 55.0).abs() < 1e-9);
        assert!((scale.step_value(50.0, 0.0, 100.0, -1.0, 0.05) - 45.0).abs() < 1e-9);
        assert!((scale.step_value(98.0, 0.0, 100.0, 1.0, 0.05) - 100.0).abs() < 1e-9);
        assert!((scale.step_value(2.0, 0.0, 100.0, -1.0, 0.05) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn step_sizes_are_documented_fractions() {
        assert_eq!(super::step_sizes::NORMAL, 0.05);
        assert_eq!(super::step_sizes::FINE, 0.005);
        assert_eq!(super::step_sizes::LARGE, 0.1);
    }
}
