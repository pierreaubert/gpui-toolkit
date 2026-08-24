//! The sphere-lattice modes: globe (searching), rubik (solving) and
//! wave (listening). All draw a lat/long dot field with mode-specific
//! motion, then hand off to the shared z-sorted painter. Ported from
//! `thinking-orbs` 0.3.1 `engine/lattice.ts`, MIT © Jakub Antalik.

use super::core::{Dot, OrbFrame, angle_delta, finalize_frame, hash_d, make_proj, radius_scale};
use super::profiles::ModeOpts;

// --- the shared solver heartbeat (rubik) ------------------------------
// Rapid eased moves scramble, then replay in reverse (palindrome) so
// everything clicks back to solved, rests, repeats.

#[derive(Clone, Copy)]
struct Move {
    axis: usize, // 0 = x, 1 = y, 2 = z
    lo: f64,
    hi: f64,
    ang: f64,
}

struct SolveCycle {
    amount: Vec<f64>,
    active: i64,
}

fn solve_cycle(time: f64, count: usize, slot_dur: f64, rest: f64) -> SolveCycle {
    let cyc = 2.0 * count as f64 * slot_dur + rest;
    let tc = time % cyc;
    let mut amount = vec![0.0; count];
    let mut active: i64 = -1;
    if tc < 2.0 * count as f64 * slot_dur {
        let slot = (tc / slot_dur).floor() as usize;
        let p = (tc - slot as f64 * slot_dur) / slot_dur;
        let cl = (p / 0.7).min(1.0);
        let ep = 1.0 - (1.0 - cl).powi(3); // machine ease-out
        if slot < count {
            for a in amount.iter_mut().take(slot) {
                *a = 1.0;
            }
            amount[slot] = ep;
            active = slot as i64;
        } else {
            let u = 2 * count - 1 - slot;
            for a in amount.iter_mut().take(u) {
                *a = 1.0;
            }
            amount[u] = 1.0 - ep;
            active = u as i64;
        }
    }
    SolveCycle { amount, active }
}

fn apply_moves(pt3: (f64, f64, f64), moves: &[Move], sc: &SolveCycle) -> (f64, f64, f64, bool) {
    let (mut x, mut y, mut z) = pt3;
    let mut in_active = false;
    for (i, mv) in moves.iter().enumerate() {
        if sc.amount[i] <= 0.0 {
            continue;
        }
        let coord = match mv.axis {
            0 => x,
            1 => y,
            _ => z,
        };
        if coord < mv.lo || coord >= mv.hi {
            continue;
        }
        if i as i64 == sc.active {
            in_active = true;
        }
        let a = mv.ang * sc.amount[i];
        let ca = a.cos();
        let sa = a.sin();
        if mv.axis == 0 {
            let y2 = y * ca - z * sa;
            z = y * sa + z * ca;
            y = y2;
        } else if mv.axis == 1 {
            let x2 = x * ca + z * sa;
            z = -x * sa + z * ca;
            x = x2;
        } else {
            let x2 = x * ca - y * sa;
            y = x * sa + y * ca;
            x = x2;
        }
    }
    (x, y, z, in_active)
}

fn make_moves(count: usize) -> Vec<Move> {
    let mut moves = Vec::with_capacity(count);
    for i in 0..count {
        let axis = (hash_d(i as f64, 2.3) * 3.0).floor().min(2.0) as usize;
        let lo = -1.0 + 0.5 * (hash_d(i as f64, 5.9) * 4.0).floor().min(3.0);
        let dir = if hash_d(i as f64, 7.7) < 0.5 {
            1.0
        } else {
            -1.0
        };
        moves.push(Move {
            axis,
            lo,
            hi: lo + 0.5,
            ang: dir * std::f64::consts::PI / 2.0,
        });
    }
    moves
}

// --- Globe: lat/long field, a scan meridian sweeps — searching --------

/// Frame function for [`super::ModeKey::Globe`].
pub fn frame_globe(size: f64, t: f64, o: &ModeOpts) -> OrbFrame {
    let spin = 0.5;
    let cx = size / 2.0;
    let cy = size / 2.0;
    let radius = (size / 2.0) * 0.82;
    let tilt = 0.4 + 0.06 * (t * 0.35).sin();
    let pt = make_proj(t * spin, tilt, cx, cy, radius);
    // scan sweeps relative to the spin; scanMul scales that relative rate
    let scan = t * (spin + (1.7 - spin) * o.get("scanMul", 1.0));
    let rs = radius_scale(size, o.get("rsPow", 0.6));
    let dim_base = o.get("dimBase", 1.0);

    let mut dots: Vec<Dot> = Vec::new();
    let lat_rings = o.get("latRings", 17.0) as usize;
    let lon_density = o.get("lonDensity", 44.0);
    for li in 0..=lat_rings {
        let lat =
            -std::f64::consts::FRAC_PI_2 + (li as f64 / lat_rings as f64) * std::f64::consts::PI;
        let cos_lat = lat.cos();
        let sin_lat = lat.sin();
        let lon_count = (cos_lat.abs() * lon_density).round().max(1.0) as usize;
        for lj in 0..lon_count {
            let lon = (lj as f64 / lon_count as f64) * 2.0 * std::f64::consts::PI;
            let (px, py, z) = pt.project(cos_lat * lon.cos(), sin_lat, cos_lat * lon.sin());
            let depth = (z + 1.0) / 2.0;
            // the scan: a moving meridian read as a size ripple, not a shine
            let d = angle_delta(lon + t * spin, scan);
            let boost = (-(d * d) / 0.18).exp() * z.max(0.0);
            dots.push(Dot {
                x: px,
                y: py,
                z,
                r: (o.get("rBase", 0.6)
                    + o.get("rDepth", 1.7) * depth
                    + o.get("rBoost", 1.0) * boost)
                    * rs,
                white: o.get("inkFar", 0.62) - o.get("inkSpan", 0.54) * depth,
                // dimBase < 1 fades un-scanned dots so the meridian reads clearly
                a: Some(dim_base + (1.0 - dim_base) * boost.min(1.0)),
            });
        }
    }
    finalize_frame(dots, Vec::new(), o.get("rMin", 0.3))
}

// --- Rubik: bands twist in quarter turns, scramble → solve — solving --

/// Frame function for [`super::ModeKey::Rubik`].
pub fn frame_rubik(size: f64, t: f64, o: &ModeOpts) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    let r_max = (size / 2.0) * 0.82;
    let pt = make_proj(t * 0.55, 0.35 + 0.1 * (t * 0.9).sin(), cx, cy, r_max);
    let rs = radius_scale(size, o.get("rsPow", 0.6));
    let move_count = o.get("moveCount", 14.0) as usize;
    let moves = make_moves(move_count);
    let sc = solve_cycle(t, move_count, 0.42, 1.2);

    let mut dots: Vec<Dot> = Vec::new();
    let lat_rings = o.get("latRings", 15.0) as usize;
    let lon_density = o.get("lonDensity", 40.0);
    for li in 0..=lat_rings {
        let lat =
            -std::f64::consts::FRAC_PI_2 + (li as f64 / lat_rings as f64) * std::f64::consts::PI;
        let cos_lat = lat.cos();
        let sin_lat = lat.sin();
        let lon_count = (cos_lat.abs() * lon_density).round().max(1.0) as usize;
        for lj in 0..lon_count {
            let lon = (lj as f64 / lon_count as f64) * 2.0 * std::f64::consts::PI;
            let (x, y, z, in_active) = apply_moves(
                (cos_lat * lon.cos(), sin_lat, cos_lat * lon.sin()),
                &moves,
                &sc,
            );
            let (px, py, zr) = pt.project(x, y, z);
            let depth = (zr + 1.0) / 2.0;
            // the band being turned inks a touch darker — the "hand"
            dots.push(Dot {
                x: px,
                y: py,
                z: zr,
                r: (o.get("rBase", 0.6)
                    + o.get("rDepth", 1.7) * depth
                    + if in_active {
                        o.get("rActive", 0.3)
                    } else {
                        0.0
                    })
                    * rs,
                white: o.get("inkFar", 0.62)
                    - o.get("inkSpan", 0.54) * depth
                    - if in_active { 0.14 } else { 0.0 },
                a: None,
            });
        }
    }
    finalize_frame(dots, Vec::new(), o.get("rMin", 0.3))
}

// --- Wave: a waveform rolls through the rings — listening -------------

/// Frame function for [`super::ModeKey::Wave`].
pub fn frame_wave(size: f64, t: f64, o: &ModeOpts) -> OrbFrame {
    let cx = size / 2.0;
    let cy = size / 2.0;
    // 0.76 base × 1.15 — the undulation pulls the sphere inward, so wave read
    // ~15% smaller than the other lattice modes; scaled up to match them
    let r_max = (size / 2.0) * 0.874;
    let pt = make_proj(t * 0.18, 0.38, cx, cy, 1.0);
    let rs = radius_scale(size, o.get("rsPow", 0.6));

    let mut dots: Vec<Dot> = Vec::new();
    let rings = o.get("rings", 15.0) as usize;
    let lon_density = o.get("lonDensity", 40.0);
    for ri in 0..=rings {
        let lat = -std::f64::consts::FRAC_PI_2 + (ri as f64 / rings as f64) * std::f64::consts::PI;
        let cos_lat = lat.cos();
        let sin_lat = lat.sin();
        // two waves, different tempi — organic, never quite repeating
        let w =
            0.62 * (t * 2.1 - ri as f64 * 0.52).sin() + 0.38 * (t * 1.27 + ri as f64 * 0.83).sin();
        let rr = r_max * (0.88 + 0.105 * w);
        let lon_count = (cos_lat.abs() * lon_density).round().max(1.0) as usize;
        for lj in 0..lon_count {
            let lon = (lj as f64 / lon_count as f64) * 2.0 * std::f64::consts::PI;
            let (px, py, z) = pt.project(
                cos_lat * lon.cos() * rr,
                sin_lat * rr,
                cos_lat * lon.sin() * rr,
            );
            let depth = (z / r_max + 1.0) / 2.0;
            let crest = w.max(0.0);
            dots.push(Dot {
                x: px,
                y: py,
                z,
                r: (o.get("rBase", 0.6) + o.get("rDepth", 1.7) * depth) * (1.0 + 0.4 * crest) * rs,
                white: 0.66 - 0.56 * depth - 0.1 * crest,
                a: None,
            });
        }
    }
    finalize_frame(dots, Vec::new(), o.get("rMin", 0.3))
}
