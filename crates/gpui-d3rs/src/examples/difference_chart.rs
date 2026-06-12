//! Difference Chart — <https://observablehq.com/@d3/difference-chart/2>
//!
//! Demonstrates: `Area` generator with two y baselines to show the
//! difference between two temperature series (value0 vs value1).

use crate::scale::{LinearScale, Scale};
use crate::shape::path::{Path, PathBuilder};

#[derive(Debug, Clone)]
pub struct DiffChartRow {
    pub date_index: usize,
    pub value0: f64,
    pub value1: f64,
}

#[derive(Debug)]
pub struct DiffChartResult {
    pub width: f64,
    pub height: f64,
    /// Area path where value0 > value1 (clipped above)
    pub above_path: Path,
    /// Area path where value1 > value0 (clipped below)
    pub below_path: Path,
    /// Line path for value0
    pub line0_path: Path,
    /// Line path for value1
    pub line1_path: Path,
    pub x_domain: [f64; 2],
    pub y_domain: [f64; 2],
}

/// Parse SFO temperature CSV: date,value0,value1
pub fn load_csv(csv_str: &str) -> Vec<DiffChartRow> {
    csv_str
        .lines()
        .skip(1)
        .enumerate()
        .filter_map(|(i, line)| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 3 {
                return None;
            }
            let value0: f64 = cols[1].parse().ok()?;
            let value1: f64 = cols[2].parse().ok()?;
            Some(DiffChartRow {
                date_index: i,
                value0,
                value1,
            })
        })
        .collect()
}

/// Compute difference chart from two-series data.
pub fn compute(data: &[DiffChartRow]) -> DiffChartResult {
    let width = 928.0;
    let height = 500.0;
    let margin_top = 20.0;
    let margin_right = 20.0;
    let margin_bottom = 30.0;
    let margin_left = 40.0;

    if data.is_empty() {
        let empty = PathBuilder::new().build();
        return DiffChartResult {
            width,
            height,
            above_path: empty.clone(),
            below_path: empty.clone(),
            line0_path: empty.clone(),
            line1_path: empty,
            x_domain: [0.0, 0.0],
            y_domain: [0.0, 0.0],
        };
    }

    let n = data.len();
    let y_min = data
        .iter()
        .flat_map(|d| [d.value0, d.value1])
        .fold(f64::INFINITY, f64::min);
    let y_max = data
        .iter()
        .flat_map(|d| [d.value0, d.value1])
        .fold(f64::NEG_INFINITY, f64::max);

    let x_scale = LinearScale::new()
        .domain(0.0, (n - 1) as f64)
        .range(margin_left, width - margin_right);
    let y_scale = LinearScale::new()
        .domain(y_min, y_max)
        .range(height - margin_bottom, margin_top);

    // Step curve helper: for curveStep, at each data point we draw a horizontal
    // segment to the midpoint between previous and current x, then a vertical
    // segment to the new y. This matches D3's curveStep exactly.
    let step_x = |i: usize| -> f64 { x_scale.scale(i as f64) };

    // Build step-curve forward path for a series
    let build_step_forward = |values: &[f64]| -> Vec<(f64, f64)> {
        let mut pts = Vec::with_capacity(values.len() * 2);
        for (i, &v) in values.iter().enumerate() {
            let x = step_x(i);
            let y = y_scale.scale(v);
            if i == 0 {
                pts.push((x, y));
            } else {
                // Step: horizontal to midpoint, then vertical
                let prev_x = step_x(i - 1);
                let mid_x = (prev_x + x) / 2.0;
                let prev_y = pts.last().unwrap().1;
                pts.push((mid_x, prev_y));
                pts.push((mid_x, y));
                pts.push((x, y));
            }
        }
        pts
    };

    let v0_values: Vec<f64> = data.iter().map(|d| d.value0).collect();
    let v1_values: Vec<f64> = data.iter().map(|d| d.value1).collect();
    let min_values: Vec<f64> = data.iter().map(|d| d.value0.min(d.value1)).collect();

    let v0_step = build_step_forward(&v0_values);
    let v1_step = build_step_forward(&v1_values);
    let min_step = build_step_forward(&min_values);

    // "Above" area: value0 on top, min(v0,v1) on bottom
    // Shows where SF (value0) is warmer than NY (value1)
    let mut above_builder = PathBuilder::new();
    for (i, &(x, y)) in v0_step.iter().enumerate() {
        if i == 0 {
            above_builder = above_builder.move_to(x, y);
        } else {
            above_builder = above_builder.line_to(x, y);
        }
    }
    for &(x, y) in min_step.iter().rev() {
        above_builder = above_builder.line_to(x, y);
    }
    above_builder = above_builder.close_path();

    // "Below" area: value1 on top, min(v0,v1) on bottom
    // Shows where NY (value1) is warmer than SF (value0)
    let mut below_builder = PathBuilder::new();
    for (i, &(x, y)) in v1_step.iter().enumerate() {
        if i == 0 {
            below_builder = below_builder.move_to(x, y);
        } else {
            below_builder = below_builder.line_to(x, y);
        }
    }
    for &(x, y) in min_step.iter().rev() {
        below_builder = below_builder.line_to(x, y);
    }
    below_builder = below_builder.close_path();

    // Line path for value0 (SF reference line) — step curve
    let mut line0 = PathBuilder::new();
    for (i, &(x, y)) in v0_step.iter().enumerate() {
        if i == 0 {
            line0 = line0.move_to(x, y);
        } else {
            line0 = line0.line_to(x, y);
        }
    }

    // Line path for value1 (NY) — step curve
    let mut line1 = PathBuilder::new();
    for (i, &(x, y)) in v1_step.iter().enumerate() {
        if i == 0 {
            line1 = line1.move_to(x, y);
        } else {
            line1 = line1.line_to(x, y);
        }
    }

    DiffChartResult {
        width,
        height,
        above_path: above_builder.build(),
        below_path: below_builder.build(),
        line0_path: line0.build(),
        line1_path: line1.build(),
        x_domain: [0.0, (n - 1) as f64],
        y_domain: [y_min, y_max],
    }
}
