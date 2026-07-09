//! Radial shape generators
//!
//! These generators create shapes in polar coordinates, useful for
//! radar charts, polar area charts, and circular visualizations.

use std::fmt;

use super::curve::Curve;
use super::path::PathBuilder;
use crate::util::scratch::path_to_string;

/// Recoverable errors for checked radial shape generation.
#[derive(Debug, Clone, PartialEq)]
pub enum RadialGenerationError {
    /// Named centers, angles, and radii must be finite.
    NonFiniteParameter { parameter: &'static str, value: f64 },
    /// Named checked radial radii must be zero or positive.
    NegativeRadius { parameter: &'static str, value: f64 },
    /// Radial point fields must be finite.
    NonFinitePoint {
        index: usize,
        field: RadialPointField,
        value: f64,
    },
    /// Radial point radii must be zero or positive.
    NegativePointRadius { index: usize, value: f64 },
    /// Grid circle radii must be finite.
    NonFiniteGridRadius { index: usize, value: f64 },
    /// Grid circle radii must be zero or positive.
    NegativeGridRadius { index: usize, value: f64 },
    /// Grid ray angles must be finite.
    NonFiniteGridAngle { index: usize, value: f64 },
}

/// Field name for [`RadialPoint`] validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadialPointField {
    Angle,
    Radius,
}

impl RadialPointField {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Angle => "angle",
            Self::Radius => "radius",
        }
    }
}

impl fmt::Display for RadialGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter { parameter, value } => {
                write!(f, "radial parameter {parameter} is not finite: {value}")
            }
            Self::NegativeRadius { parameter, value } => {
                write!(f, "radial radius {parameter} is negative: {value}")
            }
            Self::NonFinitePoint {
                index,
                field,
                value,
            } => {
                write!(
                    f,
                    "radial point {index} {} is not finite: {value}",
                    field.as_str()
                )
            }
            Self::NegativePointRadius { index, value } => {
                write!(f, "radial point {index} radius is negative: {value}")
            }
            Self::NonFiniteGridRadius { index, value } => {
                write!(f, "polar grid radius {index} is not finite: {value}")
            }
            Self::NegativeGridRadius { index, value } => {
                write!(f, "polar grid radius {index} is negative: {value}")
            }
            Self::NonFiniteGridAngle { index, value } => {
                write!(f, "polar grid angle {index} is not finite: {value}")
            }
        }
    }
}

impl std::error::Error for RadialGenerationError {}

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

    /// Convert to Cartesian coordinates after validating the point and center.
    pub fn try_to_cartesian(&self, cx: f64, cy: f64) -> Result<(f64, f64), RadialGenerationError> {
        validate_radial_center(cx, cy)?;
        validate_radial_point(0, self)?;
        Ok(self.to_cartesian(cx, cy))
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

    path_to_string(&builder.build())
}

/// Checked radial line generator.
pub fn try_radial_line(
    points: &[RadialPoint],
    config: &RadialLineConfig,
) -> Result<String, RadialGenerationError> {
    validate_radial_line(points, config)?;
    Ok(radial_line(points, config))
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

    path_to_string(&builder.close_path().build())
}

/// Checked radial area generator.
pub fn try_radial_area(
    points: &[RadialPoint],
    config: &RadialAreaConfig,
) -> Result<String, RadialGenerationError> {
    validate_radial_area(points, config)?;
    Ok(radial_area(points, config))
}

/// Generate a polar grid of concentric circles
pub fn polar_grid_circles(cx: f64, cy: f64, radii: &[f64]) -> Vec<String> {
    radii
        .iter()
        .map(|&r| {
            path_to_string(
                &PathBuilder::new()
                    .arc(cx, cy, r, 0.0, std::f64::consts::TAU, false)
                    .build(),
            )
        })
        .collect()
}

/// Checked polar grid circle generator.
pub fn try_polar_grid_circles(
    cx: f64,
    cy: f64,
    radii: &[f64],
) -> Result<Vec<String>, RadialGenerationError> {
    validate_radial_center(cx, cy)?;
    for (index, &radius) in radii.iter().enumerate() {
        validate_grid_radius(index, radius)?;
    }
    Ok(polar_grid_circles(cx, cy, radii))
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
            path_to_string(
                &PathBuilder::new()
                    .move_to(inner_x, inner_y)
                    .line_to(outer_x, outer_y)
                    .build(),
            )
        })
        .collect()
}

/// Checked polar grid radial line generator.
pub fn try_polar_grid_rays(
    cx: f64,
    cy: f64,
    outer_radius: f64,
    angles: &[f64],
    inner_radius: f64,
) -> Result<Vec<String>, RadialGenerationError> {
    validate_radial_center(cx, cy)?;
    validate_radius("outer_radius", outer_radius)?;
    validate_radius("inner_radius", inner_radius)?;
    for (index, &angle) in angles.iter().enumerate() {
        validate_grid_angle(index, angle)?;
    }
    Ok(polar_grid_rays(cx, cy, outer_radius, angles, inner_radius))
}

fn validate_radial_line(
    points: &[RadialPoint],
    config: &RadialLineConfig,
) -> Result<(), RadialGenerationError> {
    validate_radial_center(config.cx, config.cy)?;
    for (index, point) in points.iter().enumerate() {
        validate_radial_point(index, point)?;
    }
    Ok(())
}

fn validate_radial_area(
    points: &[RadialPoint],
    config: &RadialAreaConfig,
) -> Result<(), RadialGenerationError> {
    validate_radial_center(config.cx, config.cy)?;
    validate_radius("inner_radius", config.inner_radius)?;
    for (index, point) in points.iter().enumerate() {
        validate_radial_point(index, point)?;
    }
    Ok(())
}

fn validate_radial_center(cx: f64, cy: f64) -> Result<(), RadialGenerationError> {
    validate_finite("cx", cx)?;
    validate_finite("cy", cy)
}

fn validate_radial_point(index: usize, point: &RadialPoint) -> Result<(), RadialGenerationError> {
    if !point.angle.is_finite() {
        return Err(RadialGenerationError::NonFinitePoint {
            index,
            field: RadialPointField::Angle,
            value: point.angle,
        });
    }
    if !point.radius.is_finite() {
        return Err(RadialGenerationError::NonFinitePoint {
            index,
            field: RadialPointField::Radius,
            value: point.radius,
        });
    }
    if point.radius < 0.0 {
        return Err(RadialGenerationError::NegativePointRadius {
            index,
            value: point.radius,
        });
    }
    Ok(())
}

fn validate_finite(parameter: &'static str, value: f64) -> Result<(), RadialGenerationError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(RadialGenerationError::NonFiniteParameter { parameter, value })
    }
}

fn validate_radius(parameter: &'static str, value: f64) -> Result<(), RadialGenerationError> {
    validate_finite(parameter, value)?;
    if value < 0.0 {
        Err(RadialGenerationError::NegativeRadius { parameter, value })
    } else {
        Ok(())
    }
}

fn validate_grid_radius(index: usize, radius: f64) -> Result<(), RadialGenerationError> {
    if !radius.is_finite() {
        return Err(RadialGenerationError::NonFiniteGridRadius {
            index,
            value: radius,
        });
    }
    if radius < 0.0 {
        return Err(RadialGenerationError::NegativeGridRadius {
            index,
            value: radius,
        });
    }
    Ok(())
}

fn validate_grid_angle(index: usize, angle: f64) -> Result<(), RadialGenerationError> {
    if angle.is_finite() {
        Ok(())
    } else {
        Err(RadialGenerationError::NonFiniteGridAngle {
            index,
            value: angle,
        })
    }
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

    #[test]
    fn checked_radial_generators_match_permissive_generators() {
        let points = vec![
            RadialPoint::new(0.0, 100.0),
            RadialPoint::new(PI / 2.0, 80.0),
            RadialPoint::new(PI, 100.0),
            RadialPoint::new(3.0 * PI / 2.0, 80.0),
        ];

        let line_config = RadialLineConfig::new(200.0, 200.0).closed(true);
        assert_eq!(
            radial_line(&points, &line_config),
            try_radial_line(&points, &line_config).unwrap()
        );

        let area_config = RadialAreaConfig::new(200.0, 200.0).inner_radius(50.0);
        assert_eq!(
            radial_area(&points, &area_config),
            try_radial_area(&points, &area_config).unwrap()
        );

        assert_eq!(
            points[0].to_cartesian(200.0, 200.0),
            points[0].try_to_cartesian(200.0, 200.0).unwrap()
        );
        assert_eq!(
            polar_grid_circles(200.0, 200.0, &[50.0, 100.0]),
            try_polar_grid_circles(200.0, 200.0, &[50.0, 100.0]).unwrap()
        );
        assert_eq!(
            polar_grid_rays(200.0, 200.0, 100.0, &[0.0, PI], 0.0),
            try_polar_grid_rays(200.0, 200.0, 100.0, &[0.0, PI], 0.0).unwrap()
        );
    }

    #[test]
    fn checked_radial_generators_reject_non_finite_values() {
        let points = vec![
            RadialPoint::new(0.0, 100.0),
            RadialPoint::new(f64::NAN, 80.0),
        ];
        let error = try_radial_line(&points, &RadialLineConfig::new(200.0, 200.0)).unwrap_err();
        match error {
            RadialGenerationError::NonFinitePoint {
                index,
                field,
                value,
            } => {
                assert_eq!(index, 1);
                assert_eq!(field, RadialPointField::Angle);
                assert!(value.is_nan());
            }
            error => panic!("unexpected error: {error:?}"),
        }

        assert_eq!(
            try_radial_area(
                &[RadialPoint::new(0.0, 100.0)],
                &RadialAreaConfig::new(f64::INFINITY, 200.0)
            )
            .unwrap_err(),
            RadialGenerationError::NonFiniteParameter {
                parameter: "cx",
                value: f64::INFINITY,
            }
        );

        assert_eq!(
            try_polar_grid_rays(200.0, 200.0, 100.0, &[0.0, f64::INFINITY], 0.0).unwrap_err(),
            RadialGenerationError::NonFiniteGridAngle {
                index: 1,
                value: f64::INFINITY,
            }
        );
    }

    #[test]
    fn checked_radial_generators_reject_negative_radii() {
        assert_eq!(
            try_radial_line(
                &[RadialPoint::new(0.0, -1.0)],
                &RadialLineConfig::new(200.0, 200.0)
            )
            .unwrap_err(),
            RadialGenerationError::NegativePointRadius {
                index: 0,
                value: -1.0,
            }
        );

        assert_eq!(
            try_radial_area(
                &[RadialPoint::new(0.0, 100.0)],
                &RadialAreaConfig::new(200.0, 200.0).inner_radius(-1.0)
            )
            .unwrap_err(),
            RadialGenerationError::NegativeRadius {
                parameter: "inner_radius",
                value: -1.0,
            }
        );

        assert_eq!(
            try_polar_grid_circles(200.0, 200.0, &[50.0, -1.0]).unwrap_err(),
            RadialGenerationError::NegativeGridRadius {
                index: 1,
                value: -1.0,
            }
        );

        assert_eq!(
            try_polar_grid_rays(200.0, 200.0, -1.0, &[0.0], 0.0).unwrap_err(),
            RadialGenerationError::NegativeRadius {
                parameter: "outer_radius",
                value: -1.0,
            }
        );
    }
}
