//! Donut Chart — <https://observablehq.com/@d3/donut-chart>
//!
//! Demonstrates: `Pie` with inner radius, `Arc` path generation, pad angle.

use crate::shape::arc::Arc;
use crate::shape::pie::Pie;

/// Computed donut slice with geometry.
#[derive(Debug, Clone)]
pub struct SliceResult {
    pub name: String,
    pub value: f64,
    pub index: usize,
    pub start_angle: f64,
    pub end_angle: f64,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub arc_path: String,
    pub centroid: [f64; 2],
}

/// Full donut chart computation result.
#[derive(Debug)]
pub struct DonutChartResult {
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub inner_radius: f64,
    pub pad_angle: f64,
    pub slices: Vec<SliceResult>,
    pub total_value: f64,
}

pub const DEFAULT_DATA: &[(&str, f64)] = &[
    ("JavaScript", 67.7),
    ("Python", 44.1),
    ("TypeScript", 34.8),
    ("Java", 33.3),
    ("C#", 27.6),
    ("Rust", 13.0),
    ("Go", 11.2),
];

pub fn compute(data: &[(&str, f64)]) -> DonutChartResult {
    let width: f64 = 928.0;
    let height: f64 = 500.0;
    let radius = width.min(height) / 2.0;
    let inner_radius = radius * 0.67;
    let pad_angle = 1.0 / radius;

    let pie = Pie::new()
        .inner_radius(inner_radius)
        .outer_radius(radius - 1.0)
        .pad_angle(pad_angle)
        .sort(false);

    let values: Vec<f64> = data.iter().map(|(_, v)| *v).collect();
    let slices = pie.generate(&values, |v| *v);
    let arc = Arc::new();

    let results: Vec<SliceResult> = slices
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let centroid = s.arc.centroid();
            SliceResult {
                name: data[i].0.to_string(),
                value: data[i].1,
                index: i,
                start_angle: s.arc.start_angle,
                end_angle: s.arc.end_angle,
                inner_radius: s.arc.inner_radius,
                outer_radius: s.arc.outer_radius,
                arc_path: arc.path_string(&s.arc),
                centroid: [centroid.x, centroid.y],
            }
        })
        .collect();

    DonutChartResult {
        width,
        height,
        radius,
        inner_radius,
        pad_angle,
        slices: results,
        total_value: values.iter().sum(),
    }
}
