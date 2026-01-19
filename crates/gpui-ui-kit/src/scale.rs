//! Shared value scaling utilities for audio UI components
//!
//! Provides linear and logarithmic scaling for parameters like
//! frequency (Hz), gain (dB), Q factor, etc.

/// Scale type for value mapping between UI position and actual value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
