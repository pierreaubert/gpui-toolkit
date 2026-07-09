//! Link shape generators
//!
//! Link shapes create smooth curves between two points, useful for
//! network visualizations and tree diagrams.

use std::fmt;

use super::path::PathBuilder;
use crate::util::scratch::path_to_string;

/// Recoverable errors for checked link path generation.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkGenerationError {
    /// Link coordinates and radial angles must be finite.
    NonFiniteParameter { parameter: &'static str, value: f64 },
    /// Checked radial radii must be zero or positive.
    NegativeRadius { parameter: &'static str, value: f64 },
}

impl fmt::Display for LinkGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter { parameter, value } => {
                write!(f, "link parameter {parameter} is not finite: {value}")
            }
            Self::NegativeRadius { parameter, value } => {
                write!(f, "link radius {parameter} is negative: {value}")
            }
        }
    }
}

impl std::error::Error for LinkGenerationError {}

/// Link direction/orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkDirection {
    /// Horizontal links (left to right)
    Horizontal,
    /// Vertical links (top to bottom)
    Vertical,
    /// Radial links (center outward)
    Radial,
}

/// A link connects a source point to a target point with a smooth curve
#[derive(Debug, Clone, Copy)]
pub struct Link {
    pub source_x: f64,
    pub source_y: f64,
    pub target_x: f64,
    pub target_y: f64,
}

impl Link {
    /// Create a new link between two points
    pub fn new(source_x: f64, source_y: f64, target_x: f64, target_y: f64) -> Self {
        Self {
            source_x,
            source_y,
            target_x,
            target_y,
        }
    }

    /// Create from (x, y) tuples
    pub fn from_points(source: (f64, f64), target: (f64, f64)) -> Self {
        Self::new(source.0, source.1, target.0, target.1)
    }

    /// Validate that all link coordinates are finite.
    pub fn validate(&self) -> Result<(), LinkGenerationError> {
        validate_link_coordinate("source_x", self.source_x)?;
        validate_link_coordinate("source_y", self.source_y)?;
        validate_link_coordinate("target_x", self.target_x)?;
        validate_link_coordinate("target_y", self.target_y)?;
        Ok(())
    }
}

/// Generator for horizontal links (Bezier curves, horizontal emphasis)
///
/// Creates a cubic Bezier curve that starts horizontal and ends horizontal.
/// Useful for left-to-right tree layouts.
///
/// # Example
///
/// ```
/// use d3rs::shape::{Link, link_horizontal};
///
/// let link = Link::new(0.0, 50.0, 200.0, 150.0);
/// let path = link_horizontal(&link);
/// assert!(path.starts_with("M"));
/// ```
pub fn link_horizontal(link: &Link) -> String {
    let midx = (link.source_x + link.target_x) / 2.0;

    let path = PathBuilder::new()
        .move_to(link.source_x, link.source_y)
        .cubic_curve_to(
            midx,
            link.source_y,
            midx,
            link.target_y,
            link.target_x,
            link.target_y,
        )
        .build();
    path_to_string(&path)
}

/// Checked generator for horizontal links.
pub fn try_link_horizontal(link: &Link) -> Result<String, LinkGenerationError> {
    link.validate()?;
    Ok(link_horizontal(link))
}

/// Generator for vertical links (Bezier curves, vertical emphasis)
///
/// Creates a cubic Bezier curve that starts vertical and ends vertical.
/// Useful for top-to-bottom tree layouts.
///
/// # Example
///
/// ```
/// use d3rs::shape::{Link, link_vertical};
///
/// let link = Link::new(100.0, 0.0, 150.0, 200.0);
/// let path = link_vertical(&link);
/// assert!(path.starts_with("M"));
/// ```
pub fn link_vertical(link: &Link) -> String {
    let midy = (link.source_y + link.target_y) / 2.0;

    let path = PathBuilder::new()
        .move_to(link.source_x, link.source_y)
        .cubic_curve_to(
            link.source_x,
            midy,
            link.target_x,
            midy,
            link.target_x,
            link.target_y,
        )
        .build();
    path_to_string(&path)
}

/// Checked generator for vertical links.
pub fn try_link_vertical(link: &Link) -> Result<String, LinkGenerationError> {
    link.validate()?;
    Ok(link_vertical(link))
}

/// A radial link for polar coordinate connections
#[derive(Debug, Clone, Copy)]
pub struct RadialLink {
    pub source_angle: f64,
    pub source_radius: f64,
    pub target_angle: f64,
    pub target_radius: f64,
}

impl RadialLink {
    /// Create a new radial link
    pub fn new(
        source_angle: f64,
        source_radius: f64,
        target_angle: f64,
        target_radius: f64,
    ) -> Self {
        Self {
            source_angle,
            source_radius,
            target_angle,
            target_radius,
        }
    }

    /// Convert to Cartesian link with given center
    pub fn to_cartesian(&self, cx: f64, cy: f64) -> Link {
        Link {
            source_x: cx + self.source_radius * self.source_angle.cos(),
            source_y: cy + self.source_radius * self.source_angle.sin(),
            target_x: cx + self.target_radius * self.target_angle.cos(),
            target_y: cy + self.target_radius * self.target_angle.sin(),
        }
    }

    /// Convert to Cartesian link after validating radial parameters and center.
    pub fn try_to_cartesian(&self, cx: f64, cy: f64) -> Result<Link, LinkGenerationError> {
        validate_radial_link(self, cx, cy)?;
        Ok(self.to_cartesian(cx, cy))
    }
}

/// Generator for radial links (curved connections in polar space)
///
/// Creates a smooth curve connecting two points in polar coordinates.
/// Useful for radial tree layouts.
///
/// # Example
///
/// ```
/// use d3rs::shape::{RadialLink, link_radial};
/// use std::f64::consts::PI;
///
/// let link = RadialLink::new(0.0, 50.0, PI / 2.0, 100.0);
/// let path = link_radial(&link, 200.0, 200.0);
/// assert!(path.starts_with("M"));
/// ```
pub fn link_radial(link: &RadialLink, cx: f64, cy: f64) -> String {
    let source_x = cx + link.source_radius * link.source_angle.cos();
    let source_y = cy + link.source_radius * link.source_angle.sin();
    let target_x = cx + link.target_radius * link.target_angle.cos();
    let target_y = cy + link.target_radius * link.target_angle.sin();

    // Midpoint in polar coordinates
    let mid_angle = (link.source_angle + link.target_angle) / 2.0;
    let mid_radius = (link.source_radius + link.target_radius) / 2.0;

    let mid_x = cx + mid_radius * mid_angle.cos();
    let mid_y = cy + mid_radius * mid_angle.sin();

    let path = PathBuilder::new()
        .move_to(source_x, source_y)
        .quadratic_curve_to(mid_x, mid_y, target_x, target_y)
        .build();
    path_to_string(&path)
}

/// Checked generator for radial links.
pub fn try_link_radial(link: &RadialLink, cx: f64, cy: f64) -> Result<String, LinkGenerationError> {
    validate_radial_link(link, cx, cy)?;
    Ok(link_radial(link, cx, cy))
}

/// Create a step link (orthogonal connection)
///
/// Creates a path with right-angle corners, useful for flowcharts.
pub fn link_step(link: &Link, direction: LinkDirection) -> String {
    match direction {
        LinkDirection::Horizontal => {
            let midx = (link.source_x + link.target_x) / 2.0;
            let path = PathBuilder::new()
                .move_to(link.source_x, link.source_y)
                .line_to(midx, link.source_y)
                .line_to(midx, link.target_y)
                .line_to(link.target_x, link.target_y)
                .build();
            path_to_string(&path)
        }
        LinkDirection::Vertical => {
            let midy = (link.source_y + link.target_y) / 2.0;
            let path = PathBuilder::new()
                .move_to(link.source_x, link.source_y)
                .line_to(link.source_x, midy)
                .line_to(link.target_x, midy)
                .line_to(link.target_x, link.target_y)
                .build();
            path_to_string(&path)
        }
        LinkDirection::Radial => {
            // For radial, just use straight lines
            let path = PathBuilder::new()
                .move_to(link.source_x, link.source_y)
                .line_to(link.target_x, link.target_y)
                .build();
            path_to_string(&path)
        }
    }
}

/// Checked generator for step links.
pub fn try_link_step(link: &Link, direction: LinkDirection) -> Result<String, LinkGenerationError> {
    link.validate()?;
    Ok(link_step(link, direction))
}

fn validate_radial_link(link: &RadialLink, cx: f64, cy: f64) -> Result<(), LinkGenerationError> {
    validate_link_coordinate("cx", cx)?;
    validate_link_coordinate("cy", cy)?;
    validate_link_coordinate("source_angle", link.source_angle)?;
    validate_link_coordinate("target_angle", link.target_angle)?;
    validate_link_radius("source_radius", link.source_radius)?;
    validate_link_radius("target_radius", link.target_radius)?;
    Ok(())
}

fn validate_link_coordinate(
    parameter: &'static str,
    value: f64,
) -> Result<(), LinkGenerationError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(LinkGenerationError::NonFiniteParameter { parameter, value })
    }
}

fn validate_link_radius(parameter: &'static str, value: f64) -> Result<(), LinkGenerationError> {
    validate_link_coordinate(parameter, value)?;
    if value < 0.0 {
        Err(LinkGenerationError::NegativeRadius { parameter, value })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_link_horizontal() {
        let link = Link::new(0.0, 50.0, 200.0, 150.0);
        let path = link_horizontal(&link);
        assert!(path.starts_with("M0,50"));
        assert!(path.contains("C")); // Contains Bezier curve
    }

    #[test]
    fn test_link_vertical() {
        let link = Link::new(100.0, 0.0, 150.0, 200.0);
        let path = link_vertical(&link);
        assert!(path.starts_with("M100,0"));
        assert!(path.contains("C"));
    }

    #[test]
    fn test_link_radial() {
        let link = RadialLink::new(0.0, 50.0, PI / 2.0, 100.0);
        let path = link_radial(&link, 200.0, 200.0);
        assert!(path.starts_with("M")); // Starts at source
        assert!(path.contains("Q")); // Contains quadratic curve
    }

    #[test]
    fn test_link_step_horizontal() {
        let link = Link::new(0.0, 50.0, 200.0, 150.0);
        let path = link_step(&link, LinkDirection::Horizontal);
        assert!(path.starts_with("M0,50"));
        // Should have 3 line segments
        assert_eq!(path.matches('L').count(), 3);
    }

    #[test]
    fn test_link_step_vertical() {
        let link = Link::new(50.0, 0.0, 150.0, 200.0);
        let path = link_step(&link, LinkDirection::Vertical);
        assert!(path.starts_with("M50,0"));
        assert_eq!(path.matches('L').count(), 3);
    }

    #[test]
    fn test_radial_link_to_cartesian() {
        let link = RadialLink::new(0.0, 100.0, PI, 100.0);
        let cart = link.to_cartesian(200.0, 200.0);

        // At angle 0, point is at (200+100, 200) = (300, 200)
        assert!((cart.source_x - 300.0).abs() < 1e-6);
        assert!((cart.source_y - 200.0).abs() < 1e-6);

        // At angle PI, point is at (200-100, 200) = (100, 200)
        assert!((cart.target_x - 100.0).abs() < 1e-6);
        assert!((cart.target_y - 200.0).abs() < 1e-6);
    }

    #[test]
    fn checked_link_generators_match_permissive_generators() {
        let link = Link::new(0.0, 50.0, 200.0, 150.0);
        assert_eq!(link_horizontal(&link), try_link_horizontal(&link).unwrap());
        assert_eq!(link_vertical(&link), try_link_vertical(&link).unwrap());
        assert_eq!(
            link_step(&link, LinkDirection::Horizontal),
            try_link_step(&link, LinkDirection::Horizontal).unwrap()
        );

        let radial = RadialLink::new(0.0, 50.0, PI / 2.0, 100.0);
        assert_eq!(
            link_radial(&radial, 200.0, 200.0),
            try_link_radial(&radial, 200.0, 200.0).unwrap()
        );
        assert_eq!(
            radial.to_cartesian(200.0, 200.0).source_x,
            radial.try_to_cartesian(200.0, 200.0).unwrap().source_x
        );
    }

    #[test]
    fn checked_link_generators_reject_non_finite_coordinates() {
        let link = Link::new(0.0, f64::NAN, 200.0, 150.0);
        let error = try_link_horizontal(&link).unwrap_err();
        match error {
            LinkGenerationError::NonFiniteParameter { parameter, value } => {
                assert_eq!(parameter, "source_y");
                assert!(value.is_nan());
            }
            _ => panic!("unexpected error: {error:?}"),
        }

        assert_eq!(
            try_link_step(
                &Link::new(0.0, 50.0, f64::INFINITY, 150.0),
                LinkDirection::Vertical
            )
            .unwrap_err(),
            LinkGenerationError::NonFiniteParameter {
                parameter: "target_x",
                value: f64::INFINITY,
            }
        );
    }

    #[test]
    fn checked_radial_links_reject_invalid_polar_inputs() {
        let radial = RadialLink::new(0.0, -1.0, PI / 2.0, 100.0);
        assert_eq!(
            try_link_radial(&radial, 0.0, 0.0).unwrap_err(),
            LinkGenerationError::NegativeRadius {
                parameter: "source_radius",
                value: -1.0,
            }
        );

        let radial = RadialLink::new(f64::NAN, 50.0, PI / 2.0, 100.0);
        let error = radial.try_to_cartesian(0.0, 0.0).unwrap_err();
        match error {
            LinkGenerationError::NonFiniteParameter { parameter, value } => {
                assert_eq!(parameter, "source_angle");
                assert!(value.is_nan());
            }
            _ => panic!("unexpected error: {error:?}"),
        }

        assert_eq!(
            try_link_radial(&RadialLink::new(0.0, 50.0, PI, 100.0), f64::INFINITY, 0.0)
                .unwrap_err(),
            LinkGenerationError::NonFiniteParameter {
                parameter: "cx",
                value: f64::INFINITY,
            }
        );
    }
}
