//! Line Chart — <https://observablehq.com/@d3/line-chart>
//!
//! Demonstrates: `LinearScale`, `d3.line()` with multiple curve types.

use crate::scale::{LinearScale, Scale};
use crate::shape::curve::Curve;
use crate::shape::path::PathBuilder;

#[derive(Debug, Clone)]
pub struct LineChartResult {
    pub width: f64,
    pub height: f64,
    pub x_domain: [f64; 2],
    pub y_domain: [f64; 2],
    pub paths: Vec<(String, String)>, // (curve_name, svg_path)
}

/// Default temperature-like data (30 points).
pub fn default_data() -> Vec<(f64, f64)> {
    let n = 30;
    (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            let v = 15.0
                + 10.0 * (t * 2.0 * std::f64::consts::PI).sin()
                + 3.0 * (t * 4.0 * std::f64::consts::PI).cos()
                + 2.0 * (i as f64 * 1.7).sin();
            (i as f64, (v * 100.0).round() / 100.0)
        })
        .collect()
}

/// Compute line chart with multiple curve types.
pub fn compute(data: &[(f64, f64)]) -> LineChartResult {
    let width = 928.0;
    let height = 500.0;
    let margin_top = 20.0;
    let margin_right = 30.0;
    let margin_bottom = 30.0;
    let margin_left = 40.0;

    let x_ext = data
        .iter()
        .fold((f64::MAX, f64::MIN), |a, d| (a.0.min(d.0), a.1.max(d.0)));
    let y_max = data.iter().fold(f64::NEG_INFINITY, |a, d| a.max(d.1));

    let x_scale = LinearScale::new()
        .domain(x_ext.0, x_ext.1)
        .range(margin_left, width - margin_right);

    let y_scale = LinearScale::new()
        .domain(0.0, y_max)
        .nice(None)
        .range(height - margin_bottom, margin_top);

    // Project data points
    let points: Vec<crate::shape::path::Point> = data
        .iter()
        .map(|(x, y)| crate::shape::path::Point::new(x_scale.scale(*x), y_scale.scale(*y)))
        .collect();

    let curves: Vec<(&str, Curve)> = vec![
        ("linear", Curve::linear()),
        ("step", Curve::Step),
        ("basis", Curve::basis()),
        ("cardinal", Curve::cardinal(0.0)),
        ("natural", Curve::natural()),
        ("monotoneX", Curve::monotone_x()),
        ("catmullRom", Curve::catmull_rom(0.5)),
    ];

    let paths: Vec<(String, String)> = curves
        .into_iter()
        .map(|(name, curve)| {
            let interpolated = curve.interpolate(&points);
            let mut builder = PathBuilder::new();
            for (i, p) in interpolated.iter().enumerate() {
                if i == 0 {
                    builder = builder.move_to(p.x, p.y);
                } else {
                    builder = builder.line_to(p.x, p.y);
                }
            }
            (name.to_string(), builder.build().to_svg_string())
        })
        .collect();

    LineChartResult {
        width,
        height,
        x_domain: [x_ext.0, x_ext.1],
        y_domain: [0.0, y_scale.domain_max()],
        paths,
    }
}
