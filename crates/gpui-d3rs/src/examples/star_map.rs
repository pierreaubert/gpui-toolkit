//! Star Map — <https://observablehq.com/@d3/star-map>
//!
//! Stereographic star map with magnitude-scaled circles.
//! Uses a stereographic projection centered on the north celestial pole.

use crate::geo::Projection;
use crate::geo::projection::Stereographic;
use crate::scale::{LinearScale, Scale};
use crate::shape::path::{Path, PathBuilder};

#[derive(Debug, Clone)]
pub struct Star {
    pub ra: f64,  // right ascension in degrees
    pub dec: f64, // declination in degrees
    pub magnitude: f64,
    pub px: f64,     // projected x
    pub py: f64,     // projected y
    pub radius: f64, // display radius
}

#[derive(Debug)]
pub struct StarMapResult {
    pub width: f64,
    pub height: f64,
    pub stars: Vec<Star>,
    pub graticule_path: Path,
    pub outline_path: Path,
}

/// Parse stars.csv: columns 0,1 are pre-projected x,y; we use RA/Dec columns.
/// Format: x,y,ID,greek_letter,constellation,RA_hour,RA_min,RA_sec,dec_deg,dec_min,dec_sec,magnitude
pub fn load_csv(csv_str: &str) -> Vec<(f64, f64, f64)> {
    csv_str
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 12 {
                return None;
            }
            let ra_h: f64 = cols[5].trim().parse().ok()?;
            let ra_m: f64 = cols[6].trim().parse().ok()?;
            let ra_s: f64 = cols[7].trim().parse().ok()?;
            let dec_d: f64 = cols[8].trim().parse().ok()?;
            let dec_m: f64 = cols[9].trim().parse().ok()?;
            let dec_s: f64 = cols[10].trim().parse().ok()?;
            let mag: f64 = cols[11].trim().parse().ok()?;
            // Convert to degrees
            let ra = (ra_h + ra_m / 60.0 + ra_s / 3600.0) * 15.0; // hours → degrees
            let dec_sign = if dec_d < 0.0 { -1.0 } else { 1.0 };
            let dec = dec_d.abs() + dec_m / 60.0 + dec_s / 3600.0;
            let dec = dec * dec_sign;
            Some((ra, dec, mag))
        })
        .collect()
}

/// Compute star map with stereographic projection.
pub fn compute(stars_data: &[(f64, f64, f64)]) -> StarMapResult {
    let width: f64 = 928.0;
    let height: f64 = 928.0;

    // Stereographic projection centered on north pole, reflected Y (like D3)
    let mut proj = Stereographic::new();
    proj.set_scale(width / 2.0 * 0.9);
    proj.set_translate(width / 2.0, height / 2.0);
    proj.set_rotate(0.0, -90.0, 0.0); // center on north pole

    // Magnitude → radius scale (D3: scaleLinear([6, -1], [0, 8]))
    let radius_scale = LinearScale::new().domain(6.0, -1.0).range(0.0, 8.0);

    let stars: Vec<Star> = stars_data
        .iter()
        .filter_map(|&(ra, dec, mag)| {
            // Only show stars above the horizon (dec > -30° for a north-centered map)
            if dec < -30.0 {
                return None;
            }
            let (px, py) = proj.project(ra, dec);
            if !px.is_finite() || !py.is_finite() {
                return None;
            }
            // Clip to canvas
            let dx = px - width / 2.0;
            let dy = py - height / 2.0;
            let r = width / 2.0 * 0.95;
            if dx * dx + dy * dy > r * r {
                return None;
            }
            let radius = radius_scale.scale(mag).max(0.0);
            Some(Star {
                ra,
                dec,
                magnitude: mag,
                px,
                py,
                radius,
            })
        })
        .collect();

    // Graticule
    let mut grat_builder = PathBuilder::new();
    // RA lines every 2 hours (30°)
    for h in (0..24).step_by(2) {
        let ra = h as f64 * 15.0;
        let mut first = true;
        for dec_i in (-30..=90).step_by(2) {
            let dec = dec_i as f64;
            let (px, py) = proj.project(ra, dec);
            if px.is_finite() && py.is_finite() {
                if first {
                    grat_builder = grat_builder.move_to(px, py);
                    first = false;
                } else {
                    grat_builder = grat_builder.line_to(px, py);
                }
            } else {
                first = true;
            }
        }
    }
    // Dec circles every 30°
    for dec_i in (-30..=60).step_by(30) {
        let dec = dec_i as f64;
        let mut first = true;
        for ra_i in (0..=360).step_by(2) {
            let ra = ra_i as f64;
            let (px, py) = proj.project(ra, dec);
            if px.is_finite() && py.is_finite() {
                if first {
                    grat_builder = grat_builder.move_to(px, py);
                    first = false;
                } else {
                    grat_builder = grat_builder.line_to(px, py);
                }
            } else {
                first = true;
            }
        }
    }

    // Outline circle
    let n_sides = 64;
    let r = width / 2.0 * 0.95;
    let mut outline_builder = PathBuilder::new();
    for v in 0..n_sides {
        let angle = std::f64::consts::TAU * v as f64 / n_sides as f64;
        let x = width / 2.0 + r * angle.cos();
        let y = height / 2.0 + r * angle.sin();
        if v == 0 {
            outline_builder = outline_builder.move_to(x, y);
        } else {
            outline_builder = outline_builder.line_to(x, y);
        }
    }
    outline_builder = outline_builder.close_path();

    StarMapResult {
        width,
        height,
        stars,
        graticule_path: grat_builder.build(),
        outline_path: outline_builder.build(),
    }
}
