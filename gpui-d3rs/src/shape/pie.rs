//! Pie layout generator
//!
//! Computes the arc angles for pie and donut charts from data.

use std::f64::consts::PI;

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

        // Calculate the available angle range
        let range = self.end_angle - self.start_angle;
        let n = entries.len();
        let total_padding = self.pad_angle * n as f64;
        let available_range = (range - total_padding).max(0.0);

        // Generate slices
        let mut current_angle = self.start_angle;
        let mut slices = Vec::with_capacity(n);

        for (index, data, value) in entries {
            let slice_angle = if value > 0.0 {
                available_range * value / total
            } else {
                0.0
            };

            let start = current_angle;
            let end = current_angle + slice_angle;

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

            current_angle = end + self.pad_angle;
        }

        slices
    }
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

        // With padding, slices should be slightly smaller
        let slice_angle = slices[0].arc.end_angle - slices[0].arc.start_angle;
        assert!(slice_angle < PI); // Less than half without padding
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
}
