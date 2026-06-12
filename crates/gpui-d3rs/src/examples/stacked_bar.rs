//! Stacked Bar Chart — <https://observablehq.com/@d3/stacked-bar-chart>
//!
//! Demonstrates: `BandScale`, `LinearScale`, `Stack` with diverging offset.

use crate::scale::BandScale;
use crate::shape::stack::{Stack, StackOffset, StackOrder, StackSeries};

#[derive(Debug)]
pub struct StackedBarResult {
    pub width: f64,
    pub height: f64,
    pub states: Vec<String>,
    pub categories: Vec<String>,
    pub series: Vec<StackSeries>,
    pub band_positions: Vec<f64>,
    pub bandwidth: f64,
    pub y_domain: [f64; 2],
}

/// Default population data (5 states × 8 age groups).
pub fn default_data() -> (Vec<String>, Vec<String>, Vec<Vec<f64>>) {
    let states: Vec<String> = ["California", "Texas", "Florida", "New York", "Illinois"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let ages: Vec<String> = [
        "<10", "10-19", "20-29", "30-39", "40-49", "50-59", "60-69", "70+",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let matrix = vec![
        vec![
            5038.0, 5170.0, 5765.0, 5430.0, 5044.0, 4835.0, 3738.0, 2920.0,
        ],
        vec![
            3983.0, 3862.0, 3872.0, 3678.0, 3360.0, 3092.0, 2388.0, 1708.0,
        ],
        vec![
            2211.0, 2331.0, 2641.0, 2574.0, 2524.0, 2685.0, 2462.0, 2285.0,
        ],
        vec![
            2334.0, 2470.0, 2903.0, 2700.0, 2523.0, 2706.0, 2128.0, 1709.0,
        ],
        vec![
            1625.0, 1710.0, 1826.0, 1699.0, 1591.0, 1688.0, 1259.0, 953.0,
        ],
    ];
    (states, ages, matrix)
}

pub fn compute(states: &[String], categories: &[String], matrix: &[Vec<f64>]) -> StackedBarResult {
    let width = 928.0;
    let height = 500.0;
    let _margin_top = 10.0;
    let margin_right = 10.0;
    let _margin_bottom = 20.0;
    let margin_left = 40.0;

    let stack = Stack::new()
        .keys(categories.to_vec())
        .order(StackOrder::None)
        .offset(StackOffset::Diverging);

    let series = stack.generate(matrix);

    // Band scale for states
    let band = BandScale::new()
        .domain(states.to_vec())
        .range(margin_left, width - margin_right)
        .padding_inner(0.1);

    let band_positions: Vec<f64> = states
        .iter()
        .map(|s| band.scale(s).unwrap_or(0.0))
        .collect();

    // Y extent
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    for s in &series {
        for i in 0..matrix.len() {
            if let Some(v) = s.get(i) {
                y_min = y_min.min(v[0]);
                y_max = y_max.max(v[1]);
            }
        }
    }

    StackedBarResult {
        width,
        height,
        states: states.to_vec(),
        categories: categories.to_vec(),
        series,
        band_positions,
        bandwidth: band.bandwidth(),
        y_domain: [y_min, y_max],
    }
}
