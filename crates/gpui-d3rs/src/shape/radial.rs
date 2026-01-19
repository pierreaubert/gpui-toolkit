//! Radial shape generators
//!
//! These generators create shapes in polar coordinates, useful for
//! radar charts, polar area charts, and circular visualizations.

use super::curve::Curve;
use super::path::PathBuilder;

/// A point in polar coordinates
#[derive(Debug, Clone, Copy)]
pub struct RadialPoint {
    /// Angle in radians (0 = right, PI/2 = down)
    pub angle: f64,
    /// Distance from center
    pub radius: f64,
}

impl RadialPoint {
    /// Create a new radial point
    pub fn new(angle: f64, radius: f64) -> Self {
        Self { angle, radius }
    }

    /// Convert to Cartesian coordinates with given center
    pub fn to_cartesian(&self, cx: f64, cy: f64) -> (f64, f64) {
        (
            cx + self.radius * self.angle.cos(),
            cy + self.radius * self.angle.sin(),
        )
    }

    /// Create from Cartesian coordinates
    pub fn from_cartesian(x: f64, y: f64, cx: f64, cy: f64) -> Self {
        let dx = x - cx;
        let dy = y - cy;
        Self {
            angle: dy.atan2(dx),
            radius: (dx * dx + dy * dy).sqrt(),
        }
    }
}

/// Configuration for radial line generator
#[derive(Debug, Clone)]
pub struct RadialLineConfig {
    /// Center X coordinate
    pub cx: f64,
    /// Center Y coordinate
    pub cy: f64,
    /// Curve type for interpolation
    pub curve: Curve,
    /// Whether to close the path
    pub closed: bool,
}

impl Default for RadialLineConfig {
    fn default() -> Self {
        Self {
            cx: 0.0,
            cy: 0.0,
            curve: Curve::Linear,
            closed: false,
        }
    }
}

impl RadialLineConfig {
    /// Create a new config with given center
    pub fn new(cx: f64, cy: f64) -> Self {
        Self {
            cx,
            cy,
            ..Default::default()
        }
    }

    /// Set the curve type
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Set whether to close the path
    pub fn closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
    }
}

/// Generate a radial line path
///
/// Connects points in polar coordinates with the specified curve type.
///
/// # Example
///
/// ```
/// use d3rs::shape::{RadialPoint, RadialLineConfig, radial_line};
/// use std::f64::consts::PI;
///
/// let points = vec![
///     RadialPoint::new(0.0, 100.0),
///     RadialPoint::new(PI / 2.0, 80.0),
///     RadialPoint::new(PI, 100.0),
///     RadialPoint::new(3.0 * PI / 2.0, 80.0),
/// ];
///
/// let config = RadialLineConfig::new(200.0, 200.0).closed(true);
/// let path = radial_line(&points, &config);
/// ```
pub fn radial_line(points: &[RadialPoint], config: &RadialLineConfig) -> String {
    if points.is_empty() {
        return String::new();
    }

    let cartesian: Vec<(f64, f64)> = points
        .iter()
        .map(|p| p.to_cartesian(config.cx, config.cy))
        .collect();

    let mut builder = PathBuilder::new();

    // For now, use linear interpolation (can be enhanced with curve support)
    if let Some(&(x, y)) = cartesian.first() {
        builder = builder.move_to(x, y);
    }

    for &(x, y) in cartesian.iter().skip(1) {
        builder = builder.line_to(x, y);
    }

    if config.closed {
        builder = builder.close_path();
    }

    builder.build().to_svg_string()
}

/// Configuration for radial area generator
#[derive(Debug, Clone)]
pub struct RadialAreaConfig {
    /// Center X coordinate
    pub cx: f64,
    /// Center Y coordinate
    pub cy: f64,
    /// Inner radius (can be constant or per-point)
    pub inner_radius: f64,
    /// Curve type for interpolation
    pub curve: Curve,
}

impl Default for RadialAreaConfig {
    fn default() -> Self {
        Self {
            cx: 0.0,
            cy: 0.0,
            inner_radius: 0.0,
            curve: Curve::Linear,
        }
    }
}

impl RadialAreaConfig {
    /// Create a new config with given center
    pub fn new(cx: f64, cy: f64) -> Self {
        Self {
            cx,
            cy,
            ..Default::default()
        }
    }

    /// Set the inner radius
    pub fn inner_radius(mut self, r: f64) -> Self {
        self.inner_radius = r;
        self
    }

    /// Set the curve type
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }
}

/// Generate a radial area path
///
/// Creates a filled area between the inner radius and the points.
///
/// # Example
///
/// ```
/// use d3rs::shape::{RadialPoint, RadialAreaConfig, radial_area};
/// use std::f64::consts::PI;
///
/// let points = vec![
///     RadialPoint::new(0.0, 100.0),
///     RadialPoint::new(PI / 2.0, 80.0),
///     RadialPoint::new(PI, 100.0),
///     RadialPoint::new(3.0 * PI / 2.0, 80.0),
/// ];
///
/// let config = RadialAreaConfig::new(200.0, 200.0).inner_radius(50.0);
/// let path = radial_area(&points, &config);
/// ```
pub fn radial_area(points: &[RadialPoint], config: &RadialAreaConfig) -> String {
    if points.is_empty() {
        return String::new();
    }

    // Outer path (clockwise)
    let outer: Vec<(f64, f64)> = points
        .iter()
        .map(|p| p.to_cartesian(config.cx, config.cy))
        .collect();

    // Inner path (counter-clockwise)
    let inner: Vec<(f64, f64)> = points
        .iter()
        .map(|p| RadialPoint::new(p.angle, config.inner_radius).to_cartesian(config.cx, config.cy))
        .collect();

    let mut builder = PathBuilder::new();

    // Draw outer path
    if let Some(&(x, y)) = outer.first() {
        builder = builder.move_to(x, y);
    }
    for &(x, y) in outer.iter().skip(1) {
        builder = builder.line_to(x, y);
    }

    // Draw inner path in reverse
    for &(x, y) in inner.iter().rev() {
        builder = builder.line_to(x, y);
    }

    builder.close_path().build().to_svg_string()
}

/// Generate a polar grid of concentric circles
pub fn polar_grid_circles(cx: f64, cy: f64, radii: &[f64]) -> Vec<String> {
    radii
        .iter()
        .map(|&r| {
            PathBuilder::new()
                .arc(cx, cy, r, 0.0, std::f64::consts::TAU, false)
                .build()
                .to_svg_string()
        })
        .collect()
}

/// Generate polar grid radial lines
pub fn polar_grid_rays(
    cx: f64,
    cy: f64,
    outer_radius: f64,
    angles: &[f64],
    inner_radius: f64,
) -> Vec<String> {
    angles
        .iter()
        .map(|&angle| {
            let inner_x = cx + inner_radius * angle.cos();
            let inner_y = cy + inner_radius * angle.sin();
            let outer_x = cx + outer_radius * angle.cos();
            let outer_y = cy + outer_radius * angle.sin();
            PathBuilder::new()
                .move_to(inner_x, inner_y)
                .line_to(outer_x, outer_y)
                .build()
                .to_svg_string()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_radial_point_to_cartesian() {
        let p = RadialPoint::new(0.0, 100.0);
        let (x, y) = p.to_cartesian(200.0, 200.0);
        assert!((x - 300.0).abs() < 1e-6);
        assert!((y - 200.0).abs() < 1e-6);
    }

    #[test]
    fn test_radial_point_from_cartesian() {
        let p = RadialPoint::from_cartesian(300.0, 200.0, 200.0, 200.0);
        assert!((p.angle - 0.0).abs() < 1e-6);
        assert!((p.radius - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_radial_line() {
        let points = vec![
            RadialPoint::new(0.0, 100.0),
            RadialPoint::new(PI / 2.0, 100.0),
            RadialPoint::new(PI, 100.0),
        ];
        let config = RadialLineConfig::new(200.0, 200.0);
        let path = radial_line(&points, &config);
        assert!(path.starts_with("M"));
        assert_eq!(path.matches('L').count(), 2);
    }

    #[test]
    fn test_radial_line_closed() {
        let points = vec![
            RadialPoint::new(0.0, 100.0),
            RadialPoint::new(PI / 2.0, 100.0),
            RadialPoint::new(PI, 100.0),
        ];
        let config = RadialLineConfig::new(200.0, 200.0).closed(true);
        let path = radial_line(&points, &config);
        assert!(path.ends_with("Z"));
    }

    #[test]
    fn test_radial_area() {
        let points = vec![
            RadialPoint::new(0.0, 100.0),
            RadialPoint::new(PI / 2.0, 80.0),
            RadialPoint::new(PI, 100.0),
            RadialPoint::new(3.0 * PI / 2.0, 80.0),
        ];
        let config = RadialAreaConfig::new(200.0, 200.0).inner_radius(50.0);
        let path = radial_area(&points, &config);
        assert!(path.starts_with("M"));
        assert!(path.ends_with("Z"));
    }

    #[test]
    fn test_polar_grid_circles() {
        let circles = polar_grid_circles(200.0, 200.0, &[50.0, 100.0, 150.0]);
        assert_eq!(circles.len(), 3);
        for circle in &circles {
            assert!(circle.contains("A")); // Arc command
        }
    }

    #[test]
    fn test_polar_grid_rays() {
        let rays = polar_grid_rays(200.0, 200.0, 100.0, &[0.0, PI / 2.0, PI], 0.0);
        assert_eq!(rays.len(), 3);
        for ray in &rays {
            assert!(ray.starts_with("M"));
            assert!(ray.contains("L"));
        }
    }
}
