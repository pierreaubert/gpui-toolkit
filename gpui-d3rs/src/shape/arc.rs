//! Arc generator
//!
//! Generates arc shapes for pie and donut charts.

use std::f64::consts::PI;

use super::path::{Path, PathBuilder, Point};

/// Arc datum containing the angles and radii for an arc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcDatum {
    /// Inner radius of the arc
    pub inner_radius: f64,
    /// Outer radius of the arc
    pub outer_radius: f64,
    /// Start angle in radians (0 = 12 o'clock, clockwise)
    pub start_angle: f64,
    /// End angle in radians
    pub end_angle: f64,
    /// Corner radius for rounded corners
    pub corner_radius: f64,
    /// Padding angle in radians
    pub pad_angle: f64,
}

impl Default for ArcDatum {
    fn default() -> Self {
        Self {
            inner_radius: 0.0,
            outer_radius: 100.0,
            start_angle: 0.0,
            end_angle: PI * 2.0,
            corner_radius: 0.0,
            pad_angle: 0.0,
        }
    }
}

impl ArcDatum {
    /// Create a new arc datum.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the inner radius.
    pub fn inner_radius(mut self, r: f64) -> Self {
        self.inner_radius = r;
        self
    }

    /// Set the outer radius.
    pub fn outer_radius(mut self, r: f64) -> Self {
        self.outer_radius = r;
        self
    }

    /// Set the start angle (in radians).
    pub fn start_angle(mut self, a: f64) -> Self {
        self.start_angle = a;
        self
    }

    /// Set the end angle (in radians).
    pub fn end_angle(mut self, a: f64) -> Self {
        self.end_angle = a;
        self
    }

    /// Set the corner radius.
    pub fn corner_radius(mut self, r: f64) -> Self {
        self.corner_radius = r;
        self
    }

    /// Set the padding angle.
    pub fn pad_angle(mut self, a: f64) -> Self {
        self.pad_angle = a;
        self
    }

    /// Get the centroid of the arc.
    ///
    /// Returns the point at the center of the arc, useful for label positioning.
    pub fn centroid(&self) -> Point {
        let r = (self.inner_radius + self.outer_radius) / 2.0;
        let a = (self.start_angle + self.end_angle) / 2.0 - PI / 2.0;
        Point::new(r * a.cos(), r * a.sin())
    }
}

/// Arc generator for creating arc paths.
///
/// # Example
///
/// ```
/// use d3rs::shape::arc::{Arc, ArcDatum};
/// use std::f64::consts::PI;
///
/// let arc = Arc::new();
/// let datum = ArcDatum::new()
///     .inner_radius(50.0)
///     .outer_radius(100.0)
///     .start_angle(0.0)
///     .end_angle(PI);
///
/// let path = arc.generate(&datum);
/// assert!(!path.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct Arc {
    /// Center X offset
    center_x: f64,
    /// Center Y offset
    center_y: f64,
}

impl Default for Arc {
    fn default() -> Self {
        Self::new()
    }
}

impl Arc {
    /// Create a new arc generator.
    pub fn new() -> Self {
        Self {
            center_x: 0.0,
            center_y: 0.0,
        }
    }

    /// Set the center offset.
    pub fn center(mut self, x: f64, y: f64) -> Self {
        self.center_x = x;
        self.center_y = y;
        self
    }

    /// Generate an arc path from the given datum.
    pub fn generate(&self, datum: &ArcDatum) -> Path {
        let mut builder = PathBuilder::new();

        let inner = datum.inner_radius;
        let outer = datum.outer_radius;
        let mut start = datum.start_angle - PI / 2.0; // Convert to math coordinates
        let mut end = datum.end_angle - PI / 2.0;

        // Apply padding
        if datum.pad_angle > 0.0 && inner > 0.0 {
            let pad = datum.pad_angle / 2.0;
            start += pad;
            end -= pad;
        }

        let cx = self.center_x;
        let cy = self.center_y;

        // Check for full circle
        let delta = (end - start).abs();
        let full_circle = delta >= 2.0 * PI - 1e-6;

        if full_circle {
            // Full circle/ring
            if inner > 0.0 {
                // Donut
                builder = builder
                    .move_to(cx + outer, cy)
                    .arc(cx, cy, outer, 0.0, PI, false)
                    .arc(cx, cy, outer, PI, 2.0 * PI, false)
                    .move_to(cx + inner, cy)
                    .arc(cx, cy, inner, 0.0, PI, true)
                    .arc(cx, cy, inner, PI, 2.0 * PI, true)
                    .close_path();
            } else {
                // Full pie
                builder = builder
                    .move_to(cx + outer, cy)
                    .arc(cx, cy, outer, 0.0, PI, false)
                    .arc(cx, cy, outer, PI, 2.0 * PI, false)
                    .close_path();
            }
        } else {
            // Arc segment
            let outer_start = Point::new(cx + outer * start.cos(), cy + outer * start.sin());
            if inner > 0.0 {
                // Arc with inner radius (donut slice)
                let inner_end = Point::new(cx + inner * end.cos(), cy + inner * end.sin());

                builder = builder
                    .move_to(outer_start.x, outer_start.y)
                    .arc(cx, cy, outer, start, end, false)
                    .line_to(inner_end.x, inner_end.y)
                    .arc(cx, cy, inner, end, start, true)
                    .close_path();
            } else {
                // Pie slice (from center)
                builder = builder
                    .move_to(cx, cy)
                    .line_to(outer_start.x, outer_start.y)
                    .arc(cx, cy, outer, start, end, false)
                    .close_path();
            }
        }

        builder.build()
    }

    /// Generate an arc and return the SVG path string.
    pub fn path_string(&self, datum: &ArcDatum) -> String {
        self.generate(datum).to_svg_string()
    }
}

/// Generate points along an arc for rendering.
///
/// # Arguments
///
/// * `datum` - The arc datum
/// * `segments` - Number of line segments to use
/// * `cx` - Center X
/// * `cy` - Center Y
pub fn arc_points(datum: &ArcDatum, segments: usize, cx: f64, cy: f64) -> Vec<Point> {
    let mut points = Vec::with_capacity(segments * 2 + 4);

    let inner = datum.inner_radius;
    let outer = datum.outer_radius;
    let start = datum.start_angle - PI / 2.0;
    let end = datum.end_angle - PI / 2.0;
    let delta = end - start;

    // Outer arc points
    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let angle = start + delta * t;
        points.push(Point::new(
            cx + outer * angle.cos(),
            cy + outer * angle.sin(),
        ));
    }

    if inner > 0.0 {
        // Inner arc points (reverse order)
        for i in (0..=segments).rev() {
            let t = i as f64 / segments as f64;
            let angle = start + delta * t;
            points.push(Point::new(
                cx + inner * angle.cos(),
                cy + inner * angle.sin(),
            ));
        }
    } else {
        // Single center point for pie slice
        points.push(Point::new(cx, cy));
    }

    // Close the shape
    if !points.is_empty() {
        points.push(points[0]);
    }

    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_datum() {
        let datum = ArcDatum::new()
            .inner_radius(50.0)
            .outer_radius(100.0)
            .start_angle(0.0)
            .end_angle(PI);

        assert_eq!(datum.inner_radius, 50.0);
        assert_eq!(datum.outer_radius, 100.0);
    }

    #[test]
    fn test_arc_centroid() {
        let datum = ArcDatum::new()
            .inner_radius(0.0)
            .outer_radius(100.0)
            .start_angle(0.0)
            .end_angle(PI / 2.0);

        let centroid = datum.centroid();
        // With 0 = 12 o'clock and clockwise rotation:
        // Angle range 0 to PI/2 means right side of clock (12 to 3 o'clock)
        // Average angle = PI/4 - PI/2 = -PI/4 (to convert to standard math coords)
        // So centroid.x > 0, centroid.y < 0 (bottom-right quadrant in screen coords)
        assert!(centroid.x > 0.0);
        // Y is negative in this coordinate system
        assert!(centroid.y < 0.0);
    }

    #[test]
    fn test_arc_generator() {
        let arc = Arc::new();
        let datum = ArcDatum::new()
            .inner_radius(50.0)
            .outer_radius(100.0)
            .start_angle(0.0)
            .end_angle(PI);

        let path = arc.generate(&datum);
        assert!(!path.is_empty());
    }

    #[test]
    fn test_arc_points() {
        let datum = ArcDatum::new()
            .inner_radius(0.0)
            .outer_radius(100.0)
            .start_angle(0.0)
            .end_angle(PI);

        let points = arc_points(&datum, 10, 0.0, 0.0);
        assert!(!points.is_empty());
    }

    #[test]
    fn test_full_circle_arc() {
        let arc = Arc::new();
        let datum = ArcDatum::new()
            .inner_radius(50.0)
            .outer_radius(100.0)
            .start_angle(0.0)
            .end_angle(2.0 * PI);

        let path = arc.generate(&datum);
        assert!(!path.is_empty());
    }
}
