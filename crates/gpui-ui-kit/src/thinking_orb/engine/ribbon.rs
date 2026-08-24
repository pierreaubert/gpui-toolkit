//! Ribbon: an undulating sash of parallel strands rides a great circle —
//! the "composing" state. The tuned preset freezes the 3D tumble
//! (spin 0), leaving the traveling undulation on a fixed band.
//!
//! The same painter also drives "breathing" (ring), via the `faceOn` flag:
//! a face-on circle whose radius — not its out-of-plane offset — undulates,
//! so it reads as a ring slowly morphing rather than a sash in orbit.
//! Ported from `thinking-orbs` 0.3.1 `engine/ribbon.ts`, MIT © Jakub Antalik.

use super::core::{Dot, OrbFrame, fib_dir, finalize_frame, make_proj, radius_scale};
use super::profiles::ModeOpts;

/// Frame function for [`super::ModeKey::Ribbon`] and [`super::ModeKey::Ring`]
/// (ring sets the `faceOn` profile flag).
pub fn frame_ribbon(size: f64, t: f64, o: &ModeOpts) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    let r_max = (size / 2.0) * 0.78;
    // spin scales the 3D tumble; spin=0 freezes the band's orientation,
    // leaving only the traveling undulation
    let spin = o.get("spin", 1.0);
    let cam_tilt = 0.3;
    let pt = make_proj(t * 0.1 * spin, cam_tilt, cx, cy, 1.0);
    let rs = radius_scale(size, o.get("rsPow", 0.6));
    let face_on = o.get("faceOn", 0.0) != 0.0;

    let mut dots: Vec<Dot> = Vec::new();
    let ghost_n = o.get("ghostN", 150.0) as usize;
    for i in 0..ghost_n {
        let d = fib_dir(i as f64, ghost_n as f64);
        let (px, py, z) = pt.project(d.0 * r_max, d.1 * r_max, d.2 * r_max);
        let depth = (z / r_max + 1.0) / 2.0;
        dots.push(Dot {
            x: px,
            y: py,
            z,
            r: 0.8 * rs,
            white: 0.78,
            a: Some(0.1 + 0.22 * depth),
        });
    }

    // The band plane, precessing (frozen when spin=0). The projection squashes
    // the band's great circle vertically by cos(ta + camTilt); face-on sets
    // ta = -camTilt so that term is 1 and the band reads as a true circle
    // rather than ribbon's tilted ellipse.
    let ya = t * 0.24 * spin;
    let ta = if face_on {
        -cam_tilt
    } else {
        0.55 + 0.3 * (t * 0.18).sin() * spin
    };
    let ux = ya.cos();
    let uy = 0.0;
    let uz = ya.sin();
    let vx = -uz * ta.sin();
    let vy = ta.cos();
    let vz = ux * ta.sin();
    // plane normal n = u × v
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;

    // Radial lobes swell past R, so pull the base radius in by (most of) the
    // wobble amplitude. The silhouette then stays inside the frame however far
    // the deformation is pushed, while lobes keep getting deeper relative to
    // the mean radius.
    let wob_amp = 0.23 * o.get("wobMul", 1.0);
    let base_r = if face_on {
        r_max / (1.0 + 0.85 * wob_amp)
    } else {
        r_max
    };

    let base_lanes = o.get("lanes", 5.0);
    let segs = o.get("segs", 88.0) as usize;
    let lanes = (base_lanes * o.get("bandMul", 1.0)).round().max(1.0) as usize;
    for w in 0..lanes {
        let lane_off = (w as f64 - (lanes as f64 - 1.0) / 2.0) * 0.075;
        let edge =
            (w as f64 - (lanes as f64 - 1.0) / 2.0).abs() / ((lanes as f64 - 1.0) / 2.0).max(1.0);
        for k in 0..segs {
            let a = (k as f64 / segs as f64) * 2.0 * std::f64::consts::PI;
            // the undulation: two traveling waves along the band; wobMul
            // scales the deformation — 0 is a clean band
            let wob = (0.16 * (a * 3.0 - t * 1.7 + w as f64 * 0.22).sin()
                + 0.07 * (a * 5.0 + t * 1.1).sin())
                * o.get("wobMul", 1.0);
            // A normal-direction wobble is cancelled by the re-normalisation below:
            // the point lands back on the sphere, so the silhouette is pinned at R
            // and the deformation can only ever pull dots inward. Face-on instead
            // modulates the in-plane RADIUS, so lobes genuinely swell outward and
            // pinch inward. Ribbon keeps the original out-of-plane sash wobble.
            let radial = if face_on { 1.0 + wob } else { 1.0 };
            let off = if face_on { lane_off } else { lane_off + wob };
            let x = ux * a.cos() + vx * a.sin() + nx * off;
            let y = uy * a.cos() + vy * a.sin() + ny * off;
            let z = uz * a.cos() + vz * a.sin() + nz * off;
            let l = (x * x + y * y + z * z).sqrt();
            let rr = base_r * radial;
            let (px, py, zr) = pt.project((x / l) * rr, (y / l) * rr, (z / l) * rr);
            let depth = (zr / r_max + 1.0) / 2.0;
            dots.push(Dot {
                x: px,
                y: py,
                z: zr,
                r: (o.get("rBase", 1.1) + o.get("rDepth", 1.7) * depth) * (1.0 - 0.25 * edge) * rs,
                white: 0.52 - 0.44 * depth + 0.18 * edge,
                a: Some(0.4 + 0.6 * depth),
            });
        }
    }
    finalize_frame(dots, Vec::new(), o.get("rMin", 0.3))
}
