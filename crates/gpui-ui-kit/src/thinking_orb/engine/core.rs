//! Shared primitives for the dotted 3D thought-orbs: honestly 3D — rotated,
//! depth-shaded, z-sorted. Depth is carried by dot size and ink weight alone.
//! Ported from `thinking-orbs` 0.3.1 `engine/core.ts`, MIT © Jakub Antalik.

use std::cmp::Ordering;

/// One ink dot: a filled circle in the 2D frame, carrying its pre-projection
/// depth `z` for sorting and shading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dot {
    /// X position in frame pixels.
    pub x: f64,
    /// Y position in frame pixels.
    pub y: f64,
    /// Depth after rotation (pre-projection z), used for the far→near sort.
    pub z: f64,
    /// Radius in pixels.
    pub r: f64,
    /// Ink value: 0 = darkest ink on paper. Mirrored on dark themes.
    pub white: f64,
    /// Optional alpha; `None` behaves as 1.0 (the TS `a ?? 1`).
    pub a: Option<f64>,
}

/// A stroked edge between two projected points (the `connecting` web).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    /// Start X in frame pixels.
    pub x1: f64,
    /// Start Y in frame pixels.
    pub y1: f64,
    /// End X in frame pixels.
    pub x2: f64,
    /// End Y in frame pixels.
    pub y2: f64,
    /// Ink value, same convention as [`Dot::white`].
    pub white: f64,
    /// Optional alpha; `None` behaves as 1.0.
    pub a: Option<f64>,
    /// Stroke width in pixels.
    pub w: f64,
}

/// One rendered instant: a complete, final set of draw instructions.
/// `dots` is already z-sorted into draw order and radius-clamped; `lines`
/// are drawn first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OrbFrame {
    /// Dots in draw order (z ascending).
    pub dots: Vec<Dot>,
    /// Edges, drawn before the dots.
    pub lines: Vec<Line>,
}

/// Linear interpolation between `a` and `b` by fraction `f`.
pub fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + (b - a) * f
}

/// Fractional part of `x` (`x - floor(x)`).
pub fn frac(x: f64) -> f64 {
    x - x.floor()
}

/// Deterministic hash in [0, 1).
pub fn hash_d(a: f64, b: f64) -> f64 {
    let h = (a * 12.9898 + b * 78.233).sin() * 43758.5453;
    h - h.floor()
}

/// Value noise on a 2D lattice — smooth, deterministic, cheap.
pub fn vnoise(x: f64, y: f64) -> f64 {
    let xi = x.floor();
    let yi = y.floor();
    let mut fx = x - xi;
    let mut fy = y - yi;
    fx = fx * fx * (3.0 - 2.0 * fx);
    fy = fy * fy * (3.0 - 2.0 * fy);
    let a = hash_d(xi, yi);
    let b = hash_d(xi + 1.0, yi);
    let c = hash_d(xi, yi + 1.0);
    let d = hash_d(xi + 1.0, yi + 1.0);
    a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy
}

/// Stable directions on a unit sphere (Fibonacci lattice).
pub fn fib_dir(i: f64, n: f64) -> (f64, f64, f64) {
    let golden = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
    let y = 1.0 - (2.0 * (i + 0.5)) / n;
    let rad = (1.0 - y * y).sqrt();
    let a = i * golden;
    (rad * a.cos(), y, rad * a.sin())
}

/// Shortest signed angular distance, wrapped to (-π, π].
pub fn angle_delta(a: f64, b: f64) -> f64 {
    (a - b).sin().atan2((a - b).cos())
}

/// Shared spin + tilt + orthographic projection.
#[derive(Clone, Copy, Debug)]
pub struct Proj {
    st: f64,
    ct: f64,
    sy: f64,
    cyw: f64,
    cx: f64,
    cy: f64,
    scale: f64,
}

impl Proj {
    /// Project a 3D point to `(px, py, z)` — frame pixels plus depth.
    pub fn project(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let x1 = x * self.cyw + z * self.sy;
        let z1 = -x * self.sy + z * self.cyw;
        let y1 = y * self.ct - z1 * self.st;
        let z2 = y * self.st + z1 * self.ct;
        (self.cx + x1 * self.scale, self.cy - y1 * self.scale, z2)
    }
}

/// Build the shared spin + tilt + orthographic projector.
pub fn make_proj(yaw: f64, tilt: f64, cx: f64, cy: f64, scale: f64) -> Proj {
    Proj {
        st: libm::sin(tilt),
        ct: libm::cos(tilt),
        sy: libm::sin(yaw),
        cyw: libm::cos(yaw),
        cx,
        cy,
        scale,
    }
}

/// Turn raw mode output into a finished frame: drop invisible marks, clamp
/// radii to the mode's floor, and z-sort far→near into draw order.
///
/// This runs in the GEOMETRY step, not the painter, so a frame is a complete
/// set of draw instructions: every value is final and the array order is the
/// order to draw in.
pub fn finalize_frame(dots: Vec<Dot>, lines: Vec<Line>, r_min: f64) -> OrbFrame {
    let mut visible: Vec<Dot> = dots
        .into_iter()
        .filter(|d| d.a.unwrap_or(1.0) >= 0.02)
        .map(|mut d| {
            d.r = d.r.max(r_min);
            d
        })
        .collect();
    visible.sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(Ordering::Equal));
    OrbFrame {
        dots: visible,
        lines: lines
            .into_iter()
            .filter(|l| l.a.unwrap_or(1.0) >= 0.02)
            .collect(),
    }
}

/// Dot radii were tuned for a 300pt frame; sub-linear scaling keeps small
/// spinners legible. Lower pow = radii shrink less with size.
pub fn radius_scale(size: f64, pow: f64) -> f64 {
    (size / 300.0).powf(pow)
}
