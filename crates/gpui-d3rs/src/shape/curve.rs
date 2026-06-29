//! Curve interpolation algorithms
//!
//! Provides various curve types for line and area chart interpolation.

use super::path::Point;

mod interpolate;
mod misc;

use interpolate::interpolate_basis;
use interpolate::interpolate_bundle;
use interpolate::interpolate_cardinal;
use interpolate::interpolate_catmull_rom;
use interpolate::interpolate_monotone_x;
use interpolate::interpolate_monotone_y;
use interpolate::interpolate_natural;
use interpolate::interpolate_step;

/// Curve interpolation type.
///
/// # Example
///
/// ```
/// use d3rs::shape::curve::Curve;
/// use d3rs::shape::path::Point;
///
/// let points = vec![
///     Point::new(0.0, 0.0),
///     Point::new(10.0, 50.0),
///     Point::new(20.0, 30.0),
///     Point::new(30.0, 70.0),
/// ];
///
/// let curved = Curve::CatmullRom { alpha: 0.5 }.interpolate(&points);
/// assert!(curved.len() > points.len());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Curve {
    /// Linear interpolation (straight lines)
    #[default]
    Linear,
    /// Step function (horizontal then vertical)
    Step,
    /// Step before (vertical then horizontal at start)
    StepBefore,
    /// Step after (horizontal then vertical at end)
    StepAfter,
    /// Basis spline (smooth, doesn't pass through points)
    Basis,
    /// Closed basis spline
    BasisClosed,
    /// Open basis spline (no tangents at endpoints)
    BasisOpen,
    /// Bundle curve (straightens toward 0 = straight line, 1 = basis)
    Bundle { beta: f64 },
    /// Cardinal spline (passes through points with tension)
    Cardinal { tension: f64 },
    /// Closed cardinal spline
    CardinalClosed { tension: f64 },
    /// Open cardinal spline
    CardinalOpen { tension: f64 },
    /// Catmull-Rom spline (passes through points)
    CatmullRom { alpha: f64 },
    /// Closed Catmull-Rom spline
    CatmullRomClosed { alpha: f64 },
    /// Open Catmull-Rom spline
    CatmullRomOpen { alpha: f64 },
    /// Monotone in X (preserves monotonicity)
    MonotoneX,
    /// Monotone in Y (preserves monotonicity)
    MonotoneY,
    /// Natural cubic spline
    Natural,
}

impl Curve {
    /// Create a linear curve.
    pub fn linear() -> Self {
        Curve::Linear
    }

    /// Create a basis spline curve.
    pub fn basis() -> Self {
        Curve::Basis
    }

    /// Create a cardinal spline with the given tension.
    ///
    /// Tension ranges from 0 (smooth) to 1 (straight lines).
    pub fn cardinal(tension: f64) -> Self {
        Curve::Cardinal {
            tension: tension.clamp(0.0, 1.0),
        }
    }

    /// Create a Catmull-Rom spline with the given alpha.
    ///
    /// - alpha = 0: uniform Catmull-Rom
    /// - alpha = 0.5: centripetal Catmull-Rom
    /// - alpha = 1: chordal Catmull-Rom
    pub fn catmull_rom(alpha: f64) -> Self {
        Curve::CatmullRom {
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// Create a monotone X curve.
    pub fn monotone_x() -> Self {
        Curve::MonotoneX
    }

    /// Create a natural cubic spline.
    pub fn natural() -> Self {
        Curve::Natural
    }

    /// Interpolate points using this curve type.
    ///
    /// Returns a new set of points that follow the curve.
    pub fn interpolate(&self, points: &[Point]) -> Vec<Point> {
        let mut out = Vec::new();
        self.interpolate_into(points, &mut out);
        out
    }

    /// Interpolate points using this curve type, appending into `out`.
    ///
    /// `out` is cleared first. Callers can reuse `out` across frames to avoid
    /// reallocating the output buffer.
    pub fn interpolate_into(&self, points: &[Point], out: &mut Vec<Point>) {
        out.clear();
        if points.len() < 2 {
            out.extend_from_slice(points);
            return;
        }

        let pts = match self {
            Curve::Linear => {
                out.extend_from_slice(points);
                return;
            }
            Curve::Step => interpolate_step(points, 0.5),
            Curve::StepBefore => interpolate_step(points, 0.0),
            Curve::StepAfter => interpolate_step(points, 1.0),
            Curve::Basis | Curve::BasisClosed | Curve::BasisOpen => interpolate_basis(points),
            Curve::Bundle { beta } => interpolate_bundle(points, *beta),
            Curve::Cardinal { tension }
            | Curve::CardinalClosed { tension }
            | Curve::CardinalOpen { tension } => interpolate_cardinal(points, *tension),
            Curve::CatmullRom { alpha }
            | Curve::CatmullRomClosed { alpha }
            | Curve::CatmullRomOpen { alpha } => interpolate_catmull_rom(points, *alpha),
            Curve::MonotoneX => interpolate_monotone_x(points),
            Curve::MonotoneY => interpolate_monotone_y(points),
            Curve::Natural => interpolate_natural(points),
        };
        out.extend_from_slice(&pts);
    }

    /// Get the number of subdivisions per segment for this curve type.
    pub fn subdivisions(&self) -> usize {
        match self {
            Curve::Linear => 1,
            Curve::Step | Curve::StepBefore | Curve::StepAfter => 2,
            _ => 16, // Smooth curves get more subdivisions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_curve() {
        let points = vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)];
        let result = Curve::Linear.interpolate(&points);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_step_curve() {
        let points = vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)];
        let result = Curve::Step.interpolate(&points);
        assert!(result.len() > 2);
    }

    #[test]
    fn test_basis_curve() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 50.0),
            Point::new(20.0, 30.0),
            Point::new(30.0, 70.0),
        ];
        let result = Curve::Basis.interpolate(&points);
        assert!(result.len() > points.len());
    }

    #[test]
    fn test_cardinal_curve() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 50.0),
            Point::new(20.0, 30.0),
        ];
        let result = Curve::cardinal(0.5).interpolate(&points);
        assert!(result.len() > points.len());
    }

    #[test]
    fn test_catmull_rom_curve() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 50.0),
            Point::new(20.0, 30.0),
            Point::new(30.0, 70.0),
        ];
        let result = Curve::catmull_rom(0.5).interpolate(&points);
        assert!(result.len() > points.len());
    }

    #[test]
    fn test_monotone_x() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 50.0),
            Point::new(20.0, 30.0),
        ];
        let result = Curve::MonotoneX.interpolate(&points);
        assert!(result.len() > points.len());
    }

    #[test]
    fn test_natural_spline() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 50.0),
            Point::new(20.0, 30.0),
        ];
        let result = Curve::Natural.interpolate(&points);
        assert!(result.len() > points.len());
    }

    #[test]
    fn test_interpolate_into_matches_interpolate() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 50.0),
            Point::new(20.0, 30.0),
            Point::new(30.0, 70.0),
        ];

        for curve in [
            Curve::Linear,
            Curve::Step,
            Curve::Basis,
            Curve::cardinal(0.5),
            Curve::catmull_rom(0.5),
            Curve::MonotoneX,
            Curve::Natural,
        ] {
            let expected = curve.interpolate(&points);
            let mut out = Vec::new();
            curve.interpolate_into(&points, &mut out);
            assert_eq!(
                expected, out,
                "interpolate_into should match interpolate for {:?}",
                curve
            );
        }
    }
}
