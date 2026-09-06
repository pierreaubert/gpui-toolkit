//! Pie layout generator
//!
//! Computes the arc angles for pie and donut charts from data.

use std::f64::consts::PI;
use std::fmt;

use super::arc::ArcDatum;

/// A single slice in a pie chart.
#[derive(Debug, Clone)]
pub struct PieSlice<T> {
    /// The original data
    pub data: T,
    /// The computed arc datum
    pub arc: ArcDatum,
    /// Index in the original data
    pub index: usize,
    /// The value used for computing the angle
    pub value: f64,
}

/// Recoverable errors for checked pie layout input validation.
#[derive(Debug, Clone, PartialEq)]
pub enum PieLayoutError {
    /// Pie values must be finite.
    NonFiniteValue { index: usize, value: f64 },
    /// Checked pie values must be zero or positive.
    NegativeValue { index: usize, value: f64 },
    /// Layout parameters such as angles and radii must be finite.
    NonFiniteLayoutParameter { parameter: &'static str, value: f64 },
    /// Checked radii and padding must be zero or positive.
    NegativeLayoutParameter { parameter: &'static str, value: f64 },
}

impl fmt::Display for PieLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue { index, value } => {
                write!(f, "pie value at index {index} is not finite: {value}")
            }
            Self::NegativeValue { index, value } => {
                write!(f, "pie value at index {index} is negative: {value}")
            }
            Self::NonFiniteLayoutParameter { parameter, value } => {
                write!(f, "pie layout parameter {parameter} is not finite: {value}")
            }
            Self::NegativeLayoutParameter { parameter, value } => {
                write!(f, "pie layout parameter {parameter} is negative: {value}")
            }
        }
    }
}

impl std::error::Error for PieLayoutError {}

/// Pie layout generator.
///
/// Computes start and end angles for pie chart slices based on data values.
///
/// # Example
///
/// ```
/// use d3rs::shape::pie::Pie;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0];
/// let pie = Pie::new();
/// let slices = pie.generate(&data, |d| *d);
///
/// assert_eq!(slices.len(), 4);
/// // All slices should sum to 2π
/// let total_angle: f64 = slices.iter()
///     .map(|s| s.arc.end_angle - s.arc.start_angle)
///     .sum();
/// assert!((total_angle - std::f64::consts::PI * 2.0).abs() < 0.001);
/// ```
#[derive(Debug, Clone)]
pub struct Pie {
    /// Start angle in radians (default: 0)
    start_angle: f64,
    /// End angle in radians (default: 2π)
    end_angle: f64,
    /// Padding angle between slices
    pad_angle: f64,
    /// Inner radius for donut charts
    inner_radius: f64,
    /// Outer radius
    outer_radius: f64,
    /// Corner radius
    corner_radius: f64,
    /// Sort slices by value
    sort_values: bool,
    /// Sort descending (largest first)
    sort_descending: bool,
}

impl Default for Pie {
    fn default() -> Self {
        Self {
            start_angle: 0.0,
            end_angle: 2.0 * PI,
            pad_angle: 0.0,
            inner_radius: 0.0,
            outer_radius: 100.0,
            corner_radius: 0.0,
            sort_values: false,
            sort_descending: true,
        }
    }
}

impl Pie {
    /// Create a new pie layout generator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the start angle in radians.
    pub fn start_angle(mut self, angle: f64) -> Self {
        self.start_angle = angle;
        self
    }

    /// Set the end angle in radians.
    pub fn end_angle(mut self, angle: f64) -> Self {
        self.end_angle = angle;
        self
    }

    /// Set the padding angle between slices.
    pub fn pad_angle(mut self, angle: f64) -> Self {
        self.pad_angle = angle;
        self
    }

    /// Set the inner radius (for donut charts).
    pub fn inner_radius(mut self, radius: f64) -> Self {
        self.inner_radius = radius;
        self
    }

    /// Set the outer radius.
    pub fn outer_radius(mut self, radius: f64) -> Self {
        self.outer_radius = radius;
        self
    }

    /// Set the corner radius.
    pub fn corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Enable sorting slices by value.
    pub fn sort(mut self, sort: bool) -> Self {
        self.sort_values = sort;
        self
    }

    /// Sort in descending order (largest slices first).
    pub fn sort_descending(mut self, descending: bool) -> Self {
        self.sort_descending = descending;
        self
    }

    /// Generate pie slices from data.
    ///
    /// # Arguments
    ///
    /// * `data` - The input data
    /// * `value` - Function to extract the numeric value from each datum
    pub fn generate<T: Clone, F>(&self, data: &[T], value: F) -> Vec<PieSlice<T>>
    where
        F: Fn(&T) -> f64,
    {
        if data.is_empty() {
            return Vec::new();
        }

        // Extract values and compute indices
        let mut entries: Vec<(usize, T, f64)> = data
            .iter()
            .enumerate()
            .map(|(i, d)| (i, d.clone(), value(d)))
            .collect();

        // Sort if requested
        if self.sort_values {
            if self.sort_descending {
                entries.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            } else {
                entries.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
            }
        }

        self.generate_entries(entries)
    }

    /// Generate pie slices from data, returning recoverable errors for
    /// invalid user-provided values or layout parameters.
    ///
    /// `generate` keeps the older permissive behavior for compatibility. Use
    /// `try_generate` when data comes from files, user input, or other
    /// external sources where NaN, infinity, or negative values should be
    /// handled explicitly.
    pub fn try_generate<T: Clone, F>(
        &self,
        data: &[T],
        value: F,
    ) -> Result<Vec<PieSlice<T>>, PieLayoutError>
    where
        F: Fn(&T) -> f64,
    {
        validate_layout_parameter("start_angle", self.start_angle, false)?;
        validate_layout_parameter("end_angle", self.end_angle, false)?;
        validate_layout_parameter("pad_angle", self.pad_angle, true)?;
        validate_layout_parameter("inner_radius", self.inner_radius, true)?;
        validate_layout_parameter("outer_radius", self.outer_radius, true)?;
        validate_layout_parameter("corner_radius", self.corner_radius, true)?;

        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::with_capacity(data.len());
        for (index, datum) in data.iter().enumerate() {
            let value = value(datum);
            if !value.is_finite() {
                return Err(PieLayoutError::NonFiniteValue { index, value });
            }
            if value < 0.0 {
                return Err(PieLayoutError::NegativeValue { index, value });
            }
            entries.push((index, datum.clone(), value));
        }

        if self.sort_values {
            if self.sort_descending {
                entries.sort_by(|a, b| b.2.total_cmp(&a.2));
            } else {
                entries.sort_by(|a, b| a.2.total_cmp(&b.2));
            }
        }

        Ok(self.generate_entries(entries))
    }

    fn generate_entries<T>(&self, entries: Vec<(usize, T, f64)>) -> Vec<PieSlice<T>> {
        // Compute total value
        let total: f64 = entries.iter().map(|(_, _, v)| v.max(0.0)).sum();

        if total <= 0.0 {
            // All zeros or negative - return empty slices at start angle
            return entries
                .into_iter()
                .map(|(index, data, value)| PieSlice {
                    data,
                    arc: ArcDatum {
                        inner_radius: self.inner_radius,
                        outer_radius: self.outer_radius,
                        start_angle: self.start_angle,
                        end_angle: self.start_angle,
                        corner_radius: self.corner_radius,
                        pad_angle: self.pad_angle,
                    },
                    index,
                    value,
                })
                .collect();
        }

        // Match d3-shape `pie()`: slices are contiguous and each slice's
        // span includes its trailing pad (`a1 = a0 + v * k + pa` with
        // `k = (da - n * pa) / sum`). The pad is carved out by the arc
        // generator, not by the layout, so `start/end` angles agree with D3.
        let range = self.end_angle - self.start_angle;
        let n = entries.len();
        let pa = if n > 0 && range != 0.0 {
            self.pad_angle.min(range.abs() / n as f64).copysign(range)
        } else {
            0.0
        };
        let k = (range - n as f64 * pa) / total;

        // Generate slices
        let mut current_angle = self.start_angle;
        let mut slices = Vec::with_capacity(n);

        for (index, data, value) in entries {
            let slice_angle = if value > 0.0 { value * k } else { 0.0 };

            let start = current_angle;
            let end = current_angle + slice_angle + pa;

            slices.push(PieSlice {
                data,
                arc: ArcDatum {
                    inner_radius: self.inner_radius,
                    outer_radius: self.outer_radius,
                    start_angle: start,
                    end_angle: end,
                    corner_radius: self.corner_radius,
                    pad_angle: self.pad_angle,
                },
                index,
                value,
            });

            current_angle = end;
        }

        slices
    }
}

fn validate_layout_parameter(
    parameter: &'static str,
    value: f64,
    require_non_negative: bool,
) -> Result<(), PieLayoutError> {
    if !value.is_finite() {
        return Err(PieLayoutError::NonFiniteLayoutParameter { parameter, value });
    }
    if require_non_negative && value < 0.0 {
        return Err(PieLayoutError::NegativeLayoutParameter { parameter, value });
    }
    Ok(())
}

/// Generate a simple pie chart layout from values.
///
/// # Example
///
/// ```
/// use d3rs::shape::pie::pie;
///
/// let values = vec![10.0, 20.0, 30.0, 40.0];
/// let slices = pie(&values, 100.0);
///
/// assert_eq!(slices.len(), 4);
/// ```
pub fn pie(values: &[f64], radius: f64) -> Vec<PieSlice<f64>> {
    Pie::new().outer_radius(radius).generate(values, |v| *v)
}

/// Checked simple pie chart layout from values.
pub fn try_pie(values: &[f64], radius: f64) -> Result<Vec<PieSlice<f64>>, PieLayoutError> {
    Pie::new().outer_radius(radius).try_generate(values, |v| *v)
}

/// Generate a donut chart layout from values.
///
/// # Example
///
/// ```
/// use d3rs::shape::pie::donut;
///
/// let values = vec![10.0, 20.0, 30.0, 40.0];
/// let slices = donut(&values, 50.0, 100.0);
///
/// assert_eq!(slices.len(), 4);
/// assert_eq!(slices[0].arc.inner_radius, 50.0);
/// ```
pub fn donut(values: &[f64], inner_radius: f64, outer_radius: f64) -> Vec<PieSlice<f64>> {
    Pie::new()
        .inner_radius(inner_radius)
        .outer_radius(outer_radius)
        .generate(values, |v| *v)
}

/// Checked donut chart layout from values.
pub fn try_donut(
    values: &[f64],
    inner_radius: f64,
    outer_radius: f64,
) -> Result<Vec<PieSlice<f64>>, PieLayoutError> {
    Pie::new()
        .inner_radius(inner_radius)
        .outer_radius(outer_radius)
        .try_generate(values, |v| *v)
}

/// Generate a half-pie (semicircle) layout.
///
/// # Example
///
/// ```
/// use d3rs::shape::pie::half_pie;
/// use std::f64::consts::PI;
///
/// let values = vec![25.0, 75.0];
/// let slices = half_pie(&values, 100.0);
///
/// // Should span from -π/2 to π/2 (top half)
/// let first = &slices[0];
/// assert!((first.arc.start_angle - (-PI / 2.0)).abs() < 0.001);
/// ```
pub fn half_pie(values: &[f64], radius: f64) -> Vec<PieSlice<f64>> {
    Pie::new()
        .outer_radius(radius)
        .start_angle(-PI / 2.0)
        .end_angle(PI / 2.0)
        .generate(values, |v| *v)
}

/// Checked half-pie (semicircle) layout from values.
pub fn try_half_pie(values: &[f64], radius: f64) -> Result<Vec<PieSlice<f64>>, PieLayoutError> {
    Pie::new()
        .outer_radius(radius)
        .start_angle(-PI / 2.0)
        .end_angle(PI / 2.0)
        .try_generate(values, |v| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pie_basic() {
        let data = vec![1.0, 1.0, 1.0, 1.0];
        let slices = Pie::new().generate(&data, |d| *d);

        assert_eq!(slices.len(), 4);

        // Each slice should be π/2 (quarter of the circle)
        for slice in &slices {
            let angle = slice.arc.end_angle - slice.arc.start_angle;
            assert!((angle - PI / 2.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_pie_sorted() {
        let data = vec![1.0, 3.0, 2.0];
        let slices = Pie::new()
            .sort(true)
            .sort_descending(true)
            .generate(&data, |d| *d);

        // Should be sorted descending: 3, 2, 1
        assert_eq!(slices[0].value, 3.0);
        assert_eq!(slices[1].value, 2.0);
        assert_eq!(slices[2].value, 1.0);
    }

    #[test]
    fn test_pie_with_padding() {
        let data = vec![1.0, 1.0];
        let slices = Pie::new().pad_angle(0.1).generate(&data, |d| *d);

        // d3-shape semantics: layout slices stay contiguous (the pad lives
        // inside each slice span) and the arc generator carves it out.
        assert!((slices[0].arc.end_angle - slices[1].arc.start_angle).abs() < 1e-12);
        assert!((slices[0].arc.start_angle - 0.0).abs() < 1e-12);
        assert!((slices[1].arc.end_angle - 2.0 * PI).abs() < 1e-9);
        // Rendered width excludes the pad on both sides.
        let rendered = slices[0].arc.end_angle - slices[0].arc.start_angle - 0.1;
        assert!(rendered < PI);
        assert!((rendered - (PI - 0.1)).abs() < 1e-9);
    }

    #[test]
    fn test_donut() {
        let values = vec![10.0, 20.0, 30.0];
        let slices = donut(&values, 50.0, 100.0);

        assert_eq!(slices.len(), 3);
        assert_eq!(slices[0].arc.inner_radius, 50.0);
        assert_eq!(slices[0].arc.outer_radius, 100.0);
    }

    #[test]
    fn test_half_pie() {
        let values = vec![50.0, 50.0];
        let slices = half_pie(&values, 100.0);

        // Total angle should be π (half circle)
        let total: f64 = slices
            .iter()
            .map(|s| s.arc.end_angle - s.arc.start_angle)
            .sum();
        assert!((total - PI).abs() < 0.001);
    }

    #[test]
    fn test_pie_empty() {
        let data: Vec<f64> = vec![];
        let slices = Pie::new().generate(&data, |d| *d);
        assert!(slices.is_empty());
    }

    #[test]
    fn test_pie_zeros() {
        let data = vec![0.0, 0.0, 0.0];
        let slices = Pie::new().generate(&data, |d| *d);

        // All slices should have zero angle
        for slice in &slices {
            let angle = slice.arc.end_angle - slice.arc.start_angle;
            assert!(angle.abs() < 0.001);
        }
    }

    #[test]
    fn try_generate_accepts_valid_values() {
        let data = vec![1.0, 2.0, 3.0];
        let slices = Pie::new().try_generate(&data, |d| *d).unwrap();

        assert_eq!(slices.len(), 3);
        assert_eq!(slices[0].index, 0);
        assert_eq!(slices[1].value, 2.0);
    }

    #[test]
    fn try_generate_rejects_non_finite_and_negative_values() {
        let error = Pie::new()
            .try_generate(&[1.0, f64::NAN], |d| *d)
            .unwrap_err();
        match error {
            PieLayoutError::NonFiniteValue { index, value } => {
                assert_eq!(index, 1);
                assert!(value.is_nan());
            }
            error => panic!("unexpected error: {error:?}"),
        }

        let error = Pie::new().try_generate(&[1.0, -1.0], |d| *d).unwrap_err();
        assert_eq!(
            error,
            PieLayoutError::NegativeValue {
                index: 1,
                value: -1.0
            }
        );
    }

    #[test]
    fn try_generate_rejects_invalid_layout_parameters() {
        let error = Pie::new()
            .pad_angle(f64::INFINITY)
            .try_generate(&[1.0], |d| *d)
            .unwrap_err();
        assert_eq!(
            error,
            PieLayoutError::NonFiniteLayoutParameter {
                parameter: "pad_angle",
                value: f64::INFINITY
            }
        );

        let error = Pie::new()
            .outer_radius(-1.0)
            .try_generate(&[1.0], |d| *d)
            .unwrap_err();
        assert_eq!(
            error,
            PieLayoutError::NegativeLayoutParameter {
                parameter: "outer_radius",
                value: -1.0
            }
        );
    }

    #[test]
    fn generate_keeps_permissive_negative_value_behavior() {
        let slices = Pie::new().generate(&[1.0, -1.0], |d| *d);

        assert_eq!(slices.len(), 2);
        assert_eq!(slices[1].value, -1.0);
        assert_eq!(slices[1].arc.start_angle, slices[1].arc.end_angle);
    }

    #[test]
    fn checked_convenience_functions_match_permissive_helpers() {
        let values = vec![10.0, 20.0, 30.0, 40.0];

        let permissive = pie(&values, 100.0);
        let checked = try_pie(&values, 100.0).unwrap();
        assert_eq!(permissive.len(), checked.len());
        assert_eq!(permissive[0].arc.outer_radius, checked[0].arc.outer_radius);
        assert_eq!(permissive[3].arc.end_angle, checked[3].arc.end_angle);

        let permissive = donut(&values, 50.0, 100.0);
        let checked = try_donut(&values, 50.0, 100.0).unwrap();
        assert_eq!(permissive.len(), checked.len());
        assert_eq!(permissive[0].arc.inner_radius, checked[0].arc.inner_radius);
        assert_eq!(permissive[0].arc.outer_radius, checked[0].arc.outer_radius);

        let permissive = half_pie(&values, 100.0);
        let checked = try_half_pie(&values, 100.0).unwrap();
        assert_eq!(permissive.len(), checked.len());
        assert_eq!(permissive[0].arc.start_angle, checked[0].arc.start_angle);
        assert_eq!(permissive[3].arc.end_angle, checked[3].arc.end_angle);
    }

    #[test]
    fn checked_convenience_functions_reject_invalid_values_and_radii() {
        let error = try_pie(&[1.0, f64::NAN], 100.0).unwrap_err();
        match error {
            PieLayoutError::NonFiniteValue { index, value } => {
                assert_eq!(index, 1);
                assert!(value.is_nan());
            }
            error => panic!("unexpected error: {error:?}"),
        }

        assert_eq!(
            try_donut(&[1.0], -1.0, 100.0).unwrap_err(),
            PieLayoutError::NegativeLayoutParameter {
                parameter: "inner_radius",
                value: -1.0,
            }
        );
        assert_eq!(
            try_half_pie(&[1.0], f64::INFINITY).unwrap_err(),
            PieLayoutError::NonFiniteLayoutParameter {
                parameter: "outer_radius",
                value: f64::INFINITY,
            }
        );
    }
}
