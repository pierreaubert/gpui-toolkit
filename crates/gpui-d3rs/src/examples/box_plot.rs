//! Box Plot — <https://observablehq.com/@d3/box-plot>
//!
//! Demonstrates: `d3.quantile`, IQR-based whiskers, `BandScale`, `LinearScale`.

use crate::array::statistics::quantile_sorted;
use crate::scale::BandScale;

#[derive(Debug, Clone)]
pub struct BoxStats {
    pub group: String,
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    pub iqr: f64,
    pub whisker_low: f64,
    pub whisker_high: f64,
    pub outliers: Vec<f64>,
}

#[derive(Debug)]
pub struct BoxPlotResult {
    pub width: f64,
    pub height: f64,
    pub groups: Vec<BoxStats>,
    pub band_positions: Vec<f64>,
    pub bandwidth: f64,
    pub y_domain: [f64; 2],
}

/// Default data: 5 groups with varying distributions.
pub fn default_data() -> Vec<(String, Vec<f64>)> {
    let groups = ["A", "B", "C", "D", "E"];
    groups
        .iter()
        .enumerate()
        .map(|(gi, group)| {
            let base = 20.0 + gi as f64 * 15.0;
            let spread = 5.0 + gi as f64 * 3.0;
            let values: Vec<f64> = (0..50)
                .map(|i| {
                    let r = (i as f64 * 7.3 + gi as f64 * 13.1).sin() * 0.5 + 0.5;
                    let mut v = base + (r - 0.5) * spread * 2.0;
                    if i % 17 == 0 {
                        v = base + spread * 4.0 * if r > 0.5 { 1.0 } else { -1.0 };
                    }
                    (v * 100.0).round() / 100.0
                })
                .collect();
            (group.to_string(), values)
        })
        .collect()
}

pub fn compute(data: &[(String, Vec<f64>)]) -> BoxPlotResult {
    let width = 928.0;
    let height = 500.0;
    let _margin_top = 20.0;
    let margin_right = 20.0;
    let _margin_bottom = 30.0;
    let margin_left = 40.0;

    let group_names: Vec<String> = data.iter().map(|(g, _)| g.clone()).collect();

    let groups: Vec<BoxStats> = data
        .iter()
        .map(|(group, raw_values)| {
            let mut values = raw_values.clone();
            values.sort_by(|a, b| a.total_cmp(b));

            let q1 = quantile_sorted(&values, 0.25).unwrap_or(f64::NAN);
            let q2 = quantile_sorted(&values, 0.50).unwrap_or(f64::NAN);
            let q3 = quantile_sorted(&values, 0.75).unwrap_or(f64::NAN);
            let iqr = q3 - q1;
            let min = values[0];
            let max = values[values.len() - 1];
            let r0 = min.max(q1 - iqr * 1.5);
            let r1 = max.min(q3 + iqr * 1.5);
            let outliers: Vec<f64> = values
                .iter()
                .filter(|&&v| v < r0 || v > r1)
                .copied()
                .collect();

            BoxStats {
                group: group.clone(),
                count: values.len(),
                min,
                max,
                q1,
                median: q2,
                q3,
                iqr,
                whisker_low: r0,
                whisker_high: r1,
                outliers,
            }
        })
        .collect();

    let band = BandScale::new()
        .domain(group_names.clone())
        .range(margin_left, width - margin_right)
        .padding_inner(0.4);

    let band_positions: Vec<f64> = group_names
        .iter()
        .map(|g| band.scale(g).unwrap_or(0.0))
        .collect();

    let all_values: Vec<f64> = data.iter().flat_map(|(_, v)| v.iter().copied()).collect();
    let y_min = all_values.iter().copied().fold(f64::MAX, f64::min);
    let y_max = all_values.iter().copied().fold(f64::MIN, f64::max);

    BoxPlotResult {
        width,
        height,
        groups,
        band_positions,
        bandwidth: band.bandwidth(),
        y_domain: [y_min, y_max],
    }
}
