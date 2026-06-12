//! Global Temperature Trends — <https://observablehq.com/@d3/global-temperature-trends>
//!
//! Scatter plot of monthly temperature anomalies with diverging color scale.
//! Each data point is a circle colored by its anomaly value (red=warm, blue=cool).

use crate::scale::{LinearScale, Scale};

#[derive(Debug, Clone)]
pub struct TempPoint {
    pub date_index: usize,
    pub value: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug)]
pub struct TempTrendsResult {
    pub width: f64,
    pub height: f64,
    pub points: Vec<TempPoint>,
    pub y_domain: [f64; 2],
    pub max_abs: f64,
    pub radius: f64,
}

/// Parse temperatures.csv: date,value
pub fn load_csv(csv_str: &str) -> Vec<f64> {
    csv_str
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 2 {
                return None;
            }
            cols[1].parse::<f64>().ok()
        })
        .collect()
}

/// Compute temperature trends scatter plot.
pub fn compute(values: &[f64]) -> TempTrendsResult {
    let width = 928.0;
    let height = 600.0;
    let margin_top = 20.0;
    let margin_right = 20.0;
    let margin_bottom = 30.0;
    let margin_left = 40.0;

    let n = values.len();
    let y_min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let y_max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    // Nice the y domain
    let y_lo = (y_min * 10.0).floor() / 10.0;
    let y_hi = (y_max * 10.0).ceil() / 10.0;

    let x_scale = LinearScale::new()
        .domain(0.0, (n - 1) as f64)
        .range(margin_left, width - margin_right);
    let y_scale = LinearScale::new()
        .domain(y_lo, y_hi)
        .range(height - margin_bottom, margin_top);

    let max_abs = values.iter().map(|v| v.abs()).fold(0.0f64, f64::max);

    let points: Vec<TempPoint> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| TempPoint {
            date_index: i,
            value: v,
            x: x_scale.scale(i as f64),
            y: y_scale.scale(v),
        })
        .collect();

    TempTrendsResult {
        width,
        height,
        points,
        y_domain: [y_lo, y_hi],
        max_abs,
        radius: 2.5,
    }
}
