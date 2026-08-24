//! Orbits: particles on tilted orbits — the "working" state. No nucleus
//! (the tuned preset runs coreless): just ghost paths and the particles
//! doing the work. Ported from `thinking-orbs` 0.3.1 `engine/orbits.ts`,
//! MIT © Jakub Antalik.

use super::core::{Dot, OrbFrame, finalize_frame, hash_d, make_proj, radius_scale};
use super::profiles::ModeOpts;

/// Frame function for [`super::ModeKey::Orbits`].
pub fn frame_orbits(size: f64, t: f64, o: &ModeOpts) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    let r_max = (size / 2.0) * 0.82;
    let pt = make_proj(t * 0.12, 0.3, cx, cy, 1.0);
    let rs = radius_scale(size, o.get("rsPow", 0.6));

    let mut dots: Vec<Dot> = Vec::new();
    let orbit_n = o.get("orbitN", 12.0) as usize;
    let ghost_n = o.get("ghostN", 40.0) as usize;
    let particles = o.get("particles", 3.0) as usize;

    // orbits: each a tilted circle — a ghost path + running particles
    for orb in 0..orbit_n {
        let h1 = hash_d(orb as f64, 1.7);
        let h2 = hash_d(orb as f64, 5.2);
        let h3 = hash_d(orb as f64, 8.9);
        let ro = r_max * (0.45 + 0.52 * h1);
        let th = h1 * 2.0 * std::f64::consts::PI;
        let phi = (2.0 * h2 - 1.0).acos();
        // orbit plane basis (u, v ⟂ normal n)
        let nx = phi.sin() * th.cos();
        let ny = phi.cos();
        let nz = phi.sin() * th.sin();
        let mut ux = -ny;
        let mut uy = nx;
        let uz = 0.0;
        let ul = (ux * ux + uy * uy).sqrt().max(1e-6);
        ux /= ul;
        uy /= ul;
        let vx = ny * uz - nz * uy;
        let vy = nz * ux - nx * uz;
        let vz = nx * uy - ny * ux;
        let speed = (0.25 + 0.55 * h3) * if h3 > 0.5 { 1.0 } else { -1.0 };

        // ghost path
        for k in 0..ghost_n {
            let a = (k as f64 / ghost_n as f64) * 2.0 * std::f64::consts::PI;
            let (px, py, z) = pt.project(
                (ux * a.cos() + vx * a.sin()) * ro,
                (uy * a.cos() + vy * a.sin()) * ro,
                (uz * a.cos() + vz * a.sin()) * ro,
            );
            let depth = (z / ro + 1.0) / 2.0;
            dots.push(Dot {
                x: px,
                y: py,
                z,
                r: o.get("ghostR", 0.9) * rs,
                white: 0.72,
                a: Some(o.get("ghostA", 0.5) * (0.4 + 0.6 * depth)),
            });
        }
        // the particles doing the work
        for m in 0..particles {
            let a =
                t * speed + (m as f64 / particles as f64) * 2.0 * std::f64::consts::PI + h2 * 6.0;
            let (px, py, z) = pt.project(
                (ux * a.cos() + vx * a.sin()) * ro,
                (uy * a.cos() + vy * a.sin()) * ro,
                (uz * a.cos() + vz * a.sin()) * ro,
            );
            let depth = (z / ro + 1.0) / 2.0;
            dots.push(Dot {
                x: px,
                y: py,
                z,
                r: (o.get("partR", 1.2) + o.get("partRDepth", 1.6) * depth) * rs,
                white: 0.3 - 0.22 * depth,
                a: None,
            });
        }
    }
    finalize_frame(dots, Vec::new(), o.get("rMin", 0.3))
}
