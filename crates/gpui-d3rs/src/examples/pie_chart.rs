//! Pie Chart — <https://observablehq.com/@d3/pie-chart>
//!
//! Demonstrates: `Pie` layout, `Arc` path generation, `LinearScale` for color.
//!
//! ```rust,no_run
//! let result = d3rs::examples::pie_chart::compute(&d3rs::examples::pie_chart::DEFAULT_DATA);
//! assert_eq!(result.slices.len(), 5);
//! ```

use crate::shape::arc::Arc;
use crate::shape::pie::Pie;

/// A single data item for the pie chart.
#[derive(Debug, Clone)]
pub struct DataItem {
    pub name: String,
    pub value: f64,
}

/// Computed pie slice with geometry.
#[derive(Debug, Clone)]
pub struct SliceResult {
    pub name: String,
    pub value: f64,
    pub index: usize,
    pub start_angle: f64,
    pub end_angle: f64,
    pub arc_path: String,
    pub centroid: [f64; 2],
}

/// Full pie chart computation result.
#[derive(Debug)]
pub struct PieChartResult {
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub slices: Vec<SliceResult>,
    pub total_value: f64,
}

/// Default dataset (energy consumption by sector).
pub const DEFAULT_DATA: &[(&str, f64)] = &[
    ("Residential", 48.5),
    ("Commercial", 18.6),
    ("Industrial", 13.1),
    ("Transportation", 11.3),
    ("Other", 8.5),
];

/// Compute the pie chart from labeled value pairs.
pub fn compute(data: &[(&str, f64)]) -> PieChartResult {
    let width: f64 = 928.0;
    let height: f64 = 500.0;
    let radius = width.min(height) / 2.0;

    let items: Vec<DataItem> = data
        .iter()
        .map(|(n, v)| DataItem {
            name: n.to_string(),
            value: *v,
        })
        .collect();

    let pie = Pie::new().outer_radius(radius - 1.0).sort(false);
    let values: Vec<f64> = items.iter().map(|d| d.value).collect();
    let slices = pie.generate(&values, |v| *v);

    let arc = Arc::new();

    let results: Vec<SliceResult> = slices
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let centroid = s.arc.centroid();
            SliceResult {
                name: items[i].name.clone(),
                value: items[i].value,
                index: i,
                start_angle: s.arc.start_angle,
                end_angle: s.arc.end_angle,
                arc_path: arc.path_string(&s.arc),
                centroid: [centroid.x, centroid.y],
            }
        })
        .collect();

    PieChartResult {
        width,
        height,
        radius,
        slices: results,
        total_value: values.iter().sum(),
    }
}
