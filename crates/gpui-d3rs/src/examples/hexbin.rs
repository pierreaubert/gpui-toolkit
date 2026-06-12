//! Hexbin Chart — <https://observablehq.com/@d3/hexbin>
//!
//! Demonstrates: `LogScale`, `Hexbin` binning through projected coordinates.

use crate::color::{D3Color, SequentialScheme};
use crate::hexbin::Hexbin as HexbinLayout;
use crate::scale::{LogScale, Scale};

#[derive(Debug, Clone)]
pub struct HexbinResult {
    pub width: f64,
    pub height: f64,
    pub bins: Vec<BinResult>,
    pub x_domain: [f64; 2],
    pub y_domain: [f64; 2],
    pub hex_radius: f64,
}

#[derive(Debug, Clone)]
pub struct BinResult {
    pub x: f64,
    pub y: f64,
    pub count: usize,
    pub color: D3Color,
}

/// Compute hexbin from (carat, price) data using log scales.
pub fn compute(data: &[(f64, f64)]) -> HexbinResult {
    let width = 928.0;
    let height = 928.0;
    let margin_top = 20.0;
    let margin_right = 20.0;
    let margin_bottom = 30.0;
    let margin_left = 40.0;
    let radius = 8.0;

    let x_extent = data.iter().fold((f64::MAX, f64::MIN), |acc, d| {
        (acc.0.min(d.0), acc.1.max(d.0))
    });
    let y_extent = data.iter().fold((f64::MAX, f64::MIN), |acc, d| {
        (acc.0.min(d.1), acc.1.max(d.1))
    });

    let x_scale = LogScale::new()
        .domain(x_extent.0, x_extent.1)
        .range(margin_left, width - margin_right);

    let y_scale = LogScale::new()
        .domain(y_extent.0, y_extent.1)
        .range(height - margin_bottom, margin_top);

    let hex_radius = radius * width / 928.0;

    // Project data through scales, then bin
    let projected: Vec<[f64; 2]> = data
        .iter()
        .map(|(c, p)| [x_scale.scale(*c), y_scale.scale(*p)])
        .collect();

    let hex = HexbinLayout::new().radius(hex_radius).extent(
        margin_left,
        margin_top,
        width - margin_right,
        height - margin_bottom,
    );

    let bins = hex.bin(projected);

    let max_count = bins.iter().map(|b| b.len()).max().unwrap_or(1);
    let color_scale = SequentialScheme::bu_pu();

    let bin_results: Vec<BinResult> = bins
        .iter()
        .map(|b| {
            let t = b.len() as f64 / max_count as f64;
            BinResult {
                x: b.x,
                y: b.y,
                count: b.len(),
                color: color_scale.get(t),
            }
        })
        .collect();

    HexbinResult {
        width,
        height,
        bins: bin_results,
        x_domain: [x_extent.0, x_extent.1],
        y_domain: [y_extent.0, y_extent.1],
        hex_radius,
    }
}
