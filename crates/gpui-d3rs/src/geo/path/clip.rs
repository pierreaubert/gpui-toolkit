// D3-geo spherical clipping ported to Rust.
//
// Operates in the projection's rotated frame and returns pieces in unrotated
// geographic coordinates. Based on d3-geo's clip/antimeridian.js,
// clip/circle.js, clip/index.js, clip/rejoin.js, circle.js and polygonContains.js.

use super::super::projection::SphereRotation;
use super::super::{degrees, radians};
use std::f64::consts::PI;

const EPSILON: f64 = 1e-6;
const EPSILON2: f64 = 1e-12;
const HALF_PI: f64 = PI / 2.0;
const QUARTER_PI: f64 = PI / 4.0;
const TAU: f64 = PI * 2.0;

// -----------------------------------------------------------------------------
// Math helpers
// -----------------------------------------------------------------------------

fn acos_clamped(x: f64) -> f64 {
    if x > 1.0 {
        0.0
    } else if x < -1.0 {
        PI
    } else {
        x.acos()
    }
}

fn asin_clamped(x: f64) -> f64 {
    if x > 1.0 {
        HALF_PI
    } else if x < -1.0 {
        -HALF_PI
    } else {
        x.asin()
    }
}

fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn longitude(lambda: f64) -> f64 {
    if lambda.abs() <= PI {
        lambda
    } else {
        sign(lambda) * ((lambda.abs() + PI) % TAU - PI)
    }
}

// -----------------------------------------------------------------------------
// Cartesian / spherical helpers
// -----------------------------------------------------------------------------

pub(crate) fn cartesian(lambda: f64, phi: f64) -> [f64; 3] {
    let cos_phi = phi.cos();
    [cos_phi * lambda.cos(), cos_phi * lambda.sin(), phi.sin()]
}

fn spherical(v: [f64; 3]) -> (f64, f64) {
    (v[1].atan2(v[0]), asin_clamped(v[2]))
}

fn cartesian_dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cartesian_cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn cartesian_scale(v: [f64; 3], k: f64) -> [f64; 3] {
    [v[0] * k, v[1] * k, v[2] * k]
}

fn cartesian_add_in_place(a: &mut [f64; 3], b: [f64; 3]) {
    a[0] += b[0];
    a[1] += b[1];
    a[2] += b[2];
}

fn cartesian_normalize_in_place(v: &mut [f64; 3]) {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > EPSILON2 {
        v[0] /= len;
        v[1] /= len;
        v[2] /= len;
    }
}

fn point_equal(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < EPSILON && (a.1 - b.1).abs() < EPSILON
}

// -----------------------------------------------------------------------------
// Adaptive spherical-line resampling (d3-geo projection/resample.js)
// -----------------------------------------------------------------------------

const RESAMPLE_MAX_DEPTH: i32 = 16;
const RESAMPLE_DELTA2: f64 = 0.05;
const RESAMPLE_COS_MIN_DISTANCE: f64 = 0.8660254037844386; // cos(30°)

fn resample_line_to(
    x0: f64,
    y0: f64,
    lambda0: f64,
    a0: f64,
    b0: f64,
    c0: f64,
    x1: f64,
    y1: f64,
    lambda1: f64,
    a1: f64,
    b1: f64,
    c1: f64,
    depth: i32,
    project: &dyn Fn(f64, f64) -> (f64, f64),
    delta2: f64,
    cos_min_distance: f64,
    out: &mut Vec<(f64, f64)>,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let d2 = dx * dx + dy * dy;

    if d2 > 4.0 * delta2 && depth > 0 {
        let mut a = a0 + a1;
        let mut b = b0 + b1;
        let mut c = c0 + c1;
        let m = (a * a + b * b + c * c).sqrt();
        a /= m;
        b /= m;
        c /= m;

        let phi2 = asin_clamped(c);
        let lambda2 = if (c.abs() - 1.0).abs() < EPSILON || (lambda0 - lambda1).abs() < EPSILON {
            (lambda0 + lambda1) / 2.0
        } else {
            b.atan2(a)
        };

        let (x2, y2) = project(lambda2, phi2);
        if x2.is_finite() && y2.is_finite() {
            let dx2 = x2 - x0;
            let dy2 = y2 - y0;
            let dz = dy * dx2 - dx * dy2;
            let split = (dz * dz / d2 > delta2)
                || ((dx * dx2 + dy * dy2) / d2 - 0.5).abs() > 0.3
                || (a0 * a1 + b0 * b1 + c0 * c1) < cos_min_distance;

            if split {
                resample_line_to(
                    x0,
                    y0,
                    lambda0,
                    a0,
                    b0,
                    c0,
                    x2,
                    y2,
                    lambda2,
                    a,
                    b,
                    c,
                    depth - 1,
                    project,
                    delta2,
                    cos_min_distance,
                    out,
                );
                out.push((x2, y2));
                resample_line_to(
                    x2,
                    y2,
                    lambda2,
                    a,
                    b,
                    c,
                    x1,
                    y1,
                    lambda1,
                    a1,
                    b1,
                    c1,
                    depth - 1,
                    project,
                    delta2,
                    cos_min_distance,
                    out,
                );
            }
        }
    }
}

/// Adaptively resample a spherical polyline so that its projected image is
/// within D3's default precision (`delta2 = 0.5`) of a straight-line
/// approximation.
///
/// * `points` – vertices in radians on the unit sphere.
/// * `project` – maps `(lambda, phi)` to screen `(x, y)`.
/// * `closed` – if true, also resample the closing edge (for polygon rings).
///
/// Returns the projected `(x, y)` points of the resampled curve.
pub fn resample_spherical_line(
    points: &[(f64, f64)],
    project: &dyn Fn(f64, f64) -> (f64, f64),
    closed: bool,
) -> Vec<(f64, f64)> {
    let n = points.len();
    if n == 0 {
        return Vec::new();
    }

    let mut projected = Vec::with_capacity(n);
    let mut cartesian_pts = Vec::with_capacity(n);
    for &(lambda, phi) in points {
        projected.push(project(lambda, phi));
        cartesian_pts.push(cartesian(lambda, phi));
    }

    let mut out = Vec::new();
    let first = projected[0];
    out.push(first);

    for i in 0..n - 1 {
        let j = i + 1;
        resample_line_to(
            projected[i].0,
            projected[i].1,
            points[i].0,
            cartesian_pts[i][0],
            cartesian_pts[i][1],
            cartesian_pts[i][2],
            projected[j].0,
            projected[j].1,
            points[j].0,
            cartesian_pts[j][0],
            cartesian_pts[j][1],
            cartesian_pts[j][2],
            RESAMPLE_MAX_DEPTH,
            project,
            RESAMPLE_DELTA2,
            RESAMPLE_COS_MIN_DISTANCE,
            &mut out,
        );
        if projected[j].0.is_finite() && projected[j].1.is_finite() {
            out.push(projected[j]);
        }
    }

    if closed && n > 1 {
        resample_line_to(
            projected[n - 1].0,
            projected[n - 1].1,
            points[n - 1].0,
            cartesian_pts[n - 1][0],
            cartesian_pts[n - 1][1],
            cartesian_pts[n - 1][2],
            projected[0].0,
            projected[0].1,
            points[0].0,
            cartesian_pts[0][0],
            cartesian_pts[0][1],
            cartesian_pts[0][2],
            RESAMPLE_MAX_DEPTH,
            project,
            RESAMPLE_DELTA2,
            RESAMPLE_COS_MIN_DISTANCE,
            &mut out,
        );
    }

    out
}

// -----------------------------------------------------------------------------
// Rotation helpers
// -----------------------------------------------------------------------------

fn rotate_point_rad(rotation: &SphereRotation, lon: f64, lat: f64) -> (f64, f64) {
    rotation.rotate(radians(lon), radians(lat))
}

const POLE_SNAP_RAD: f64 = 1e-3;

fn unrotate_point_deg(rotation: &SphereRotation, lambda: f64, phi: f64) -> (f64, f64) {
    let (lon, lat) = rotation.invert(lambda, phi);
    // Near the poles the inverse rotation's longitude is numerically unstable:
    // tiny changes in the rotated latitude can shift the returned longitude by
    // 90° or 180°.  Since longitude is arbitrary at the pole, nudge it so that
    // re-rotating the point reproduces the original rotated longitude.  This
    // keeps projected bounds stable and matches d3-geo's round-trip behaviour.
    let (lon, lat) = if phi.abs() > HALF_PI - POLE_SNAP_RAD {
        let (rot_lon, rot_phi) = rotation.rotate(lon, lat);
        let err = rot_lon - lambda;
        let _ = rot_phi;
        (lon - err, lat)
    } else {
        (lon, lat)
    };
    (degrees(lon), degrees(lat))
}

// -----------------------------------------------------------------------------
// Stream-like sink trait
// -----------------------------------------------------------------------------

trait Sink {
    fn point(&mut self, lambda: f64, phi: f64, m: i32);
    fn line_start(&mut self);
    fn line_end(&mut self);
}

// -----------------------------------------------------------------------------
// ClipBuffer: collects clipped segments as [lambda, phi, m] points.
// -----------------------------------------------------------------------------

struct ClipBuffer {
    lines: Vec<Vec<[f64; 3]>>,
    current_index: usize,
}

impl ClipBuffer {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            current_index: 0,
        }
    }

    fn result(&mut self) -> Vec<Vec<[f64; 3]>> {
        std::mem::take(&mut self.lines)
    }
}

impl Sink for ClipBuffer {
    fn point(&mut self, lambda: f64, phi: f64, m: i32) {
        self.lines[self.current_index].push([lambda, phi, m as f64]);
    }

    fn line_start(&mut self) {
        self.lines.push(Vec::new());
        self.current_index = self.lines.len() - 1;
    }

    fn line_end(&mut self) {
        // No-op; segments are already appended to the current line.
    }
}

// -----------------------------------------------------------------------------
// OutSink: collects visible line pieces as geographic (lambda, phi) points.
// -----------------------------------------------------------------------------

struct OutSink {
    pieces: Vec<Vec<(f64, f64)>>,
    current: Vec<(f64, f64)>,
}

impl OutSink {
    fn new() -> Self {
        Self {
            pieces: Vec::new(),
            current: Vec::new(),
        }
    }

    fn result(self) -> Vec<Vec<(f64, f64)>> {
        self.pieces
    }
}

impl Sink for OutSink {
    fn point(&mut self, lambda: f64, phi: f64, _m: i32) {
        self.current.push((lambda, phi));
    }

    fn line_start(&mut self) {
        self.current = Vec::new();
    }

    fn line_end(&mut self) {
        if self.current.len() > 1 {
            self.pieces.push(std::mem::take(&mut self.current));
        }
        self.current.clear();
    }
}

impl<T: Sink + ?Sized> Sink for &mut T {
    fn point(&mut self, lambda: f64, phi: f64, m: i32) {
        (*self).point(lambda, phi, m)
    }

    fn line_start(&mut self) {
        (*self).line_start()
    }

    fn line_end(&mut self) {
        (*self).line_end()
    }
}

// -----------------------------------------------------------------------------
// Circle stream: samples points along a small circle centered at the origin.
// -----------------------------------------------------------------------------

fn circle_stream_angles(
    radius: f64,
    delta: f64,
    direction: f64,
    mut t0: f64,
    mut t1: f64,
) -> Vec<(f64, f64)> {
    if delta == 0.0 {
        return Vec::new();
    }
    let cos_radius = radius.cos();
    let sin_radius = radius.sin();
    let step = direction * delta;

    if direction > 0.0 && t0 < t1 {
        t0 += direction * TAU;
    } else if direction < 0.0 && t0 > t1 {
        t0 += direction * TAU;
    }

    let mut points = Vec::new();
    let mut t = t0;
    while if direction > 0.0 { t > t1 } else { t < t1 } {
        let p = spherical([cos_radius, -sin_radius * t.cos(), -sin_radius * t.sin()]);
        points.push(p);
        t -= step;
    }
    points
}

/// Generate a full small circle, matching d3's circleStream when from/to are null.
fn circle_stream_full(radius: f64, delta: f64, direction: f64) -> Vec<(f64, f64)> {
    if delta == 0.0 {
        return Vec::new();
    }
    let cos_radius = radius.cos();
    let sin_radius = radius.sin();
    let step = direction * delta;
    let t0 = radius + direction * TAU;
    let t1 = radius - step / 2.0;
    let mut points = Vec::new();
    let mut t = t0;
    while if direction > 0.0 { t > t1 } else { t < t1 } {
        let p = spherical([cos_radius, -sin_radius * t.cos(), -sin_radius * t.sin()]);
        points.push(p);
        t -= step;
    }
    points
}

fn circle_radius(cos_radius: f64, point: (f64, f64)) -> f64 {
    let mut p = cartesian(point.0, point.1);
    p[0] -= cos_radius;
    cartesian_normalize_in_place(&mut p);
    let radius = acos_clamped(-p[1]);
    let adjusted = if -p[2] < 0.0 { -radius } else { radius };
    let mut out = ((adjusted + TAU - EPSILON) % TAU + TAU) % TAU;
    if out >= TAU {
        out -= TAU;
    }
    out
}

fn circle_stream_points(
    radius: f64,
    delta: f64,
    direction: f64,
    from: (f64, f64),
    to: (f64, f64),
) -> Vec<(f64, f64)> {
    let cos_radius = radius.cos();
    let t0 = circle_radius(cos_radius, from);
    let t1 = circle_radius(cos_radius, to);
    circle_stream_angles(radius, delta, direction, t0, t1)
}

// -----------------------------------------------------------------------------
// Antimeridian clip line
// -----------------------------------------------------------------------------

fn antimeridian_intersect(lambda0: f64, phi0: f64, lambda1: f64, phi1: f64) -> f64 {
    let sin_lambda0_lambda1 = (lambda0 - lambda1).sin();
    if sin_lambda0_lambda1.abs() > EPSILON {
        let cos_phi1 = phi1.cos();
        let cos_phi0 = phi0.cos();
        ((phi0.sin() * cos_phi1 * lambda1.sin() - phi1.sin() * cos_phi0 * lambda0.sin())
            / (cos_phi0 * cos_phi1 * sin_lambda0_lambda1))
            .atan()
    } else {
        (phi0 + phi1) / 2.0
    }
}

struct AntimeridianClipLine<S: Sink> {
    sink: S,
    lambda0: f64,
    phi0: f64,
    sign0: f64,
    clean: i32,
}

impl<S: Sink> AntimeridianClipLine<S> {
    fn new(sink: S) -> Self {
        Self {
            sink,
            lambda0: f64::NAN,
            phi0: f64::NAN,
            sign0: f64::NAN,
            clean: 1,
        }
    }

    fn line_start(&mut self) {
        self.sink.line_start();
        self.clean = 1;
    }

    fn point(&mut self, lambda1: f64, phi1: f64, m: i32) {
        let sign1 = if lambda1 > 0.0 { PI } else { -PI };
        let delta = (lambda1 - self.lambda0).abs();

        if (delta - PI).abs() < EPSILON {
            // Line crosses a pole.
            let phi_pole = if (self.phi0 + phi1) / 2.0 > 0.0 {
                HALF_PI
            } else {
                -HALF_PI
            };
            self.sink.point(self.lambda0, phi_pole, 0);
            self.sink.point(self.sign0, phi_pole, 0);
            self.sink.line_end();
            self.sink.line_start();
            self.sink.point(sign1, phi_pole, 0);
            self.sink.point(lambda1, phi_pole, 0);
            self.clean = 0;
        } else if self.sign0 != sign1 && delta >= PI {
            // Line crosses antimeridian.
            let mut lambda0 = self.lambda0;
            let mut lambda1 = lambda1;
            if (lambda0 - self.sign0).abs() < EPSILON {
                lambda0 -= self.sign0 * EPSILON;
            }
            if (lambda1 - sign1).abs() < EPSILON {
                lambda1 -= sign1 * EPSILON;
            }
            let phi_i = antimeridian_intersect(lambda0, self.phi0, lambda1, phi1);
            self.sink.point(self.sign0, phi_i, 0);
            self.sink.line_end();
            self.sink.line_start();
            self.sink.point(sign1, phi_i, 0);
            self.clean = 0;
        }

        self.sink.point(lambda1, phi1, m);
        self.lambda0 = lambda1;
        self.phi0 = phi1;
        self.sign0 = sign1;
    }

    fn line_end(&mut self) {
        self.sink.line_end();
        self.lambda0 = f64::NAN;
        self.phi0 = f64::NAN;
        self.sign0 = f64::NAN;
    }

    fn clean(&self) -> i32 {
        2 - self.clean
    }
}

// -----------------------------------------------------------------------------
// Circle clip line
// -----------------------------------------------------------------------------

fn circle_visible(lambda: f64, phi: f64, cr: f64) -> bool {
    lambda.cos() * phi.cos() > cr
}

fn circle_code(lambda: f64, phi: f64, r: f64) -> u8 {
    let mut code = 0u8;
    if lambda < -r {
        code |= 1;
    } else if lambda > r {
        code |= 2;
    }
    if phi < -r {
        code |= 4;
    } else if phi > r {
        code |= 8;
    }
    code
}

fn circle_intersect(a: (f64, f64), b: (f64, f64), cr: f64, two: bool) -> Option<[(f64, f64); 2]> {
    let pa = cartesian(a.0, a.1);
    let pb = cartesian(b.0, b.1);

    let n1 = [1.0, 0.0, 0.0];
    let n2 = cartesian_cross(pa, pb);
    let n2n2 = cartesian_dot(n2, n2);
    let n1n2 = n2[0];
    let determinant = n2n2 - n1n2 * n1n2;

    if determinant.abs() < EPSILON2 {
        if !two {
            return Some([a, a]);
        }
        return None;
    }

    let c1 = cr * n2n2 / determinant;
    let c2 = -cr * n1n2 / determinant;
    let n1xn2 = cartesian_cross(n1, n2);
    let mut a_vec = cartesian_scale(n1, c1);
    let b_vec = cartesian_scale(n2, c2);
    cartesian_add_in_place(&mut a_vec, b_vec);

    let u = n1xn2;
    let w = cartesian_dot(a_vec, u);
    let uu = cartesian_dot(u, u);
    let t2 = w * w - uu * (cartesian_dot(a_vec, a_vec) - 1.0);

    if t2 < 0.0 {
        return None;
    }

    let t = t2.sqrt();
    let mut q_vec = cartesian_scale(u, (-w - t) / uu);
    cartesian_add_in_place(&mut q_vec, a_vec);
    let q = spherical(q_vec);

    if !two {
        return Some([q, q]);
    }

    let mut q1_vec = cartesian_scale(u, (-w + t) / uu);
    cartesian_add_in_place(&mut q1_vec, a_vec);
    let q1 = spherical(q1_vec);

    let mut lambda0 = a.0;
    let mut lambda1 = b.0;
    let mut phi0 = a.1;
    let mut phi1 = b.1;
    let mut z;

    if lambda1 < lambda0 {
        z = lambda0;
        lambda0 = lambda1;
        lambda1 = z;
    }

    let delta = lambda1 - lambda0;
    let polar = (delta - PI).abs() < EPSILON;
    let meridian = polar || delta < EPSILON;

    if !polar && phi1 < phi0 {
        z = phi0;
        phi0 = phi1;
        phi1 = z;
    }

    let on_arc = if meridian {
        if polar {
            (phi0 + phi1 > 0.0)
                ^ (q.1
                    < if (q.0 - lambda0).abs() < EPSILON {
                        phi0
                    } else {
                        phi1
                    })
        } else {
            phi0 <= q.1 && q.1 <= phi1
        }
    } else {
        (delta > PI) ^ (lambda0 <= q.0 && q.0 <= lambda1)
    };

    if on_arc { Some([q, q1]) } else { None }
}

struct CircleClipLine<S: Sink> {
    sink: S,
    cr: f64,
    radius: f64,
    small_radius: bool,
    not_hemisphere: bool,
    point0: Option<(f64, f64)>,
    c0: u8,
    v0: bool,
    v00: bool,
    clean: i32,
}

impl<S: Sink> CircleClipLine<S> {
    fn new(sink: S, radius: f64) -> Self {
        let cr = radius.cos();
        Self {
            sink,
            cr,
            radius,
            small_radius: cr > 0.0,
            not_hemisphere: cr.abs() > EPSILON,
            point0: None,
            c0: 0,
            v0: false,
            v00: false,
            clean: 1,
        }
    }

    fn line_start(&mut self) {
        self.v00 = false;
        self.v0 = false;
        self.clean = 1;
        self.point0 = None;
        // The sink line is started lazily when the first visible point is seen,
        // matching d3-geo's clipCircle behaviour.
    }

    fn visible(&self, lambda: f64, phi: f64) -> bool {
        circle_visible(lambda, phi, self.cr)
    }

    fn code(&self, lambda: f64, phi: f64, v: bool) -> u8 {
        if self.small_radius {
            if v {
                0
            } else {
                circle_code(lambda, phi, self.radius)
            }
        } else if v {
            circle_code(
                lambda + if lambda < 0.0 { PI } else { -PI },
                phi,
                PI - self.radius,
            )
        } else {
            0
        }
    }

    fn point(&mut self, lambda1: f64, phi1: f64, m: i32) {
        let point1 = (lambda1, phi1);
        let v = self.visible(lambda1, phi1);
        let c = self.code(lambda1, phi1, v);

        if self.point0.is_none() {
            self.v00 = v;
            self.v0 = v;
            if v {
                self.sink.line_start();
            }
        }

        if v != self.v0 {
            let mut point1m = [lambda1, phi1, m as f64];
            let point2 = circle_intersect(self.point0.unwrap(), point1, self.cr, false);
            if point2.is_none()
                || point_equal(self.point0.unwrap(), point2.unwrap()[0])
                || point_equal(point1, point2.unwrap()[0])
            {
                point1m[2] = 1.0;
            }

            self.clean = 0;
            if v {
                // Outside going in.
                self.sink.line_start();
                if let Some(p2) = circle_intersect(point1, self.point0.unwrap(), self.cr, false) {
                    self.sink.point(p2[0].0, p2[0].1, 0);
                }
            } else {
                // Inside going out.
                if let Some(p2) = point2 {
                    self.sink.point(p2[0].0, p2[0].1, 2);
                }
                self.sink.line_end();
                self.sink.line_start();
            }
            self.point0 = point2.map(|p| p[0]).or(Some(point1));
        } else if self.not_hemisphere
            && self.point0.is_some()
            && (self.small_radius ^ v)
            && (c & self.c0) == 0
        {
            // Segment may intersect the small circle even without a visibility change.
            let t = circle_intersect(point1, self.point0.unwrap(), self.cr, true);
            if let Some(t) = t {
                self.clean = 0;
                if self.small_radius {
                    self.sink.line_start();
                    self.sink.point(t[0].0, t[0].1, 0);
                    self.sink.point(t[1].0, t[1].1, 0);
                    self.sink.line_end();
                } else {
                    self.sink.point(t[1].0, t[1].1, 0);
                    self.sink.line_end();
                    self.sink.line_start();
                    self.sink.point(t[0].0, t[0].1, 3);
                }
            }
        }

        if v && (self.point0.is_none() || !point_equal(self.point0.unwrap(), point1)) {
            self.sink.point(lambda1, phi1, m);
        }

        self.point0 = Some(point1);
        self.v0 = v;
        self.c0 = c;
    }

    fn line_end(&mut self) {
        if self.v0 {
            self.sink.line_end();
        }
        self.point0 = None;
    }

    fn clean(&self) -> i32 {
        self.clean | ((self.v00 && self.v0) as i32) << 1
    }
}

// -----------------------------------------------------------------------------
// Polygon contains (for startInside)
// -----------------------------------------------------------------------------

struct Adder {
    sum: f64,
    c: f64,
}

impl Adder {
    fn new() -> Self {
        Self { sum: 0.0, c: 0.0 }
    }

    fn add(&mut self, x: f64) {
        let y = x - self.c;
        let t = self.sum + y;
        self.c = (t - self.sum) - y;
        self.sum = t;
    }

    fn value(&self) -> f64 {
        self.sum
    }
}

fn polygon_contains(polygon: &[Vec<(f64, f64)>], point: (f64, f64)) -> bool {
    let lambda = longitude(point.0);
    let phi = point.1;
    let sin_phi = phi.sin();
    let normal = [lambda.sin(), -lambda.cos(), 0.0];

    let mut angle = 0.0;
    let mut winding = 0i32;
    let mut sum = Adder::new();

    let phi_eps = if (sin_phi - 1.0).abs() < EPSILON {
        HALF_PI + EPSILON
    } else if (sin_phi + 1.0).abs() < EPSILON {
        -HALF_PI - EPSILON
    } else {
        phi
    };

    for ring in polygon {
        let m = ring.len();
        if m == 0 {
            continue;
        }

        let mut point0 = ring[m - 1];
        let mut lambda0 = longitude(point0.0);
        let mut phi0 = point0.1 / 2.0 + QUARTER_PI;
        let mut sin_phi0 = phi0.sin();
        let mut cos_phi0 = phi0.cos();

        for j in 0..m {
            let point1 = ring[j];
            let lambda1 = longitude(point1.0);
            let phi1 = point1.1 / 2.0 + QUARTER_PI;
            let sin_phi1 = phi1.sin();
            let cos_phi1 = phi1.cos();
            let delta = lambda1 - lambda0;
            let s = sign(delta);
            let abs_delta = s * delta;
            let antimeridian = abs_delta > PI;
            let k = sin_phi0 * sin_phi1;

            sum.add((k * s * abs_delta.sin()).atan2(cos_phi0 * cos_phi1 + k * abs_delta.cos()));
            angle += if antimeridian { delta + s * TAU } else { delta };

            if antimeridian ^ (lambda0 >= lambda) ^ (lambda1 >= lambda) {
                let mut arc =
                    cartesian_cross(cartesian(point0.0, point0.1), cartesian(point1.0, point1.1));
                cartesian_normalize_in_place(&mut arc);
                let mut intersection = cartesian_cross(normal, arc);
                cartesian_normalize_in_place(&mut intersection);
                let phi_arc = (if antimeridian ^ (delta >= 0.0) {
                    -1.0
                } else {
                    1.0
                }) * asin_clamped(intersection[2]);
                if phi_eps > phi_arc || (phi_eps == phi_arc && (arc[0] != 0.0 || arc[1] != 0.0)) {
                    winding += if antimeridian ^ (delta >= 0.0) { 1 } else { -1 };
                }
            }

            point0 = point1;
            lambda0 = lambda1;
            phi0 = phi1;
            sin_phi0 = sin_phi1;
            cos_phi0 = cos_phi1;
        }
    }

    (angle < -EPSILON || (angle < EPSILON && sum.value() < -EPSILON2)) ^ (winding & 1 != 0)
}

// -----------------------------------------------------------------------------
// Rejoin clipped segments into closed polygon rings.
// -----------------------------------------------------------------------------

#[derive(Clone)]
struct Intersection {
    x: (f64, f64),
    z: Option<Vec<[f64; 3]>>,
    o: usize,
    e: bool,
    v: bool,
    n: usize,
    p: usize,
}

fn compare_intersection(a: (f64, f64), b: (f64, f64)) -> std::cmp::Ordering {
    let av = if a.0 < 0.0 {
        a.1 - HALF_PI - EPSILON
    } else {
        HALF_PI - a.1
    };
    let bv = if b.0 < 0.0 {
        b.1 - HALF_PI - EPSILON
    } else {
        HALF_PI - b.1
    };
    av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
}

fn rejoin_segments(
    segments: &[Vec<[f64; 3]>],
    mut start_inside: bool,
    mut interpolate: impl FnMut((f64, f64), (f64, f64), f64, &mut Vec<(f64, f64)>),
) -> Vec<Vec<(f64, f64)>> {
    // segments are kept as [lambda, phi, m] so that intersection markers are
    // available; only the first two coordinates are used for geometry.
    if segments.is_empty() {
        return Vec::new();
    }

    let mut subject: Vec<Intersection> = Vec::new();
    let mut clip: Vec<Intersection> = Vec::new();

    for segment in segments {
        let n = segment.len();
        if n < 2 {
            continue;
        }
        let p0 = (segment[0][0], segment[0][1]);
        let p1 = (segment[n - 1][0], segment[n - 1][1]);

        if point_equal(p0, p1) && segment[0][2] == 0.0 && segment[n - 1][2] == 0.0 {
            // Degenerate closed visible segment.
            continue;
        }

        let mut p1 = p1;
        if point_equal(p0, p1) {
            p1.0 += 2.0 * EPSILON;
        }

        let si = subject.len();
        let ci = clip.len();
        let seg_z = Some(segment.clone());
        subject.push(Intersection {
            x: p0,
            z: seg_z.clone(),
            o: ci,
            e: true,
            v: false,
            n: 0,
            p: 0,
        });
        clip.push(Intersection {
            x: p0,
            z: None,
            o: si,
            e: false,
            v: false,
            n: 0,
            p: 0,
        });

        let si2 = subject.len();
        let ci2 = clip.len();
        subject.push(Intersection {
            x: p1,
            z: seg_z,
            o: ci2,
            e: false,
            v: false,
            n: 0,
            p: 0,
        });
        clip.push(Intersection {
            x: p1,
            z: None,
            o: si2,
            e: true,
            v: false,
            n: 0,
            p: 0,
        });
    }

    if subject.is_empty() {
        return Vec::new();
    }

    let mut clip_order: Vec<usize> = (0..clip.len()).collect();
    clip_order.sort_by(|&a, &b| compare_intersection(clip[a].x, clip[b].x));

    for i in 0..subject.len() {
        subject[i].n = (i + 1) % subject.len();
        subject[i].p = (i + subject.len() - 1) % subject.len();
    }

    for i in 0..clip_order.len() {
        let idx = clip_order[i];
        let next = clip_order[(i + 1) % clip_order.len()];
        let prev = clip_order[(i + clip_order.len() - 1) % clip_order.len()];
        clip[idx].n = next;
        clip[idx].p = prev;
    }

    for i in 0..clip_order.len() {
        let idx = clip_order[i];
        start_inside = !start_inside;
        clip[idx].e = start_inside;
    }

    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
    let start = 0usize;

    loop {
        let mut current = start;
        let mut is_subject = true;
        while subject[current].v {
            current = subject[current].n;
            if current == start {
                return rings;
            }
        }

        let mut ring: Vec<(f64, f64)> = Vec::new();
        loop {
            if is_subject {
                subject[current].v = true;
                clip[subject[current].o].v = true;
            } else {
                subject[clip[current].o].v = true;
                clip[current].v = true;
            }

            let e = if is_subject {
                subject[current].e
            } else {
                clip[current].e
            };

            if e {
                if is_subject {
                    if let Some(points) = &subject[current].z {
                        for pt in points {
                            ring.push((pt[0], pt[1]));
                        }
                    }
                    current = subject[current].n;
                } else {
                    let next_x = clip[clip[current].n].x;
                    let x = clip[current].x;
                    interpolate(x, next_x, 1.0, &mut ring);
                    current = clip[current].n;
                }
            } else {
                if is_subject {
                    let prev = subject[current].p;
                    if let Some(points) = &subject[prev].z {
                        for i in (0..points.len()).rev() {
                            ring.push((points[i][0], points[i][1]));
                        }
                    }
                    current = prev;
                } else {
                    let prev_x = clip[clip[current].p].x;
                    let x = clip[current].x;
                    interpolate(x, prev_x, -1.0, &mut ring);
                    current = clip[current].p;
                }
            }

            current = if is_subject {
                subject[current].o
            } else {
                clip[current].o
            };
            is_subject = !is_subject;

            let done = if is_subject {
                subject[current].v
            } else {
                clip[current].v
            };
            if done {
                break;
            }
        }

        if ring.len() > 1 {
            rings.push(ring);
        }
    }
}

// -----------------------------------------------------------------------------
// Interpolation along clip edges
// -----------------------------------------------------------------------------

fn interpolate_antimeridian(
    from: (f64, f64),
    to: (f64, f64),
    direction: f64,
    out: &mut Vec<(f64, f64)>,
) {
    if (from.0 - to.0).abs() > EPSILON {
        let lambda = if from.0 < to.0 { PI } else { -PI };
        let phi = direction * lambda / 2.0;
        out.push((-lambda, phi));
        out.push((0.0, phi));
        out.push((lambda, phi));
    } else {
        out.push((to.0, to.1));
    }
}

fn interpolate_circle(
    radius: f64,
    from: (f64, f64),
    to: (f64, f64),
    direction: f64,
    out: &mut Vec<(f64, f64)>,
) {
    let arc = circle_stream_points(radius, 2.0_f64.to_radians(), direction, from, to);
    out.extend(arc);
}

// -----------------------------------------------------------------------------
// Top-level helpers
// -----------------------------------------------------------------------------

/// Remove a trailing duplicate of the first vertex, matching d3's `ring.pop()`.
fn ring_without_duplicate(ring: &[(f64, f64)]) -> &[(f64, f64)] {
    let n = ring.len();
    if n > 1 && ring[0] == ring[n - 1] {
        &ring[..n - 1]
    } else {
        ring
    }
}

fn clip_antimeridian_line(coords: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let mut sink = OutSink::new();
    {
        let mut line = AntimeridianClipLine::new(&mut sink);
        line.line_start();
        for &(lambda, phi) in coords {
            line.point(lambda, phi, 0);
        }
        line.line_end();
    }
    sink.result()
}

fn clip_circle_line(coords: &[(f64, f64)], radius: f64) -> Vec<Vec<(f64, f64)>> {
    let mut sink = OutSink::new();
    {
        let mut line = CircleClipLine::new(&mut sink, radius);
        line.line_start();
        for &(lambda, phi) in coords {
            line.point(lambda, phi, 0);
        }
        line.line_end();
    }
    sink.result()
}

fn clip_antimeridian_ring(coords: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let coords = ring_without_duplicate(coords);

    let mut buffer = ClipBuffer::new();
    let clean = {
        let mut line = AntimeridianClipLine::new(&mut buffer);
        line.line_start();
        for &(lambda, phi) in coords {
            line.point(lambda, phi, 0);
        }
        // Close the ring by re-adding the first point.
        if let Some(&(lambda, phi)) = coords.first() {
            line.point(lambda, phi, 0);
        }
        line.line_end();
        line.clean()
    };

    let mut segments = buffer.result();

    // Rejoin first and last segments when the ring crossed the clip edge and
    // the first/last original points were visible.
    if !segments.is_empty() && (clean & 2) != 0 {
        let first = segments.remove(0);
        if let Some(last) = segments.last_mut() {
            last.extend(first);
        } else {
            segments.push(first);
        }
    }

    if (clean & 1) != 0 && !segments.is_empty() {
        // No intersections: the whole ring is visible.
        let seg = segments.pop().unwrap();
        let ring: Vec<(f64, f64)> = seg.into_iter().map(|p| (p[0], p[1])).collect();
        return vec![ring];
    }

    let polygon: Vec<Vec<(f64, f64)>> = vec![coords.to_vec()];
    let start = (-PI, -HALF_PI);
    let start_inside = polygon_contains(&polygon, start);

    if segments.is_empty() {
        // Entire ring invisible; if it contains the start point, emit the full
        // antimeridian boundary ring so that the polygon interior is preserved.
        if start_inside {
            return vec![antimeridian_boundary_ring(1.0)];
        }
        return Vec::new();
    }

    rejoin_segments(&segments, start_inside, |from, to, dir, out| {
        interpolate_antimeridian(from, to, dir, out)
    })
}

fn antimeridian_boundary_ring(direction: f64) -> Vec<(f64, f64)> {
    let mut ring = Vec::new();
    let phi = direction * HALF_PI;
    ring.push((-PI, phi));
    ring.push((0.0, phi));
    ring.push((PI, phi));
    ring.push((PI, 0.0));
    ring.push((PI, -phi));
    ring.push((0.0, -phi));
    ring.push((-PI, -phi));
    ring.push((-PI, 0.0));
    ring.push((-PI, phi));
    ring
}

fn clip_circle_ring(coords: &[(f64, f64)], radius: f64) -> Vec<Vec<(f64, f64)>> {
    let coords = ring_without_duplicate(coords);

    let cr = radius.cos();
    let small_radius = cr > 0.0;

    let mut buffer = ClipBuffer::new();
    let clean = {
        let mut line = CircleClipLine::new(&mut buffer, radius);
        line.line_start();
        for &(lambda, phi) in coords {
            line.point(lambda, phi, 0);
        }
        if let Some(&(lambda, phi)) = coords.first() {
            line.point(lambda, phi, 0);
        }
        line.line_end();
        line.clean()
    };

    let mut segments = buffer.result();

    if !segments.is_empty() && (clean & 2) != 0 {
        let first = segments.remove(0);
        if let Some(last) = segments.last_mut() {
            last.extend(first);
        } else {
            segments.push(first);
        }
    }

    if (clean & 1) != 0 && !segments.is_empty() {
        let seg = segments.pop().unwrap();
        let ring: Vec<(f64, f64)> = seg.into_iter().map(|p| (p[0], p[1])).collect();
        return vec![ring];
    }

    let polygon: Vec<Vec<(f64, f64)>> = vec![coords.to_vec()];
    let start = if small_radius {
        (0.0, -radius)
    } else {
        (-PI, radius - PI)
    };
    let start_inside = polygon_contains(&polygon, start);

    if segments.is_empty() {
        if start_inside {
            let direction = 1.0;
            let arc = circle_stream_full(radius, 2.0_f64.to_radians(), direction);
            return vec![arc];
        }
        return Vec::new();
    }

    rejoin_segments(&segments, start_inside, move |from, to, dir, out| {
        interpolate_circle(radius, from, to, dir, out)
    })
}

// -----------------------------------------------------------------------------
// Public clip functions
// -----------------------------------------------------------------------------

/// Clip a line string or polygon ring against the antimeridian in the
/// projection's rotated frame, returning geographic-coordinate pieces.
pub fn clip_antimeridian(
    ring_or_line: &[(f64, f64)],
    is_ring: bool,
    rotation: &SphereRotation,
) -> Vec<Vec<(f64, f64)>> {
    let rotated: Vec<(f64, f64)> = ring_or_line
        .iter()
        .map(|&(lon, lat)| rotate_point_rad(rotation, lon, lat))
        .collect();

    if is_ring {
        clip_antimeridian_ring(&rotated)
    } else {
        clip_antimeridian_line(&rotated)
    }
}

/// Clip all rings of a polygon against the antimeridian in the projection's
/// rotated frame, treating the first ring as the exterior and subsequent rings
/// as holes. Returns geographic-coordinate rings.
pub fn clip_antimeridian_polygon(
    rings: &[Vec<(f64, f64)>],
    rotation: &SphereRotation,
) -> Vec<Vec<(f64, f64)>> {
    let trimmed_rings: Vec<Vec<(f64, f64)>> = rings
        .iter()
        .map(|ring| {
            let n = ring.len();
            if n > 1 && ring[0] == ring[n - 1] {
                ring[..n - 1].to_vec()
            } else {
                ring.clone()
            }
        })
        .collect();

    let rotated_rings: Vec<Vec<(f64, f64)>> = trimmed_rings
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|&(lon, lat)| rotate_point_rad(rotation, lon, lat))
                .collect()
        })
        .collect();

    let mut output_rings: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut all_segments: Vec<Vec<[f64; 3]>> = Vec::new();

    for ring in &rotated_rings {
        let mut buffer = ClipBuffer::new();
        let clean = {
            let mut line = AntimeridianClipLine::new(&mut buffer);
            line.line_start();
            for &(lambda, phi) in ring {
                line.point(lambda, phi, 0);
            }
            if let Some(&(lambda, phi)) = ring.first() {
                line.point(lambda, phi, 0);
            }
            line.line_end();
            line.clean()
        };

        let mut segments = buffer.result();
        if !segments.is_empty() && (clean & 2) != 0 {
            let first = segments.remove(0);
            if let Some(last) = segments.last_mut() {
                last.extend(first);
            } else {
                segments.push(first);
            }
        }

        if (clean & 1) != 0 && !segments.is_empty() {
            // No intersections: the whole ring is visible.
            let ring: Vec<(f64, f64)> = segments
                .pop()
                .unwrap()
                .into_iter()
                .map(|p| (p[0], p[1]))
                .collect();
            output_rings.push(ring);
        } else {
            all_segments.extend(segments.into_iter().filter(|s| s.len() > 1));
        }
    }

    let start = (-PI, -HALF_PI);
    let start_inside = polygon_contains(&rotated_rings, start);

    if !all_segments.is_empty() {
        let pieces = rejoin_segments(&all_segments, start_inside, |from, to, dir, out| {
            interpolate_antimeridian(from, to, dir, out)
        });
        output_rings.extend(pieces);
    } else if start_inside {
        // The polygon contains the clip start point and no ring intersected the
        // clip edge. This is a full-sphere polygon (e.g. the ocean ring in
        // Natural Earth data); emit the entire boundary ring so its bounds cover
        // the world.
        output_rings.push(antimeridian_boundary_ring(1.0));
    }

    output_rings
}

/// Clip a line string or polygon ring against a spherical cap centered at the
/// origin in the rotated frame, returning rotated-coordinate pieces.
pub fn clip_circle(
    ring_or_line: &[(f64, f64)],
    is_ring: bool,
    rotation: &SphereRotation,
    clip_angle_rad: f64,
) -> Vec<Vec<(f64, f64)>> {
    let rotated: Vec<(f64, f64)> = ring_or_line
        .iter()
        .map(|&(lon, lat)| rotate_point_rad(rotation, lon, lat))
        .collect();

    if is_ring {
        clip_circle_ring(&rotated, clip_angle_rad)
    } else {
        clip_circle_line(&rotated, clip_angle_rad)
    }
}

/// Clip all rings of a polygon against a spherical cap centered at the origin
/// in the rotated frame, treating the first ring as the exterior and
/// subsequent rings as holes. Returns geographic-coordinate rings.
pub fn clip_circle_polygon(
    rings: &[Vec<(f64, f64)>],
    rotation: &SphereRotation,
    clip_angle_rad: f64,
) -> Vec<Vec<(f64, f64)>> {
    let trimmed_rings: Vec<Vec<(f64, f64)>> = rings
        .iter()
        .map(|ring| {
            let n = ring.len();
            if n > 1 && ring[0] == ring[n - 1] {
                ring[..n - 1].to_vec()
            } else {
                ring.clone()
            }
        })
        .collect();

    let rotated_rings: Vec<Vec<(f64, f64)>> = trimmed_rings
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|&(lon, lat)| rotate_point_rad(rotation, lon, lat))
                .collect()
        })
        .collect();

    let mut output_rings: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut all_segments: Vec<Vec<[f64; 3]>> = Vec::new();

    for ring in &rotated_rings {
        let mut buffer = ClipBuffer::new();
        let clean = {
            let mut line = CircleClipLine::new(&mut buffer, clip_angle_rad);
            line.line_start();
            for &(lambda, phi) in ring {
                line.point(lambda, phi, 0);
            }
            if let Some(&(lambda, phi)) = ring.first() {
                line.point(lambda, phi, 0);
            }
            line.line_end();
            line.clean()
        };

        let mut segments = buffer.result();
        if !segments.is_empty() && (clean & 2) != 0 {
            let first = segments.remove(0);
            if let Some(last) = segments.last_mut() {
                last.extend(first);
            } else {
                segments.push(first);
            }
        }

        if (clean & 1) != 0 && !segments.is_empty() {
            let seg = segments.pop().unwrap();
            let ring: Vec<(f64, f64)> = seg.into_iter().map(|p| (p[0], p[1])).collect();
            output_rings.push(ring);
        } else {
            all_segments.extend(segments.into_iter().filter(|s| s.len() > 1));
        }
    }

    let cr = clip_angle_rad.cos();
    let small_radius = cr > 0.0;
    let start = if small_radius {
        (0.0, -clip_angle_rad)
    } else {
        (-PI, clip_angle_rad - PI)
    };
    let start_inside = polygon_contains(&rotated_rings, start);

    if !all_segments.is_empty() {
        let pieces = rejoin_segments(&all_segments, start_inside, move |from, to, dir, out| {
            interpolate_circle(clip_angle_rad, from, to, dir, out)
        });
        output_rings.extend(pieces);
    } else if start_inside && output_rings.is_empty() {
        let direction = 1.0;
        let arc = circle_stream_full(clip_angle_rad, 2.0_f64.to_radians(), direction);
        output_rings.push(arc);
    }

    output_rings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn antimeridian_line_crossing() {
        let rotation = SphereRotation::from_degrees(0.0, 0.0, 0.0);
        // A line crossing the antimeridian at the equator is split into two
        // visible segments.
        let line = vec![(170.0, 0.0), (-170.0, 0.0)];
        let pieces = clip_antimeridian(&line, false, &rotation);
        assert_eq!(pieces.len(), 2);
    }

    #[test]
    fn circle_ring_visible() {
        let rotation = SphereRotation::from_degrees(0.0, 0.0, 0.0);
        let ring = vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 0.0),
        ];
        let pieces = clip_circle(&ring, true, &rotation, 90.0_f64.to_radians());
        assert!(!pieces.is_empty());
    }

    #[test]
    fn antimeridian_high_latitude_rectangle() {
        let rotation = SphereRotation::from_degrees(0.0, 0.0, 0.0);
        let ring = vec![
            (170.0, 80.0),
            (-170.0, 80.0),
            (-170.0, 70.0),
            (170.0, 70.0),
            (170.0, 80.0),
        ];
        let pieces = clip_antimeridian_polygon(&[ring], &rotation);
        // D3 leaves this as two separate rings; our clipper should do the same.
        assert_eq!(pieces.len(), 2);
    }

    #[test]
    fn circle_polygon_outside() {
        let rotation = SphereRotation::from_degrees(0.0, 0.0, 0.0);
        // A square outside a 10° clip cap that contains the start point is
        // emitted as the full clip circle by d3-geo.
        let ring = vec![
            (-20.0, 20.0),
            (20.0, 20.0),
            (20.0, -20.0),
            (-20.0, -20.0),
            (-20.0, 20.0),
        ];
        let pieces = clip_circle_polygon(&[ring], &rotation, 10.0_f64.to_radians());
        assert_eq!(pieces.len(), 1);
        assert!(pieces[0].len() > 100);
    }

    #[test]
    fn circle_polygon_straddling_limb() {
        let rotation = SphereRotation::from_degrees(0.0, 0.0, 0.0);
        // A rectangle crossing a 20° clip cap that does NOT contain the center.
        let ring = vec![
            (10.0, 30.0),
            (30.0, 30.0),
            (30.0, 10.0),
            (10.0, 10.0),
            (10.0, 30.0),
        ];
        let pieces = clip_circle_polygon(&[ring], &rotation, 20.0_f64.to_radians());
        println!("circle straddle pieces: {}", pieces.len());
        for (i, piece) in pieces.iter().enumerate() {
            println!(
                "piece {}: len={} first={:?} last={:?}",
                i,
                piece.len(),
                piece.first(),
                piece.last()
            );
        }
        assert!(!pieces.is_empty());
    }

    #[test]
    fn land_polygon_contains_orthographic_start() {
        use crate::geo::GeoJsonGeometry;
        use crate::geo::topojson::parse_land;
        let json = include_str!("../../../bin/showcase/data/land-50m.json");
        let land = parse_land(json).expect("parse land");
        let rotation = SphereRotation::from_degrees(45.0, -15.0, 0.0);
        let clip_angle = (90.0_f64 + 1e-6).to_radians();
        let start = (-PI, clip_angle - PI);
        let mut count = 0;
        if let GeoJsonGeometry::MultiPolygon(polygons) = land {
            for rings in &polygons {
                let rotated: Vec<Vec<(f64, f64)>> = rings
                    .iter()
                    .map(|ring| {
                        ring.iter()
                            .map(|&(lon, lat)| rotate_point_rad(&rotation, lon, lat))
                            .collect()
                    })
                    .collect();
                if polygon_contains(&rotated, start) {
                    count += 1;
                }
            }
        }
        eprintln!("Rust contains count: {}", count);
        assert_eq!(count, 0);
    }

    #[test]
    fn debug_land_polygon_93_clip() {
        use crate::geo::GeoJsonGeometry;
        use crate::geo::topojson::parse_land;
        let json = include_str!("../../../bin/showcase/data/land-50m.json");
        let land = parse_land(json).expect("parse land");
        let rotation = SphereRotation::from_degrees(45.0, -15.0, 0.0);
        let clip_angle = (90.0_f64 + 1e-6).to_radians();
        if let GeoJsonGeometry::MultiPolygon(polygons) = land {
            let rings = &polygons[93];
            let pieces = clip_circle_polygon(rings, &rotation, clip_angle);
            println!("polygon 93 pieces: {}", pieces.len());
            for (i, piece) in pieces.iter().enumerate() {
                println!(
                    "piece {}: len={} first={:?} last={:?}",
                    i,
                    piece.len(),
                    piece.first(),
                    piece.last()
                );
                println!("  first 10: {:?}", &piece[..piece.len().min(10)]);
                println!("  last 10: {:?}", &piece[piece.len().saturating_sub(10)..]);
            }
        }
    }

    #[test]
    fn debug_antarctica_conic180_60() {
        use crate::geo::ConicEqualArea;
        use crate::geo::GeoJsonGeometry;
        use crate::geo::GeoPath;
        use crate::geo::Projection;
        use crate::geo::topojson::parse_land;
        let json = include_str!("../../../bin/showcase/data/land-50m.json");
        let land = parse_land(json).expect("parse land");
        if let GeoJsonGeometry::MultiPolygon(polygons) = land {
            let geom = GeoJsonGeometry::Polygon(polygons[1379].clone());
            let proj = ConicEqualArea::new()
                .scale(100.0)
                .translate(0.0, 0.0)
                .center(0.0, 0.0)
                .rotate(180.0, -60.0, 0.0)
                .parallels(29.5, 45.5);
            let path = GeoPath::new(proj.clone());
            let b = path.bounds(&geom);
            println!("poly1379 bounds rotate 180,-60: {:?}", b);
            let s = path.render(&geom);
            let nums: Vec<f64> = s
                .split(|c: char| {
                    !c.is_ascii_digit() && c != '.' && c != '-' && c != '+' && c != 'e' && c != 'E'
                })
                .filter(|t| !t.is_empty())
                .map(|t| t.parse().unwrap())
                .collect();
            let (mut miny, mut maxx) = (f64::INFINITY, f64::NEG_INFINITY);
            let (mut pmin_render, mut pmax_render) = ((0.0, 0.0), (0.0, 0.0));
            for i in (0..nums.len()).step_by(2) {
                if i + 1 >= nums.len() {
                    break;
                }
                let (x, y) = (nums[i], nums[i + 1]);
                if y < miny {
                    miny = y;
                    pmin_render = (x, y);
                }
                if x > maxx {
                    maxx = x;
                    pmax_render = (x, y);
                }
            }
            println!(
                "render min y {:.9} at {:?} max x {:?}",
                miny, pmin_render, pmax_render
            );
            // find point in clipped piece with min projected y
            use crate::geo::path::clip::{clip_antimeridian_polygon, resample_spherical_line};
            use crate::geo::projection::SphereRotation;
            let rotation = SphereRotation::from_degrees(180.0, -60.0, 0.0);
            {
                use std::io::Write;
                let mut f = std::fs::File::create("/tmp/rust_rotated_1379_180_60.txt").unwrap();
                for (ri, ring) in polygons[1379].iter().enumerate() {
                    writeln!(f, "ring{}", ri).unwrap();
                    for &(lon, lat) in ring {
                        let (l, p) = rotation.rotate(lon.to_radians(), lat.to_radians());
                        writeln!(
                            f,
                            "{},{} -> {},{} (deg)",
                            lon,
                            lat,
                            l.to_degrees(),
                            p.to_degrees()
                        )
                        .unwrap();
                    }
                }
            }
            let pieces = clip_antimeridian_polygon(&polygons[1379], &rotation);
            {
                use std::io::Write;
                let mut f = std::fs::File::create("/tmp/rust_clipped_1379_180_60.txt").unwrap();
                for (pi, piece) in pieces.iter().enumerate() {
                    writeln!(f, "ring{}", pi).unwrap();
                    for &(l, p) in piece {
                        writeln!(f, "{},{} (deg)", l.to_degrees(), p.to_degrees()).unwrap();
                    }
                }
            }
            let (mut miny2, mut maxx2) = (f64::INFINITY, f64::NEG_INFINITY);
            let (mut pmin2, mut pmax2) = ((0.0, 0.0), (0.0, 0.0));
            for piece in &pieces {
                let pts = resample_spherical_line(piece, &|l, p| proj.project_rotated(l, p), true);
                for (x, y) in pts {
                    if y < miny2 {
                        miny2 = y;
                        pmin2 = (x, y);
                    }
                    if x > maxx2 {
                        maxx2 = x;
                        pmax2 = (x, y);
                    }
                }
            }
            println!(
                "resampled min y {:.9} at {:?} max x {:?}",
                miny2, pmin2, pmax2
            );
        }
    }

    #[test]
    fn debug_antarctica_conic() {
        use crate::geo::ConicEqualArea;
        use crate::geo::GeoJsonGeometry;
        use crate::geo::Projection;
        use crate::geo::topojson::parse_land;
        let json = include_str!("../../../bin/showcase/data/land-50m.json");
        let land = parse_land(json).expect("parse land");
        let proj = ConicEqualArea::new()
            .scale(100.0)
            .translate(0.0, 0.0)
            .center(0.0, 0.0)
            .rotate(0.0, -15.0, 0.0)
            .parallels(29.5, 45.5);
        if let GeoJsonGeometry::MultiPolygon(polygons) = land {
            let rings = &polygons[1379];
            let rotation = SphereRotation::from_degrees(0.0, -15.0, 0.0);
            let rotated_rings: Vec<Vec<(f64, f64)>> = rings
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|&(lon, lat)| rotate_point_rad(&rotation, lon, lat))
                        .collect()
                })
                .collect();
            // write rotated rings to file for comparison
            {
                use std::io::Write;
                let mut f = std::fs::File::create("/tmp/rust_rotated_1379.txt").unwrap();
                for (ri, ring) in rings.iter().enumerate() {
                    writeln!(f, "ring{}", ri).unwrap();
                    for (i, &(lon, lat)) in ring.iter().enumerate() {
                        let (l, p) = rotated_rings[ri][i];
                        writeln!(
                            f,
                            "{},{} -> {},{} (deg)",
                            lon,
                            lat,
                            l.to_degrees(),
                            p.to_degrees()
                        )
                        .unwrap();
                    }
                }
            }
            let start = (-std::f64::consts::PI, -std::f64::consts::PI / 2.0);
            println!(
                "antarctica start_inside={}",
                polygon_contains(&rotated_rings, start)
            );
            let pieces = clip_antimeridian_polygon(rings, &rotation);
            // compare with old antimeridian_clip_ring
            let old_pieces =
                crate::geo::path::geo_path::antimeridian_clip_ring(&rings[0], &rotation);
            println!("old antimeridian_clip_ring pieces: {}", old_pieces.len());
            let mut old_max_x = f64::NEG_INFINITY;
            for (i, piece) in old_pieces.iter().enumerate() {
                println!(
                    "old piece {} len={} first={:?} last={:?}",
                    i,
                    piece.len(),
                    piece.first(),
                    piece.last()
                );
                for &(lon, lat) in piece {
                    let (x, y) = proj.project(lon, lat);
                    if x > old_max_x {
                        old_max_x = x;
                    }
                }
            }
            println!("old max_x {}", old_max_x);
            // find projected extrema in piece
            let mut min_x = f64::INFINITY;
            let mut max_x = -f64::INFINITY;
            let mut min_p = (0.0, 0.0);
            let mut max_p = (0.0, 0.0);
            for &(l, p) in &pieces[0] {
                let (x, y) = proj.project_rotated(l, p);
                if p < -80.0_f64.to_radians() && l > 0.0 {
                    println!(
                        "high south pos l={:.3} p={:.3} -> x={:.3} y={:.3}",
                        l.to_degrees(),
                        p.to_degrees(),
                        x,
                        y
                    );
                }
                if x < min_x {
                    min_x = x;
                    min_p = (l, p);
                }
                if x > max_x {
                    max_x = x;
                    max_p = (l, p);
                }
            }
            println!(
                "piece extrema min_x={:.3} at raw ({:.4},{:.4}) max_x={:.3} at raw ({:.4},{:.4})",
                min_x,
                min_p.0.to_degrees(),
                min_p.1.to_degrees(),
                max_x,
                max_p.0.to_degrees(),
                max_p.1.to_degrees()
            );
            // write our clipped piece to file
            {
                use std::io::Write;
                let mut f = std::fs::File::create("/tmp/rust_clipped_1379.txt").unwrap();
                for (pi, piece) in pieces.iter().enumerate() {
                    writeln!(f, "ring{}", pi).unwrap();
                    for &(l, p) in piece {
                        writeln!(f, "{},{} (deg)", l.to_degrees(), p.to_degrees()).unwrap();
                    }
                }
            }
            println!("antarctica pieces: {}", pieces.len());
            let mut max_x = f64::NEG_INFINITY;
            let mut max_pt = (0.0, 0.0);
            for (i, piece) in pieces.iter().enumerate() {
                println!(
                    "piece {} len={} first={:?} last={:?}",
                    i,
                    piece.len(),
                    piece.first(),
                    piece.last()
                );
                let mut min_x = f64::MAX;
                let mut max_x_local = f64::NEG_INFINITY;
                for &(l, p) in piece {
                    let (x, y) = proj.project_rotated(l, p);
                    if x > max_x {
                        max_x = x;
                        max_pt = (l, p);
                    }
                    if x > max_x_local {
                        max_x_local = x;
                    }
                    if x < min_x {
                        min_x = x;
                    }
                }
                println!("  projected x range {} .. {}", min_x, max_x_local);
                println!("  extreme raw points:");
                for (k, &(l, p)) in piece.iter().enumerate() {
                    let (x, y) = proj.project_rotated(l, p);
                    if x > 240.0 || x < -240.0 {
                        println!(
                            "    k={} raw ({:.4},{:.4}) proj ({:.3},{:.3})",
                            k,
                            l.to_degrees(),
                            p.to_degrees(),
                            x,
                            y
                        );
                    }
                }
            }
            println!("our antarctica max_x {} at {:?}", max_x, max_pt);
            println!(
                "projected max_pt {:?}",
                proj.project_rotated(max_pt.0, max_pt.1)
            );
            // also render via GeoPath
            use crate::geo::GeoPath;
            let path = GeoPath::new(
                ConicEqualArea::new()
                    .scale(100.0)
                    .translate(0.0, 0.0)
                    .center(0.0, 0.0)
                    .rotate(0.0, -15.0, 0.0)
                    .parallels(29.5, 45.5),
            )
            .digits(3);
            let geom = GeoJsonGeometry::Polygon(polygons[1379].clone());
            println!("our path len {}", path.render(&geom).len());
            println!(
                "our path first 200: {}",
                &path.render(&geom)[..200.min(path.render(&geom).len())]
            );
        }
    }

    #[test]
    fn test_rotation_15_90() {
        use crate::geo::projection::SphereRotation;
        let rotation = SphereRotation::from_degrees(180.0, -60.0, 0.0);
        let (l, p) = rotation.rotate(0.0_f64.to_radians(), -90.0_f64.to_radians());
        println!(
            "rot(0,-90) = {:.6}, {:.6} deg",
            l.to_degrees(),
            p.to_degrees()
        );
    }

    #[test]
    fn test_conic_proj_point() {
        use crate::geo::ConicEqualArea;
        use crate::geo::Projection;
        let proj = ConicEqualArea::new()
            .scale(100.0)
            .translate(0.0, 0.0)
            .center(0.0, 0.0)
            .rotate(0.0, -15.0, 0.0)
            .parallels(29.5, 45.5);
        for &(l, p) in &[
            (77.2_f64.to_radians(), -84.09_f64.to_radians()),
            (45.0_f64.to_radians(), -84.0_f64.to_radians()),
            (77.0_f64.to_radians(), -80.0_f64.to_radians()),
        ] {
            let (x, y) = proj.project_rotated(l, p);
            println!(
                "proj_rotated({:.3},{:.3}) = ({:.3},{:.3})",
                l.to_degrees(),
                p.to_degrees(),
                x,
                y
            );
            // manual raw
            let phi0 = 29.5_f64.to_radians();
            let phi1 = 45.5_f64.to_radians();
            let sy0 = phi0.sin();
            let n = (sy0 + phi1.sin()) / 2.0;
            let c = 1.0 + sy0 * (2.0 * n - sy0);
            let r0 = c.sqrt() / n;
            let rho_sq = c - 2.0 * n * p.sin();
            let r = rho_sq.sqrt() / n;
            let theta = l * n;
            let xr = r * theta.sin();
            let yr = r0 - r * theta.cos();
            println!(
                "  manual raw n={:.6} c={:.6} r0={:.6} r={:.6} theta={:.6} -> ({:.6},{:.6})",
                n, c, r0, r, theta, xr, yr
            );
        }
    }

    #[test]
    fn debug_conic_polygon_1200() {
        use crate::geo::ConicEqualArea;
        use crate::geo::GeoJsonGeometry;
        use crate::geo::Projection;
        use crate::geo::topojson::parse_land;
        let proj0 = ConicEqualArea::new()
            .scale(100.0)
            .translate(0.0, 0.0)
            .center(0.0, 0.0)
            .rotate(60.0, -60.0, 0.0)
            .parallels(29.5, 45.5);
        let (x0, y0) = proj0.project(-60.0, -30.0);
        println!("DEBUG TOP proj(-60,-30) = {}, {}", x0, y0);
        let json = include_str!("../../../bin/showcase/data/land-50m.json");
        let land = parse_land(json).expect("parse land");
        let rotation = SphereRotation::from_degrees(60.0, -60.0, 0.0);
        if let GeoJsonGeometry::MultiPolygon(polygons) = land {
            let rings = &polygons[1200];
            let ring = ring_without_duplicate(&rings[0]);
            println!("trimmed ring len {}", ring.len());
            println!("first 3 rotated:");
            for &(lon, lat) in &ring[..3] {
                let (l, p) = rotation.rotate(radians(lon), radians(lat));
                println!(
                    "  geo ({:.4},{:.4}) -> rot ({:.4},{:.4})",
                    lon,
                    lat,
                    degrees(l),
                    degrees(p)
                );
            }
            println!("last 3 rotated:");
            for &(lon, lat) in &ring[ring.len() - 3..] {
                let (l, p) = rotation.rotate(radians(lon), radians(lat));
                println!(
                    "  geo ({:.4},{:.4}) -> rot ({:.4},{:.4})",
                    lon,
                    lat,
                    degrees(l),
                    degrees(p)
                );
            }
            let rotated: Vec<Vec<(f64, f64)>> = rings
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|&(lon, lat)| rotate_point_rad(&rotation, lon, lat))
                        .collect()
                })
                .collect();
            let start = (-PI, -HALF_PI);
            println!(
                "polygon 1200 contains start: {}",
                polygon_contains(&rotated, start)
            );
            // test direct line clip on closed rotated ring
            let mut closed_rotated = rotated[0].clone();
            if let Some(first) = closed_rotated.first().copied() {
                closed_rotated.push(first);
            }
            let line_segments = clip_antimeridian_line(&closed_rotated);
            println!("direct line segments: {}", line_segments.len());
            for (i, seg) in line_segments.iter().enumerate() {
                println!(
                    "seg {} len={} first={:?} last={:?}",
                    i,
                    seg.len(),
                    seg.first(),
                    seg.last()
                );
            }
            // test rejoin directly
            let segs3: Vec<Vec<[f64; 3]>> = line_segments
                .iter()
                .map(|s| s.iter().map(|&(x, y)| [x, y, 0.0]).collect())
                .collect();
            let rejoined = rejoin_segments(&segs3, true, |from, to, dir, out| {
                interpolate_antimeridian(from, to, dir, out)
            });
            println!("direct rejoined pieces: {}", rejoined.len());
            for (i, p) in rejoined.iter().enumerate() {
                println!(
                    "rejoined {} len={} first={:?} last={:?}",
                    i,
                    p.len(),
                    p.first(),
                    p.last()
                );
            }
            let pieces = clip_antimeridian_polygon(rings, &rotation);
            println!("polygon 1200 pieces: {}", pieces.len());
            let (lr, pr) = rotation.rotate(radians(-60.0), radians(-30.0));
            println!(
                "rotation of -60,-30 in debug = {}, {} deg",
                lr.to_degrees(),
                pr.to_degrees()
            );
            let (ip_l, ip_p) = rotation.invert(0.0, -HALF_PI);
            println!(
                "inverse south pole = {}, {} deg",
                ip_l.to_degrees(),
                ip_p.to_degrees()
            );
            let proj = ConicEqualArea::new()
                .scale(100.0)
                .translate(0.0, 0.0)
                .center(0.0, 0.0)
                .rotate(60.0, -60.0, 0.0)
                .parallels(29.5, 45.5);
            println!(
                "before max loop proj(-60,-30) = {:?}",
                proj.project(-60.0, -30.0)
            );
            println!(
                "before max loop proj(last-like) = {:?}",
                proj.project(-59.99999999999999, -30.000000000000014)
            );
            let (l1, p1) = rotation.rotate(radians(-60.0), radians(-30.0));
            let (l2, p2) =
                rotation.rotate(radians(-59.99999999999999), radians(-30.000000000000014));
            println!(
                "rot(-60,-30) = {}, {} deg",
                l1.to_degrees(),
                p1.to_degrees()
            );
            println!(
                "rot(last-like) = {}, {} deg",
                l2.to_degrees(),
                p2.to_degrees()
            );
            let mut max_y = f64::NEG_INFINITY;
            let mut max_pt = (0.0, 0.0);
            for piece in &pieces {
                for &(lon, lat) in piece {
                    let (x, y) = proj.project(lon, lat);
                    if y > max_y {
                        max_y = y;
                        max_pt = (lon, lat);
                    }
                }
            }
            println!("our max y {} at {:?}", max_y, max_pt);
            println!(
                "after max loop proj(-60,-30) = {:?}",
                proj.project(-60.0, -30.0)
            );
            for (i, piece) in pieces.iter().enumerate() {
                println!(
                    "inside loop start proj(-60,-30) = {:?}",
                    proj.project(-60.0, -30.0)
                );
                println!(
                    "piece {} len={} first={:?} last={:?}",
                    i,
                    piece.len(),
                    piece.first(),
                    piece.last()
                );
                println!("  first 5 {:?}", &piece[..5.min(piece.len())]);
                println!("  last 5 {:?}", &piece[piece.len().saturating_sub(5)..]);
                if let Some(last) = piece.last() {
                    let (x, y) = proj.project(last.0, last.1);
                    println!("  last {:?} projected {}, {}", last, x, y);
                    let (x2, y2) = proj.project(-59.99999999999999, -30.000000000000014);
                    println!("  direct -60,-30 projected {}, {}", x2, y2);
                }
                use std::io::Write;
                let mut f = std::fs::File::create("/tmp/our_piece_1200.json").unwrap();
                for (j, &(l, p)) in piece.iter().enumerate() {
                    if j > 0 {
                        write!(f, ",").unwrap();
                    }
                    write!(f, "[{},{}]", l, p).unwrap();
                }
                writeln!(f).unwrap();
            }
        }
    }
}
