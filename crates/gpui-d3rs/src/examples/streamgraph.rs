//! Streamgraph — <https://observablehq.com/@d3/streamgraph>
//!
//! Demonstrates: `Stack` with `Wiggle` offset and `InsideOut` order.

use crate::shape::stack::{Stack, StackOffset, StackOrder, StackSeries};

#[derive(Debug)]
pub struct StreamgraphResult {
    pub width: f64,
    pub height: f64,
    pub categories: Vec<String>,
    pub series: Vec<StackSeries>,
    pub y_extent: [f64; 2],
}

/// Default multi-series time data (5 categories, 20 time steps).
pub fn default_data() -> (Vec<String>, Vec<Vec<f64>>) {
    let categories: Vec<String> = ["alpha", "beta", "gamma", "delta", "epsilon"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let n = 20;
    let mut matrix = Vec::with_capacity(n);
    for t in 0..n {
        let mut row = Vec::with_capacity(categories.len());
        for c in 0..categories.len() {
            let base = 10.0 + 5.0 * c as f64;
            let val = base
                + 8.0 * (t as f64 * 0.5 + c as f64 * 1.3).sin()
                + 3.0 * (t as f64 * 0.3 + c as f64 * 0.7).cos();
            row.push(val.max(0.0));
        }
        matrix.push(row);
    }
    (categories, matrix)
}

/// Compute streamgraph stack layout.
pub fn compute(categories: &[String], matrix: &[Vec<f64>]) -> StreamgraphResult {
    let width = 928.0;
    let height = 500.0;

    let stack = Stack::new()
        .keys(categories.to_vec())
        .order(StackOrder::InsideOut)
        .offset(StackOffset::Wiggle);

    let series = stack.generate(matrix);

    // Compute y extent across all series
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

    StreamgraphResult {
        width,
        height,
        categories: categories.to_vec(),
        series,
        y_extent: [y_min, y_max],
    }
}
