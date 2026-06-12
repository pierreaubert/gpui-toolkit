//! Hertzsprung-Russell Diagram — <https://observablehq.com/@d3/hertzsprung-russell-diagram>
//!
//! Scatter plot of ~29000 stars: absolute magnitude vs B-V color index.
//! Each star is a tiny dot colored by its spectral type.

use crate::scale::{LinearScale, Scale};

#[derive(Debug, Clone)]
pub struct HRStar {
    pub absolute_magnitude: f64,
    pub color_index: f64,
    pub x: f64,
    pub y: f64,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug)]
pub struct HRResult {
    pub width: f64,
    pub height: f64,
    pub stars: Vec<HRStar>,
    pub x_domain: [f64; 2],
    pub y_domain: [f64; 2],
}

/// Parse catalog.csv: absolute_magnitude,color
pub fn load_csv(csv_str: &str) -> Vec<(f64, f64)> {
    csv_str
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 2 {
                return None;
            }
            let mag: f64 = cols[0].parse().ok()?;
            let color: f64 = cols[1].parse().ok()?;
            Some((mag, color))
        })
        .collect()
}

/// Convert B-V color index to RGB (matches Observable's bv2rgb function).
pub fn bv_to_rgb(bv: f64) -> (u8, u8, u8) {
    let bv = bv.clamp(-0.4, 2.0);
    let (r, g, b);
    if bv < 0.0 {
        let t = (bv + 0.40) / 0.40;
        r = 0.61 + 0.11 * t + 0.1 * t * t;
        g = 0.70 + 0.07 * t + 0.1 * t * t;
        b = 1.0;
    } else if bv < 0.40 {
        let t = bv / 0.40;
        r = 0.83 + 0.17 * t;
        g = 0.87 + 0.11 * t;
        b = 1.0;
    } else if bv < 1.60 {
        let t = (bv - 0.40) / 1.20;
        r = 1.0;
        g = 0.98 - 0.16 * t;
        b = (1.0 - 0.5 * t).max(0.0);
    } else {
        let t = (bv - 1.60) / 0.40;
        r = 1.0;
        g = (0.82 - 0.5 * t).max(0.0);
        b = (0.4 - 0.4 * t).max(0.0);
    }
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// Compute HR diagram positions and colors.
pub fn compute(data: &[(f64, f64)]) -> HRResult {
    let width = 928.0;
    let height = 924.0;
    let margin = 40.0;

    let x_scale = LinearScale::new()
        .domain(-0.39, 2.19)
        .range(margin, width - margin);
    let y_scale = LinearScale::new()
        .domain(-7.0, 19.0)
        .range(margin, height - margin); // brighter (negative mag) at top

    let stars: Vec<HRStar> = data
        .iter()
        .map(|&(mag, color)| {
            let (r, g, b) = bv_to_rgb(color);
            HRStar {
                absolute_magnitude: mag,
                color_index: color,
                x: x_scale.scale(color),
                y: y_scale.scale(mag),
                r,
                g,
                b,
            }
        })
        .collect();

    HRResult {
        width,
        height,
        stars,
        x_domain: [-0.39, 2.19],
        y_domain: [-7.0, 19.0],
    }
}
