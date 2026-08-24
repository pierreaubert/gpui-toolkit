//! Morph: a dotted outline cycling circle → triangle → square → circle —
//! the "shaping" state. Each shape is a continuous closed path
//! parameterised by arc length (top-centre start, clockwise). Every
//! frame the engine blends the two neighbouring paths, then lays the
//! dots EVENLY along the blended outline — spacing stays uniform at
//! every instant of the morph, holds and transitions alike. Ported from
//! `thinking-orbs` 0.3.1 `engine/morph.ts`, MIT © Jakub Antalik.

use super::core::{Dot, OrbFrame, finalize_frame};
use super::profiles::ModeOpts;

fn smooth_e(x: f64) -> f64 {
    x * x * (3.0 - 2.0 * x)
}

/// A closed polygonal path parameterised by arc length.
struct PolyPath {
    verts: &'static [(f64, f64)],
    segs: Vec<f64>,
    total: f64,
}

impl PolyPath {
    fn new(verts: &'static [(f64, f64)]) -> Self {
        let v = verts.len();
        let mut segs = Vec::with_capacity(v);
        let mut total = 0.0;
        for i in 0..v {
            let a = verts[i];
            let b = verts[(i + 1) % v];
            let l = (b.0 - a.0).hypot(b.1 - a.1);
            segs.push(l);
            total += l;
        }
        PolyPath { verts, segs, total }
    }

    fn at(&self, f: f64) -> (f64, f64) {
        let v = self.verts.len();
        let mut target = f * self.total;
        let mut i = 0;
        while target > self.segs[i] && i < v - 1 {
            target -= self.segs[i];
            i += 1;
        }
        let a = self.verts[i];
        let b = self.verts[(i + 1) % v];
        let ff = if self.segs[i] != 0.0 {
            (target / self.segs[i]).min(1.0)
        } else {
            0.0
        };
        (a.0 + (b.0 - a.0) * ff, a.1 + (b.1 - a.1) * ff)
    }
}

/// One morphable outline, parameterised by arc length over [0, 1).
enum ShapePath {
    Circle,
    Poly(PolyPath),
}

impl ShapePath {
    fn at(&self, f: f64) -> (f64, f64) {
        match self {
            ShapePath::Circle => {
                let a = -std::f64::consts::FRAC_PI_2 + f * 2.0 * std::f64::consts::PI;
                (a.cos() * 0.24, a.sin() * 0.24)
            }
            ShapePath::Poly(p) => p.at(f),
        }
    }
}

const TRIANGLE_VERTS: &[(f64, f64)] = &[(0.0, -0.26), (0.24, 0.16), (-0.24, 0.16)];
// 5-vertex walk so the path STARTS at top-centre like the other shapes
const SQUARE_VERTS: &[(f64, f64)] = &[
    (0.0, -0.2),
    (0.2, -0.2),
    (0.2, 0.2),
    (-0.2, 0.2),
    (-0.2, -0.2),
];

fn cycle() -> [ShapePath; 3] {
    [
        ShapePath::Circle,
        ShapePath::Poly(PolyPath::new(TRIANGLE_VERTS)),
        ShapePath::Poly(PolyPath::new(SQUARE_VERTS)),
    ]
}

// low floor keeps sparse outlines possible while never degenerating
fn morph_n(d: f64) -> usize {
    (34.0 * d).round().max(6.0) as usize
}

const HOLD: f64 = 1.4;
const MORPH: f64 = 0.9;
const SEG: f64 = HOLD + MORPH;

// This state was tuned in inkform, which paints it through a blur +
// threshold "goo" filter; we draw plain circles instead. The dot GEOMETRY is
// identical either way — the threshold just yields a hard edge where a plain
// fill has an antialiased one, so these dots read a touch softer than
// inkform's. Don't "correct" for that by shrinking the radius: it makes the
// mark genuinely smaller than the tuning.

/// Frame function for [`super::ModeKey::Morph`].
pub fn frame_morph(size: f64, t: f64, o: &ModeOpts) -> OrbFrame {
    let shapes = cycle();
    let k_count = shapes.len();
    let tc = t % (SEG * k_count as f64);
    let kf = (tc / SEG).floor();
    let k = kf as usize;
    let local = tc - kf * SEG;
    let m = if local > HOLD {
        smooth_e((local - HOLD) / MORPH)
    } else {
        0.0
    };
    let sprd = o.get("spread", 1.0);

    // blend the two shape PATHS at m, then measure the blended outline
    let p_a = &shapes[k];
    let p_b = &shapes[(k + 1) % k_count];
    let m_samples = 160usize;
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(m_samples);
    for i in 0..m_samples {
        let f = i as f64 / m_samples as f64;
        let a = p_a.at(f);
        let b = p_b.at(f);
        pts.push((
            (a.0 + (b.0 - a.0) * m) * sprd,
            (a.1 + (b.1 - a.1) * m) * sprd,
        ));
    }
    let mut seg_lens: Vec<f64> = Vec::with_capacity(m_samples);
    let mut total = 0.0;
    for i in 0..m_samples {
        let a = pts[i];
        let b = pts[(i + 1) % m_samples];
        let l = (b.0 - a.0).hypot(b.1 - a.1);
        seg_lens.push(l);
        total += l;
    }

    // dot radius depends ONLY on rDot (the size knob); the count sets the
    // gaps. Formed shapes breathe a little (uniform pulse).
    let n = morph_n(o.get("iconD", 1.0));
    let re = o.get("rDot", 0.021) * 1.35 * sprd;
    let pulse = 1.0 + 0.02 * (local * 3.1).sin();

    let mut dots: Vec<Dot> = Vec::with_capacity(n);
    let c2 = size / 2.0;
    let mut seg = 0usize;
    let mut acc = 0.0;
    for k2 in 0..n {
        let target = (k2 as f64 / n as f64) * total;
        while acc + seg_lens[seg] < target && seg < m_samples - 1 {
            acc += seg_lens[seg];
            seg += 1;
        }
        let a = pts[seg];
        let b = pts[(seg + 1) % m_samples];
        let f = if seg_lens[seg] != 0.0 {
            ((target - acc) / seg_lens[seg]).min(1.0)
        } else {
            0.0
        };
        let x = (a.0 + (b.0 - a.0) * f) * pulse;
        let y = (a.1 + (b.1 - a.1) * f) * pulse;
        dots.push(Dot {
            x: c2 + x * size,
            y: c2 + y * size,
            z: 0.0,
            r: (re * size).max(0.35),
            white: 0.1,
            a: None,
        });
    }
    finalize_frame(dots, Vec::new(), o.get("rMin", 0.3))
}
