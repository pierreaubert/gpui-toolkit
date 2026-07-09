//! Arc generator
//!
//! Generates arc shapes for pie and donut charts.

use std::f64::consts::PI;
use std::fmt;

use super::path::{Path, PathBuilder, Point};
use crate::util::scratch::path_to_string;

/// Recoverable errors for checked arc path input validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ArcGenerationError {
    /// Arc parameters and center coordinates must be finite.
    NonFiniteParameter { parameter: &'static str, value: f64 },
    /// Checked radii and padding must be zero or positive.
    NegativeParameter { parameter: &'static str, value: f64 },
    /// Checked arcs require `inner_radius <= outer_radius`.
    InnerRadiusExceedsOuterRadius {
        inner_radius: f64,
        outer_radius: f64,
    },
    /// Checked point sampling requires at least one segment.
    ZeroSegments,
}

impl fmt::Display for ArcGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter { parameter, value } => {
                write!(f, "arc parameter {parameter} is not finite: {value}")
            }
            Self::NegativeParameter { parameter, value } => {
                write!(f, "arc parameter {parameter} is negative: {value}")
            }
            Self::InnerRadiusExceedsOuterRadius {
                inner_radius,
                outer_radius,
            } => write!(
                f,
                "arc inner_radius {inner_radius} exceeds outer_radius {outer_radius}"
            ),
            Self::ZeroSegments => write!(f, "arc point sampling requires at least one segment"),
        }
    }
}

impl std::error::Error for ArcGenerationError {}

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

    /// Generate an arc path from the given datum after validating geometry.
    pub fn try_generate(&self, datum: &ArcDatum) -> Result<Path, ArcGenerationError> {
        validate_arc_geometry(datum, self.center_x, self.center_y)?;
        Ok(self.generate(datum))
    }

    /// Generate an arc and return the SVG path string.
    pub fn path_string(&self, datum: &ArcDatum) -> String {
        path_to_string(&self.generate(datum))
    }

    /// Generate a checked arc and return the SVG path string.
    pub fn try_path_string(&self, datum: &ArcDatum) -> Result<String, ArcGenerationError> {
        Ok(path_to_string(&self.try_generate(datum)?))
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

/// Generate checked points along an arc for rendering.
pub fn try_arc_points(
    datum: &ArcDatum,
    segments: usize,
    cx: f64,
    cy: f64,
) -> Result<Vec<Point>, ArcGenerationError> {
    if segments == 0 {
        return Err(ArcGenerationError::ZeroSegments);
    }

    validate_arc_geometry(datum, cx, cy)?;
    Ok(arc_points(datum, segments, cx, cy))
}

fn validate_arc_geometry(
    datum: &ArcDatum,
    center_x: f64,
    center_y: f64,
) -> Result<(), ArcGenerationError> {
    validate_arc_parameter("center_x", center_x, false)?;
    validate_arc_parameter("center_y", center_y, false)?;
    validate_arc_parameter("inner_radius", datum.inner_radius, true)?;
    validate_arc_parameter("outer_radius", datum.outer_radius, true)?;
    validate_arc_parameter("start_angle", datum.start_angle, false)?;
    validate_arc_parameter("end_angle", datum.end_angle, false)?;
    validate_arc_parameter("corner_radius", datum.corner_radius, true)?;
    validate_arc_parameter("pad_angle", datum.pad_angle, true)?;

    if datum.inner_radius > datum.outer_radius {
        return Err(ArcGenerationError::InnerRadiusExceedsOuterRadius {
            inner_radius: datum.inner_radius,
            outer_radius: datum.outer_radius,
        });
    }

    Ok(())
}

fn validate_arc_parameter(
    parameter: &'static str,
    value: f64,
    non_negative: bool,
) -> Result<(), ArcGenerationError> {
    if !value.is_finite() {
        return Err(ArcGenerationError::NonFiniteParameter { parameter, value });
    }
    if non_negative && value < 0.0 {
        return Err(ArcGenerationError::NegativeParameter { parameter, value });
    }

    Ok(())
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

    #[test]
    fn try_generate_matches_generate_for_valid_arc() {
        let arc = Arc::new().center(10.0, 20.0);
        let datum = ArcDatum::new()
            .inner_radius(50.0)
            .outer_radius(100.0)
            .start_angle(0.0)
            .end_angle(PI);

        let permissive = arc.generate(&datum);
        let checked = arc.try_generate(&datum).unwrap();

        assert_eq!(permissive.commands(), checked.commands());
        assert_eq!(
            arc.path_string(&datum),
            arc.try_path_string(&datum).unwrap()
        );
    }

    #[test]
    fn try_generate_rejects_invalid_arc_geometry() {
        let arc = Arc::new().center(f64::NAN, 0.0);
        let error = arc.try_generate(&ArcDatum::new()).unwrap_err();
        match error {
            ArcGenerationError::NonFiniteParameter { parameter, value } => {
                assert_eq!(parameter, "center_x");
                assert!(value.is_nan());
            }
            _ => panic!("unexpected error: {error:?}"),
        }

        let datum = ArcDatum::new().inner_radius(-1.0);
        assert_eq!(
            Arc::new().try_generate(&datum).unwrap_err(),
            ArcGenerationError::NegativeParameter {
                parameter: "inner_radius",
                value: -1.0,
            }
        );

        let datum = ArcDatum::new().inner_radius(100.0).outer_radius(50.0);
        assert_eq!(
            Arc::new().try_generate(&datum).unwrap_err(),
            ArcGenerationError::InnerRadiusExceedsOuterRadius {
                inner_radius: 100.0,
                outer_radius: 50.0,
            }
        );
    }

    #[test]
    fn try_arc_points_rejects_invalid_sampling_inputs() {
        let datum = ArcDatum::new();
        assert_eq!(
            try_arc_points(&datum, 0, 0.0, 0.0).unwrap_err(),
            ArcGenerationError::ZeroSegments
        );

        let datum = ArcDatum::new().outer_radius(f64::INFINITY);
        assert_eq!(
            try_arc_points(&datum, 4, 0.0, 0.0).unwrap_err(),
            ArcGenerationError::NonFiniteParameter {
                parameter: "outer_radius",
                value: f64::INFINITY,
            }
        );
    }
}
