//! Tick marks component for meter scales
//!
//! Provides tick mark rendering with support for different scale types:
//! - Linear: uniform spacing
//! - Quadratic: emphasizes values near the maximum (good for dB scales near 0)
//! - Logarithmic: true logarithmic spacing

// intentional-file: chart axis tick rendering

use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::sync::Arc;

/// Scale type for positioning tick marks and meter values
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScaleType {
    /// Linear scale: position = (value - min) / (max - min)
    Linear,
    /// Quadratic scale: position = ((value - min) / (max - min))^2
    /// Emphasizes values near max (spreads out the top, compresses the bottom)
    Quadratic,
    /// Logarithmic scale: position = log(value - min + 1) / log(max - min + 1)
    /// Good for frequency or amplitude displays
    Logarithmic,
}

impl ScaleType {
    /// Convert a value to a position (0.0 to 1.0) based on the scale type
    pub fn value_to_position(&self, value: f64, min: f64, max: f64) -> f32 {
        let normalized = ((value - min) / (max - min)).clamp(0.0, 1.0);

        let position = match self {
            ScaleType::Linear => normalized,
            ScaleType::Quadratic => {
                // Square the normalized value to emphasize the top
                normalized * normalized
            }
            ScaleType::Logarithmic => {
                // Use log scale (add 1 to avoid log(0))
                let range = max - min;
                if range <= 0.0 {
                    0.0
                } else {
                    let log_val = ((value - min).max(0.0) + 1.0).ln();
                    let log_max = (range + 1.0).ln();
                    (log_val / log_max).clamp(0.0, 1.0)
                }
            }
        };

        position as f32
    }

    /// Convert a position (0.0 to 1.0) back to a value based on the scale type
    #[allow(dead_code)]
    pub fn position_to_value(&self, position: f32, min: f64, max: f64) -> f64 {
        let pos = (position as f64).clamp(0.0, 1.0);

        let normalized = match self {
            ScaleType::Linear => pos,
            ScaleType::Quadratic => {
                // Square root to reverse the quadratic
                pos.sqrt()
            }
            ScaleType::Logarithmic => {
                let range = max - min;
                if range <= 0.0 {
                    0.0
                } else {
                    let log_max = (range + 1.0).ln();
                    (pos * log_max).exp() - 1.0
                }
            }
        };

        min + normalized * (max - min)
    }
}

/// Tick mark definition
#[derive(Clone, Debug)]
pub struct TickMark {
    /// Position from 0.0 (left/bottom) to 1.0 (right/top)
    pub position: f32,
    /// Whether this is a major tick (big) or minor tick (small)
    pub is_major: bool,
    /// Optional label for the tick
    pub label: Option<SharedString>,
}

/// Configuration for tick mark generation
pub struct TickConfig {
    /// Scale type for positioning
    pub scale: ScaleType,
    /// Minimum value of the range
    pub min: f64,
    /// Maximum value of the range
    pub max: f64,
    /// Values where major ticks should appear
    pub major_values: Vec<f64>,
    /// Number of minor ticks between each pair of major ticks
    pub minor_count: usize,
    /// Height of major ticks in pixels
    pub major_height: f32,
    /// Height of minor ticks in pixels (typically 1/3 of major)
    pub minor_height: f32,
    /// Color for tick marks
    pub tick_color: Rgba,
    /// Cached tick marks generated from this configuration.
    cache: RefCell<Option<Arc<[TickMark]>>>,
}

impl Clone for TickConfig {
    fn clone(&self) -> Self {
        Self {
            scale: self.scale,
            min: self.min,
            max: self.max,
            major_values: self.major_values.clone(),
            minor_count: self.minor_count,
            major_height: self.major_height,
            minor_height: self.minor_height,
            tick_color: self.tick_color,
            cache: RefCell::new(None),
        }
    }
}

impl Default for TickConfig {
    fn default() -> Self {
        Self {
            scale: ScaleType::Quadratic,
            min: -60.0,
            max: 0.0,
            major_values: vec![-60.0, -30.0, -10.0, 0.0],
            minor_count: 4,
            major_height: 8.0,
            minor_height: 3.0,
            tick_color: Rgba {
                r: 0.55,
                g: 0.55,
                b: 0.55,
                a: 1.0,
            },
            cache: RefCell::new(None),
        }
    }
}

impl TickConfig {
    /// Create a new tick config with the given scale type
    pub fn new(scale: ScaleType, min: f64, max: f64) -> Self {
        Self {
            scale,
            min,
            max,
            ..Default::default()
        }
    }

    /// Set the major tick values
    pub fn with_major_values(mut self, values: Vec<f64>) -> Self {
        self.major_values = values;
        self.cache.replace(None);
        self
    }

    /// Set the number of minor ticks between major ticks
    pub fn with_minor_count(mut self, count: usize) -> Self {
        self.minor_count = count;
        self.cache.replace(None);
        self
    }

    /// Set the tick heights
    pub fn with_heights(mut self, major: f32, minor: f32) -> Self {
        self.major_height = major;
        self.minor_height = minor;
        self
    }

    /// Generate tick marks based on configuration
    pub fn generate_ticks(&self) -> Arc<[TickMark]> {
        if let Some(cached) = self.cache.borrow().as_ref() {
            return cached.clone();
        }

        let mut ticks = Vec::new();

        // Add major ticks
        for &value in &self.major_values {
            if value >= self.min && value <= self.max {
                ticks.push(TickMark {
                    position: self.scale.value_to_position(value, self.min, self.max),
                    is_major: true,
                    label: None,
                });
            }
        }

        // Add minor ticks between major ticks
        if self.minor_count > 0 && self.major_values.len() >= 2 {
            for i in 0..self.major_values.len() - 1 {
                let start_val = self.major_values[i];
                let end_val = self.major_values[i + 1];
                let step = (end_val - start_val) / (self.minor_count + 1) as f64;

                for j in 1..=self.minor_count {
                    let value = start_val + step * j as f64;
                    if value > self.min && value < self.max {
                        ticks.push(TickMark {
                            position: self.scale.value_to_position(value, self.min, self.max),
                            is_major: false,
                            label: None,
                        });
                    }
                }
            }
        }

        let ticks: Arc<[TickMark]> = ticks.into();
        self.cache.replace(Some(ticks.clone()));
        ticks
    }

    /// Convert a value to position using this config's scale
    pub fn value_to_position(&self, value: f64) -> f32 {
        self.scale.value_to_position(value, self.min, self.max)
    }
}

/// Render tick marks as a horizontal row aligned with a meter bar
///
/// Uses the same flex layout as meter bars to ensure proper alignment:
/// \[label_spacer\] \[gap\] \[tick_area\] \[gap\] \[value_spacer\]
///
/// # Arguments
/// * `config` - Tick configuration
/// * `label_width` - Width of the label area (matches meter bar label)
/// * `value_width` - Width of the value area (matches meter bar value display)
/// * `gap` - Gap between elements (matches meter bar gap)
pub fn render_tick_row(
    config: &TickConfig,
    label_width: f32,
    value_width: f32,
) -> impl IntoElement {
    let ticks = config.generate_ticks();
    let major_height = config.major_height;
    let minor_height = config.minor_height;
    let tick_color = config.tick_color;
    let gap = 4.0; // Same gap as meter bar

    div()
        .flex()
        .items_center()
        .gap(px(gap))
        .h(px(major_height))
        // Label spacer (same width as meter bar label)
        .child(div().w(px(label_width)))
        // Tick area (flex-1, same as meter bar)
        // Ticks hang from top (aligned with bar above)
        .child(
            div()
                .flex_1()
                .h(px(major_height))
                .relative()
                .children(ticks.iter().map(move |tick| {
                    let height = if tick.is_major {
                        major_height
                    } else {
                        minor_height
                    };

                    div()
                        .absolute()
                        .left(gpui::Length::Definite(gpui::DefiniteLength::Fraction(
                            tick.position,
                        )))
                        .top_0() // All ticks start from top
                        .w(px(1.0))
                        .h(px(height))
                        .bg(tick_color)
                        .ml(px(-0.5)) // Center the tick on the position
                })),
        )
        // Value spacer (same width as meter bar value display)
        .child(div().w(px(value_width)))
}

/// Preset configurations for common use cases
impl TickConfig {
    /// True Peak scale (-60 to +6 dB, quadratic)
    pub fn true_peak() -> Self {
        Self {
            scale: ScaleType::Quadratic,
            min: -60.0,
            max: 6.0,
            major_values: vec![-60.0, -30.0, -10.0, 0.0, 6.0],
            minor_count: 4,
            ..Default::default()
        }
    }

    /// LUFS scale (-60 to 0 dB, quadratic)
    pub fn lufs() -> Self {
        Self {
            scale: ScaleType::Quadratic,
            min: -60.0,
            max: 0.0,
            major_values: vec![-60.0, -30.0, -10.0, 0.0],
            minor_count: 4,
            ..Default::default()
        }
    }

    /// Stereo width scale (0 to 1, linear)
    pub fn stereo_width() -> Self {
        Self {
            scale: ScaleType::Linear,
            min: 0.0,
            max: 1.0,
            major_values: vec![0.0, 0.5, 1.0],
            minor_count: 4,
            ..Default::default()
        }
    }

    /// Peak spread scale (0 to 24 dB, linear)
    pub fn peak_spread() -> Self {
        Self {
            scale: ScaleType::Linear,
            min: 0.0,
            max: 24.0,
            major_values: vec![0.0, 6.0, 12.0, 24.0],
            minor_count: 4,
            ..Default::default()
        }
    }

    /// Percentage scale (0 to 100, linear)
    pub fn percentage() -> Self {
        Self {
            scale: ScaleType::Linear,
            min: 0.0,
            max: 100.0,
            major_values: vec![0.0, 25.0, 50.0, 75.0, 100.0],
            minor_count: 4,
            ..Default::default()
        }
    }

    /// Gain reduction scale (0 to max_db, linear)
    /// For displaying compressor/limiter gain reduction
    pub fn gain_reduction(max_db: f64) -> Self {
        Self {
            scale: ScaleType::Linear,
            min: 0.0,
            max: max_db.abs(),
            major_values: vec![0.0, 10.0, 20.0, max_db.abs()],
            minor_count: 4,
            ..Default::default()
        }
    }

    /// dB scale with custom range (linear)
    pub fn db_linear(min: f64, max: f64) -> Self {
        let step = (max - min) / 4.0;
        Self {
            scale: ScaleType::Linear,
            min,
            max,
            major_values: vec![min, min + step, min + 2.0 * step, min + 3.0 * step, max],
            minor_count: 4,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ScaleType, TickConfig, TickMark, render_tick_row};

    #[test]
    fn scale_type_linear_value_to_position_round_trips() {
        let scale = ScaleType::Linear;
        assert_eq!(scale.value_to_position(0.0, 0.0, 100.0), 0.0);
        assert_eq!(scale.value_to_position(50.0, 0.0, 100.0), 0.5);
        assert_eq!(scale.value_to_position(100.0, 0.0, 100.0), 1.0);
        assert_eq!(scale.value_to_position(-10.0, 0.0, 100.0), 0.0);
        assert_eq!(scale.value_to_position(200.0, 0.0, 100.0), 1.0);

        assert_eq!(scale.position_to_value(0.5, 0.0, 100.0), 50.0);
        assert_eq!(scale.position_to_value(0.0, -10.0, 30.0), -10.0);
        assert_eq!(scale.position_to_value(1.0, -10.0, 30.0), 30.0);
    }

    #[test]
    fn scale_type_quadratic_emphasizes_top() {
        let scale = ScaleType::Quadratic;
        let mid = scale.value_to_position(50.0, 0.0, 100.0);
        assert!(mid < 0.5);
        assert_eq!(scale.position_to_value(mid, 0.0, 100.0), 50.0);
    }

    #[test]
    fn scale_type_logarithmic_handles_positive_range() {
        let scale = ScaleType::Logarithmic;
        let min = 20.0;
        let max = 20_000.0;
        assert_eq!(scale.value_to_position(min, min, max), 0.0);
        assert_eq!(scale.value_to_position(max, min, max), 1.0);
        let mid = scale.value_to_position(1_000.0, min, max);
        assert!(mid > 0.0 && mid < 1.0);
        assert!(scale.position_to_value(mid, min, max) >= min);
    }

    #[test]
    fn scale_type_logarithmic_handles_bad_range() {
        let scale = ScaleType::Logarithmic;
        assert_eq!(scale.value_to_position(5.0, 10.0, 10.0), 0.0);
        // Degenerate range keeps the min anchor.
        assert_eq!(scale.position_to_value(0.5, 10.0, 10.0), 10.0);
    }

    #[test]
    fn generate_ticks_is_cached() {
        let config = TickConfig::lufs();
        let ticks_a = config.generate_ticks();
        let ticks_b = config.generate_ticks();
        assert_eq!(ticks_a.len(), ticks_b.len());
        assert!(ticks_a.iter().zip(ticks_b.iter()).all(|(a, b)| {
            a.position == b.position && a.is_major == b.is_major && a.label == b.label
        }));
    }

    #[test]
    fn tick_config_clone_clears_cache() {
        let config = TickConfig::percentage();
        let _ = config.generate_ticks();
        let cloned = config.clone();
        assert!(cloned.cache.borrow().is_none());
    }

    #[test]
    fn tick_config_presets_generate_major_ticks() {
        for config in [
            TickConfig::true_peak(),
            TickConfig::lufs(),
            TickConfig::stereo_width(),
            TickConfig::peak_spread(),
            TickConfig::percentage(),
            TickConfig::gain_reduction(30.0),
            TickConfig::db_linear(-40.0, 10.0),
        ] {
            let ticks = config.generate_ticks();
            assert!(!ticks.is_empty());
            assert!(ticks.iter().any(|t| t.is_major));
        }
    }

    #[test]
    fn tick_config_with_major_values_and_minor_count() {
        let config = TickConfig::new(ScaleType::Linear, 0.0, 100.0)
            .with_major_values(vec![0.0, 50.0, 100.0])
            .with_minor_count(2)
            .with_heights(12.0, 4.0);
        let ticks = config.generate_ticks();
        assert!(
            ticks
                .iter()
                .any(|t| t.is_major && (t.position - 0.5).abs() < 0.01)
        );
        assert!(ticks.iter().any(|t| !t.is_major));
    }

    #[test]
    fn tick_config_value_to_position_matches_scale() {
        let config = TickConfig::percentage();
        assert_eq!(config.value_to_position(0.0), 0.0);
        assert_eq!(config.value_to_position(100.0), 1.0);
        assert_eq!(config.value_to_position(50.0), 0.5);
    }

    #[test]
    fn render_tick_row_is_constructible() {
        let config = TickConfig::lufs();
        let _row = render_tick_row(&config, 32.0, 50.0);
    }

    #[test]
    fn tick_mark_fields_are_settable() {
        let mark = TickMark {
            position: 0.5,
            is_major: true,
            label: Some("0".into()),
        };
        assert_eq!(mark.position, 0.5);
        assert!(mark.is_major);
        assert_eq!(mark.label.as_ref().unwrap(), "0");
    }
}
