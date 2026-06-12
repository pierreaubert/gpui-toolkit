//! Chord Diagram — <https://observablehq.com/@d3/chord-diagram>
//!
//! Demonstrates: `ChordLayout`, group arcs, ribbon paths.

use crate::chord::{ChordLayout, ChordResult};

#[derive(Debug)]
pub struct ChordDiagramResult {
    pub width: f64,
    pub height: f64,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub names: Vec<String>,
    pub chord_result: ChordResult,
    pub pad_angle: f64,
}

/// Phone market share matrix (from Observable chord diagram).
pub fn default_matrix() -> (Vec<String>, Vec<Vec<f64>>) {
    let names: Vec<String> = [
        "Apple", "HTC", "Huawei", "LG", "Nokia", "Samsung", "Sony", "Other",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let matrix = vec![
        vec![
            0.096899, 0.008859, 0.000554, 0.004430, 0.025471, 0.024363, 0.005537, 0.025471,
        ],
        vec![
            0.001107, 0.018272, 0.000000, 0.004983, 0.011074, 0.010520, 0.002215, 0.004983,
        ],
        vec![
            0.000554, 0.002769, 0.002215, 0.002215, 0.003876, 0.008306, 0.000554, 0.003322,
        ],
        vec![
            0.000554, 0.001107, 0.000554, 0.012182, 0.011628, 0.006645, 0.004983, 0.010520,
        ],
        vec![
            0.002215, 0.004430, 0.000000, 0.002769, 0.104097, 0.012182, 0.004983, 0.028239,
        ],
        vec![
            0.011628, 0.026024, 0.000000, 0.013843, 0.087486, 0.168328, 0.017165, 0.055925,
        ],
        vec![
            0.000554, 0.004983, 0.000000, 0.003322, 0.004430, 0.008859, 0.017719, 0.004430,
        ],
        vec![
            0.002215, 0.007198, 0.000000, 0.003322, 0.016611, 0.014950, 0.001107, 0.054264,
        ],
    ];

    (names, matrix)
}

pub fn compute(names: &[String], matrix: &[Vec<f64>]) -> ChordDiagramResult {
    let width: f64 = 928.0;
    let height: f64 = 928.0;
    let outer_radius = width.min(height) * 0.5 - 60.0;
    let inner_radius = outer_radius - 10.0;
    let pad_angle = 10.0 / inner_radius;

    let layout = ChordLayout::new()
        .pad_angle(pad_angle)
        .sort_subgroups(|a, b| b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal));
    let chord_result = layout.compute(matrix);

    ChordDiagramResult {
        width,
        height,
        inner_radius,
        outer_radius,
        names: names.to_vec(),
        chord_result,
        pad_angle,
    }
}
