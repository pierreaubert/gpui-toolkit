use super::super::path::Point;

/// Compute a point on a basis spline.
pub(super) fn basis_point(p0: &Point, p1: &Point, p2: &Point, p3: &Point, t: f64) -> Point {
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

/// Compute a point on a cardinal spline.
pub(super) fn cardinal_point(
    p0: &Point,
    p1: &Point,
    p2: &Point,
    p3: &Point,
    t: f64,
    k: f64,
) -> Point {
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

/// Compute a point on a Catmull-Rom spline.
pub(super) fn catmull_rom_point(
    p0: &Point,
    p1: &Point,
    p2: &Point,
    p3: &Point,
    t: f64,
    alpha: f64,
) -> Point {
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

/// Compute second derivatives for natural cubic spline.
///
/// Accepts a length and a coordinate accessor so callers do not need to
/// allocate a temporary `Vec<f64>` just to extract one coordinate.
pub(super) fn natural_spline_derivs(values: impl Fn(usize) -> f64, n: usize) -> Vec<f64> {
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
        u[i] = (values(i + 1) - 2.0 * values(i) + values(i - 1)) * 6.0;
        u[i] = (u[i] - 0.5 * u[i - 1]) / p;
    }

    // Back substitution
    for k in (0..n - 1).rev() {
        y2[k] = y2[k] * y2[k + 1] + u[k];
    }

    y2
}
