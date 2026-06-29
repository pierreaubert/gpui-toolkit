use super::point::Point;
use super::point::distance_to_line;
use std::f64::consts::PI;

/// Flatten a quadratic Bezier curve into line segments.
pub(super) fn flatten_quadratic(
    p0: &Point,
    p1: &Point,
    p2: &Point,
    tolerance: f64,
    points: &mut Vec<Point>,
) {
    // Check if the curve is flat enough
    let mid = p0.lerp(p2, 0.5);
    let control_dist = mid.distance(p1);

    if control_dist < tolerance {
        points.push(*p2);
    } else {
        // Subdivide
        let p01 = p0.lerp(p1, 0.5);
        let p12 = p1.lerp(p2, 0.5);
        let p012 = p01.lerp(&p12, 0.5);

        flatten_quadratic(p0, &p01, &p012, tolerance, points);
        flatten_quadratic(&p012, &p12, p2, tolerance, points);
    }
}

/// Flatten a cubic Bezier curve into line segments.
pub(super) fn flatten_cubic(
    p0: &Point,
    p1: &Point,
    p2: &Point,
    p3: &Point,
    tolerance: f64,
    points: &mut Vec<Point>,
) {
    // Check if the curve is flat enough using de Casteljau subdivision
    let d1 = distance_to_line(p1, p0, p3);
    let d2 = distance_to_line(p2, p0, p3);

    if d1 + d2 < tolerance {
        points.push(*p3);
    } else {
        // Subdivide
        let p01 = p0.lerp(p1, 0.5);
        let p12 = p1.lerp(p2, 0.5);
        let p23 = p2.lerp(p3, 0.5);
        let p012 = p01.lerp(&p12, 0.5);
        let p123 = p12.lerp(&p23, 0.5);
        let p0123 = p012.lerp(&p123, 0.5);

        flatten_cubic(p0, &p01, &p012, &p0123, tolerance, points);
        flatten_cubic(&p0123, &p123, &p23, p3, tolerance, points);
    }
}

/// Flatten an arc into line segments.
#[allow(
    clippy::too_many_arguments,
    reason = "arc flattening primitive mirrors canvas arc parameters plus output buffer"
)]
pub(super) fn flatten_arc(
    cx: f64,
    cy: f64,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    anticlockwise: bool,
    tolerance: f64,
    points: &mut Vec<Point>,
) {
    // Calculate number of segments needed
    let mut delta = end_angle - start_angle;

    if anticlockwise {
        if delta > 0.0 {
            delta -= 2.0 * PI;
        }
    } else if delta < 0.0 {
        delta += 2.0 * PI;
    }

    let n = ((delta.abs() * radius / tolerance).sqrt().ceil() as usize).max(1);

    for i in 1..=n {
        let t = i as f64 / n as f64;
        let angle = start_angle + delta * t;
        points.push(Point::new(
            cx + radius * angle.cos(),
            cy + radius * angle.sin(),
        ));
    }
}
