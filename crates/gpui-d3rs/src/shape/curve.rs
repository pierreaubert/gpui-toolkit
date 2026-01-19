//! Curve interpolation algorithms
//!
//! Provides various curve types for line and area chart interpolation.

use super::path::Point;

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
        if points.len() < 2 {
            return points.to_vec();
        }

        match self {
            Curve::Linear => points.to_vec(),
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
        }
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

/// Interpolate using step function.
fn interpolate_step(points: &[Point], position: f64) -> Vec<Point> {
    let mut result = Vec::with_capacity(points.len() * 2);

    for i in 0..points.len() {
        result.push(points[i]);

        if i < points.len() - 1 {
            let mid_x = points[i].x + (points[i + 1].x - points[i].x) * position;
            result.push(Point::new(mid_x, points[i].y));
            result.push(Point::new(mid_x, points[i + 1].y));
        }
    }

    result
}

/// Interpolate using basis spline.
fn interpolate_basis(points: &[Point]) -> Vec<Point> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let n = points.len();
    let subdivisions = 16;
    let mut result = Vec::with_capacity((n - 1) * subdivisions + 1);

    // Add reflected points for endpoints
    let p0 = Point::new(
        2.0 * points[0].x - points[1].x,
        2.0 * points[0].y - points[1].y,
    );
    let pn = Point::new(
        2.0 * points[n - 1].x - points[n - 2].x,
        2.0 * points[n - 1].y - points[n - 2].y,
    );

    for i in 0..n - 1 {
        let v0 = if i == 0 { p0 } else { points[i - 1] };
        let v1 = points[i];
        let v2 = points[i + 1];
        let v3 = if i == n - 2 { pn } else { points[i + 2] };

        for j in 0..subdivisions {
            let t = j as f64 / subdivisions as f64;
            result.push(basis_point(&v0, &v1, &v2, &v3, t));
        }
    }

    // Add final point
    result.push(*points.last().unwrap());

    result
}

/// Compute a point on a basis spline.
fn basis_point(p0: &Point, p1: &Point, p2: &Point, p3: &Point, t: f64) -> Point {
    let t2 = t * t;
    let t3 = t2 * t;

    let b0 = (1.0 - 3.0 * t + 3.0 * t2 - t3) / 6.0;
    let b1 = (4.0 - 6.0 * t2 + 3.0 * t3) / 6.0;
    let b2 = (1.0 + 3.0 * t + 3.0 * t2 - 3.0 * t3) / 6.0;
    let b3 = t3 / 6.0;

    Point::new(
        b0 * p0.x + b1 * p1.x + b2 * p2.x + b3 * p3.x,
        b0 * p0.y + b1 * p1.y + b2 * p2.y + b3 * p3.y,
    )
}

/// Interpolate using bundle curve.
fn interpolate_bundle(points: &[Point], beta: f64) -> Vec<Point> {
    // Bundle is a blend between straight line and basis curve
    let basis_points = interpolate_basis(points);

    if (beta - 1.0).abs() < 0.001 {
        return basis_points;
    }

    if beta.abs() < 0.001 {
        // Straight line from first to last
        return vec![points[0], *points.last().unwrap()];
    }

    // Blend between straight line and basis
    let first = points[0];
    let last = *points.last().unwrap();

    basis_points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let t = i as f64 / (basis_points.len() - 1) as f64;
            let straight = Point::new(
                first.x + (last.x - first.x) * t,
                first.y + (last.y - first.y) * t,
            );
            Point::new(
                straight.x + (p.x - straight.x) * beta,
                straight.y + (p.y - straight.y) * beta,
            )
        })
        .collect()
}

/// Interpolate using cardinal spline.
fn interpolate_cardinal(points: &[Point], tension: f64) -> Vec<Point> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let n = points.len();
    let subdivisions = 16;
    let k = (1.0 - tension) / 2.0;
    let mut result = Vec::with_capacity((n - 1) * subdivisions + 1);

    result.push(points[0]);

    for i in 0..n - 1 {
        let p0 = if i > 0 { points[i - 1] } else { points[i] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < n {
            points[i + 2]
        } else {
            points[i + 1]
        };

        for j in 1..=subdivisions {
            let t = j as f64 / subdivisions as f64;
            result.push(cardinal_point(&p0, &p1, &p2, &p3, t, k));
        }
    }

    result
}

/// Compute a point on a cardinal spline.
fn cardinal_point(p0: &Point, p1: &Point, p2: &Point, p3: &Point, t: f64, k: f64) -> Point {
    let t2 = t * t;
    let t3 = t2 * t;

    // Hermite basis functions
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;

    // Tangents
    let m1 = Point::new(k * (p2.x - p0.x), k * (p2.y - p0.y));
    let m2 = Point::new(k * (p3.x - p1.x), k * (p3.y - p1.y));

    Point::new(
        h00 * p1.x + h10 * m1.x + h01 * p2.x + h11 * m2.x,
        h00 * p1.y + h10 * m1.y + h01 * p2.y + h11 * m2.y,
    )
}

/// Interpolate using Catmull-Rom spline.
fn interpolate_catmull_rom(points: &[Point], alpha: f64) -> Vec<Point> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let n = points.len();
    let subdivisions = 16;
    let mut result = Vec::with_capacity((n - 1) * subdivisions + 1);

    result.push(points[0]);

    for i in 0..n - 1 {
        let p0 = if i > 0 { points[i - 1] } else { points[i] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < n {
            points[i + 2]
        } else {
            points[i + 1]
        };

        for j in 1..=subdivisions {
            let t = j as f64 / subdivisions as f64;
            result.push(catmull_rom_point(&p0, &p1, &p2, &p3, t, alpha));
        }
    }

    result
}

/// Compute a point on a Catmull-Rom spline.
fn catmull_rom_point(p0: &Point, p1: &Point, p2: &Point, p3: &Point, t: f64, alpha: f64) -> Point {
    // Calculate knot intervals based on alpha
    fn dist(a: &Point, b: &Point, alpha: f64) -> f64 {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        (dx * dx + dy * dy).powf(alpha * 0.5)
    }

    let d01 = dist(p0, p1, alpha).max(1e-8);
    let d12 = dist(p1, p2, alpha).max(1e-8);
    let d23 = dist(p2, p3, alpha).max(1e-8);

    // Compute tangents
    let m1 = Point::new(
        (p2.x - p0.x) / (d01 + d12) * d12 + (p1.x - p0.x) / d01 * d12,
        (p2.y - p0.y) / (d01 + d12) * d12 + (p1.y - p0.y) / d01 * d12,
    );

    let m2 = Point::new(
        (p3.x - p1.x) / (d12 + d23) * d12 + (p2.x - p1.x) / d12 * d12,
        (p3.y - p1.y) / (d12 + d23) * d12 + (p2.y - p1.y) / d12 * d12,
    );

    // Hermite interpolation
    let t2 = t * t;
    let t3 = t2 * t;

    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;

    Point::new(
        h00 * p1.x + h10 * m1.x + h01 * p2.x + h11 * m2.x,
        h00 * p1.y + h10 * m1.y + h01 * p2.y + h11 * m2.y,
    )
}

/// Interpolate using monotone X spline.
fn interpolate_monotone_x(points: &[Point]) -> Vec<Point> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let n = points.len();
    let subdivisions = 16;

    // Calculate slopes
    let mut slopes: Vec<f64> = Vec::with_capacity(n);

    for i in 0..n {
        if i == 0 {
            slopes.push((points[1].y - points[0].y) / (points[1].x - points[0].x).max(1e-8));
        } else if i == n - 1 {
            slopes.push(
                (points[n - 1].y - points[n - 2].y) / (points[n - 1].x - points[n - 2].x).max(1e-8),
            );
        } else {
            let s0 = (points[i].y - points[i - 1].y) / (points[i].x - points[i - 1].x).max(1e-8);
            let s1 = (points[i + 1].y - points[i].y) / (points[i + 1].x - points[i].x).max(1e-8);

            // Preserve monotonicity
            if s0 * s1 <= 0.0 {
                slopes.push(0.0);
            } else {
                slopes.push(3.0 * (s0 + s1) / ((s0 / s1 + 2.0) + (s1 / s0 + 2.0)));
            }
        }
    }

    let mut result = Vec::with_capacity((n - 1) * subdivisions + 1);
    result.push(points[0]);

    for i in 0..n - 1 {
        let dx = points[i + 1].x - points[i].x;

        for j in 1..=subdivisions {
            let t = j as f64 / subdivisions as f64;
            let t2 = t * t;
            let t3 = t2 * t;

            let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
            let h10 = t3 - 2.0 * t2 + t;
            let h01 = -2.0 * t3 + 3.0 * t2;
            let h11 = t3 - t2;

            result.push(Point::new(
                points[i].x + dx * t,
                h00 * points[i].y
                    + h10 * dx * slopes[i]
                    + h01 * points[i + 1].y
                    + h11 * dx * slopes[i + 1],
            ));
        }
    }

    result
}

/// Interpolate using monotone Y spline.
fn interpolate_monotone_y(points: &[Point]) -> Vec<Point> {
    // Swap x and y, interpolate, then swap back
    let swapped: Vec<Point> = points.iter().map(|p| Point::new(p.y, p.x)).collect();
    let result = interpolate_monotone_x(&swapped);
    result.iter().map(|p| Point::new(p.y, p.x)).collect()
}

/// Interpolate using natural cubic spline.
fn interpolate_natural(points: &[Point]) -> Vec<Point> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let n = points.len();
    let subdivisions = 16;

    // Solve tridiagonal system for second derivatives
    let second_derivs_x = natural_spline_derivs(&points.iter().map(|p| p.x).collect::<Vec<_>>());
    let second_derivs_y = natural_spline_derivs(&points.iter().map(|p| p.y).collect::<Vec<_>>());

    let mut result = Vec::with_capacity((n - 1) * subdivisions + 1);
    result.push(points[0]);

    for i in 0..n - 1 {
        let h = 1.0; // Assuming uniform parameterization

        for j in 1..=subdivisions {
            let t = j as f64 / subdivisions as f64;

            let a = 1.0 - t;
            let b = t;

            let x = a * points[i].x
                + b * points[i + 1].x
                + (a * a * a - a) * second_derivs_x[i] * h * h / 6.0
                + (b * b * b - b) * second_derivs_x[i + 1] * h * h / 6.0;

            let y = a * points[i].y
                + b * points[i + 1].y
                + (a * a * a - a) * second_derivs_y[i] * h * h / 6.0
                + (b * b * b - b) * second_derivs_y[i + 1] * h * h / 6.0;

            result.push(Point::new(x, y));
        }
    }

    result
}

/// Compute second derivatives for natural cubic spline.
fn natural_spline_derivs(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    if n < 2 {
        return vec![0.0; n];
    }

    let mut u = vec![0.0; n];
    let mut y2 = vec![0.0; n];

    // Forward pass
    for i in 1..n - 1 {
        let sig = 0.5;
        let p = sig * y2[i - 1] + 2.0;
        y2[i] = (sig - 1.0) / p;
        u[i] = (values[i + 1] - 2.0 * values[i] + values[i - 1]) * 6.0;
        u[i] = (u[i] - 0.5 * u[i - 1]) / p;
    }

    // Back substitution
    for k in (0..n - 1).rev() {
        y2[k] = y2[k] * y2[k + 1] + u[k];
    }

    y2
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
}
