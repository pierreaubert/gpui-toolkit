use super::point::Point;
use super::point::distance_to_line;
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy)]
pub(super) struct EllipticalArcCenter {
    pub center: Point,
    pub rx: f64,
    pub ry: f64,
    pub phi: f64,
    pub start_angle: f64,
    pub delta_angle: f64,
}

impl EllipticalArcCenter {
    pub fn point_at(self, angle: f64) -> Point {
        let (sin_phi, cos_phi) = self.phi.sin_cos();
        let ellipse_x = self.rx * angle.cos();
        let ellipse_y = self.ry * angle.sin();
        Point::new(
            self.center.x + cos_phi * ellipse_x - sin_phi * ellipse_y,
            self.center.y + sin_phi * ellipse_x + cos_phi * ellipse_y,
        )
    }

    pub fn contains_angle(self, angle: f64) -> bool {
        let tau = 2.0 * PI;
        if self.delta_angle >= 0.0 {
            (angle - self.start_angle).rem_euclid(tau) <= self.delta_angle + f64::EPSILON
        } else {
            (self.start_angle - angle).rem_euclid(tau) <= -self.delta_angle + f64::EPSILON
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "SVG elliptical arc API is defined by its endpoint and seven arc parameters"
)]
pub(super) fn elliptical_arc_center(
    start: Point,
    end: Point,
    mut rx: f64,
    mut ry: f64,
    x_axis_rotation: f64,
    large_arc: bool,
    sweep: bool,
) -> Option<EllipticalArcCenter> {
    if start == end {
        return None;
    }

    rx = rx.abs();
    ry = ry.abs();
    if rx == 0.0 || ry == 0.0 || !rx.is_finite() || !ry.is_finite() {
        return None;
    }

    let phi = x_axis_rotation.to_radians().rem_euclid(2.0 * PI);
    let (sin_phi, cos_phi) = phi.sin_cos();
    let half_dx = (start.x - end.x) / 2.0;
    let half_dy = (start.y - end.y) / 2.0;
    let x1_prime = cos_phi * half_dx + sin_phi * half_dy;
    let y1_prime = -sin_phi * half_dx + cos_phi * half_dy;

    let radii_scale = x1_prime.powi(2) / rx.powi(2) + y1_prime.powi(2) / ry.powi(2);
    if radii_scale > 1.0 {
        let scale = radii_scale.sqrt();
        rx *= scale;
        ry *= scale;
    }

    let rx2 = rx * rx;
    let ry2 = ry * ry;
    let x1_prime2 = x1_prime * x1_prime;
    let y1_prime2 = y1_prime * y1_prime;
    let denominator = rx2 * y1_prime2 + ry2 * x1_prime2;
    if denominator == 0.0 {
        return None;
    }

    let numerator = (rx2 * ry2 - denominator).max(0.0);
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    let coefficient = sign * (numerator / denominator).sqrt();
    let cx_prime = coefficient * rx * y1_prime / ry;
    let cy_prime = coefficient * -ry * x1_prime / rx;
    let center = Point::new(
        cos_phi * cx_prime - sin_phi * cy_prime + (start.x + end.x) / 2.0,
        sin_phi * cx_prime + cos_phi * cy_prime + (start.y + end.y) / 2.0,
    );

    let ux = (x1_prime - cx_prime) / rx;
    let uy = (y1_prime - cy_prime) / ry;
    let vx = (-x1_prime - cx_prime) / rx;
    let vy = (-y1_prime - cy_prime) / ry;
    let start_angle = uy.atan2(ux);
    let mut delta_angle = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    if sweep && delta_angle < 0.0 {
        delta_angle += 2.0 * PI;
    } else if !sweep && delta_angle > 0.0 {
        delta_angle -= 2.0 * PI;
    }

    Some(EllipticalArcCenter {
        center,
        rx,
        ry,
        phi,
        start_angle,
        delta_angle,
    })
}

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

/// Flatten an SVG endpoint-parameterized elliptical arc into line segments.
#[allow(
    clippy::too_many_arguments,
    reason = "SVG elliptical arc API is defined by its endpoint and seven arc parameters"
)]
pub(super) fn flatten_elliptical_arc(
    start: Point,
    end: Point,
    rx: f64,
    ry: f64,
    x_axis_rotation: f64,
    large_arc: bool,
    sweep: bool,
    tolerance: f64,
    points: &mut Vec<Point>,
) {
    if start == end {
        return;
    }

    let Some(arc) = elliptical_arc_center(start, end, rx, ry, x_axis_rotation, large_arc, sweep)
    else {
        points.push(end);
        return;
    };

    let max_radius = arc.rx.max(arc.ry);
    let tolerance = tolerance.abs().max(f64::EPSILON);
    let max_step = if tolerance >= max_radius {
        PI
    } else {
        (2.0 * (1.0 - tolerance / max_radius).acos()).max(0.01)
    };
    let segment_count = (arc.delta_angle.abs() / max_step).ceil().max(1.0) as usize;

    for index in 1..=segment_count {
        if index == segment_count {
            points.push(end);
            break;
        }
        let angle = arc.start_angle + arc.delta_angle * index as f64 / segment_count as f64;
        points.push(arc.point_at(angle));
    }
}
