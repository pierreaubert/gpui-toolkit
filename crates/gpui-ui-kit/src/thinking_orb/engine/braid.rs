//! Braid: three strands plait around the sphere — the "weaving" state.
//! Each strand runs pole to pole on a helix, and a radial breathing term
//! makes them trade places, reading as the over/under of a plait. Ported
//! from `thinking-orbs` 0.3.1 `engine/braid.ts`, MIT © Jakub Antalik.

use super::core::{Dot, OrbFrame, fib_dir, finalize_frame, frac, make_proj, radius_scale};
use super::profiles::ModeOpts;

/// Frame function for [`super::ModeKey::Braid`].
pub fn frame_braid(size: f64, t: f64, o: &ModeOpts) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    let r_max = (size / 2.0) * 0.76;
    let pt = make_proj(t * 0.4, 0.3, cx, cy, 1.0);
    let rs = radius_scale(size, o.get("rsPow", 0.6));

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

    let strand_n = o.get("strandN", 52.0) as usize;
    let turns = o.get("turns", 3.0);
    for s in 0..3 {
        let phase = (s as f64 / 3.0) * 2.0 * std::f64::consts::PI;
        for i in 0..strand_n {
            // u walks pole to pole; the frac() drift slides the whole strand along
            let u = (frac(i as f64 / strand_n as f64 + t * 0.045) * 2.0 - 1.0) * 0.96;
            let surf = (1.0 - u * u).max(0.0).sqrt();
            let end_fade = ((1.0 - u.abs()) / 0.1).min(1.0);
            let a = u * std::f64::consts::PI * turns + phase;
            // radial breathing: strands trade places — the over/under of a plait
            let weave = 1.0
                + 0.075 * (u * std::f64::consts::PI * turns * 2.0 + phase * 2.0 + t * 0.8).sin();
            let rr = surf * r_max * weave;
            let (px, py, zr) = pt.project(a.cos() * rr, u * r_max * weave, a.sin() * rr);
            let depth = (zr / r_max + 1.0) / 2.0;
            dots.push(Dot {
                x: px,
                y: py,
                z: zr,
                r: (o.get("rBase", 1.2) + o.get("rDepth", 1.8) * depth) * rs,
                white: 0.55 - 0.45 * depth,
                a: Some(end_fade * (0.45 + 0.55 * depth)),
            });
        }
    }
    finalize_frame(dots, Vec::new(), o.get("rMin", 0.3))
}
