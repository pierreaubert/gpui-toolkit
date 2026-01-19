//! Link shape generators
//!
//! Link shapes create smooth curves between two points, useful for
//! network visualizations and tree diagrams.

use super::path::PathBuilder;

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

    PathBuilder::new()
        .move_to(link.source_x, link.source_y)
        .cubic_curve_to(
            midx,
            link.source_y,
            midx,
            link.target_y,
            link.target_x,
            link.target_y,
        )
        .build()
        .to_svg_string()
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

    PathBuilder::new()
        .move_to(link.source_x, link.source_y)
        .cubic_curve_to(
            link.source_x,
            midy,
            link.target_x,
            midy,
            link.target_x,
            link.target_y,
        )
        .build()
        .to_svg_string()
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

    PathBuilder::new()
        .move_to(source_x, source_y)
        .quadratic_curve_to(mid_x, mid_y, target_x, target_y)
        .build()
        .to_svg_string()
}

/// Create a step link (orthogonal connection)
///
/// Creates a path with right-angle corners, useful for flowcharts.
pub fn link_step(link: &Link, direction: LinkDirection) -> String {
    match direction {
        LinkDirection::Horizontal => {
            let midx = (link.source_x + link.target_x) / 2.0;
            PathBuilder::new()
                .move_to(link.source_x, link.source_y)
                .line_to(midx, link.source_y)
                .line_to(midx, link.target_y)
                .line_to(link.target_x, link.target_y)
                .build()
                .to_svg_string()
        }
        LinkDirection::Vertical => {
            let midy = (link.source_y + link.target_y) / 2.0;
            PathBuilder::new()
                .move_to(link.source_x, link.source_y)
                .line_to(link.source_x, midy)
                .line_to(link.target_x, midy)
                .line_to(link.target_x, link.target_y)
                .build()
                .to_svg_string()
        }
        LinkDirection::Radial => {
            // For radial, just use straight lines
            PathBuilder::new()
                .move_to(link.source_x, link.source_y)
                .line_to(link.target_x, link.target_y)
                .build()
                .to_svg_string()
        }
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
}
