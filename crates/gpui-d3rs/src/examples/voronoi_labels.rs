//! Voronoi Labels — <https://observablehq.com/@d3/voronoi-labels>
//!
//! Scatter plot where labels are shown only when the Voronoi cell is large enough.
//! Label orientation is determined by the angle to the cell centroid.

use crate::delaunay::Delaunay;
use crate::shape::path::{Path, PathBuilder};

#[derive(Debug, Clone)]
pub struct LabeledPoint {
    pub index: usize,
    pub x: f64,
    pub y: f64,
    pub show_label: bool,
    pub label_anchor: LabelAnchor,
    pub cell_area: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LabelAnchor {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug)]
pub struct VoronoiLabelsResult {
    pub width: f64,
    pub height: f64,
    pub points: Vec<LabeledPoint>,
    pub voronoi_mesh: Path,
    pub point_count: usize,
    pub label_count: usize,
}

/// Parse voronoi.csv: x,y (no header).
pub fn load_csv(csv_str: &str) -> Vec<(f64, f64)> {
    csv_str
        .lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 2 {
                return None;
            }
            let x: f64 = cols[0].trim().parse().ok()?;
            let y: f64 = cols[1].trim().parse().ok()?;
            Some((x, y))
        })
        .collect()
}

/// Compute Voronoi labels for a set of 2D points.
pub fn compute(coords: &[(f64, f64)]) -> VoronoiLabelsResult {
    let width = 928.0;
    let height = 600.0;
    let min_area = 2000.0;

    let delaunay = Delaunay::new(coords);
    let voronoi = delaunay.voronoi(Some([0.0, 0.0, width, height]));

    let mut points = Vec::new();
    let mut mesh_builder = PathBuilder::new();
    let mut label_count = 0;

    for (i, &(x, y)) in coords.iter().enumerate() {
        let (show_label, anchor, area) = if let Some(cell) = voronoi.cell_polygon(i) {
            // Compute area (shoelace formula)
            let n = cell.len();
            let mut a = 0.0;
            for j in 0..n {
                let k = (j + 1) % n;
                a += cell[j].0 * cell[k].1;
                a -= cell[k].0 * cell[j].1;
            }
            let area = a.abs() / 2.0;

            // Compute centroid
            let cx: f64 = cell.iter().map(|p| p.0).sum::<f64>() / n as f64;
            let cy: f64 = cell.iter().map(|p| p.1).sum::<f64>() / n as f64;

            // Label anchor based on angle to centroid
            let angle = ((cy - y).atan2(cx - x) / std::f64::consts::PI * 2.0).round() as i32;
            let anchor = match angle {
                3 | -3 => LabelAnchor::Top,
                0 => LabelAnchor::Right,
                1 | -1 => LabelAnchor::Bottom,
                _ => LabelAnchor::Left,
            };

            // Draw cell edges for mesh
            for j in 0..n {
                let k = (j + 1) % n;
                mesh_builder = mesh_builder.move_to(cell[j].0, cell[j].1);
                mesh_builder = mesh_builder.line_to(cell[k].0, cell[k].1);
            }

            (area > min_area, anchor, area)
        } else {
            (false, LabelAnchor::Right, 0.0)
        };

        if show_label {
            label_count += 1;
        }

        points.push(LabeledPoint {
            index: i,
            x,
            y,
            show_label,
            label_anchor: anchor,
            cell_area: area,
        });
    }

    VoronoiLabelsResult {
        width,
        height,
        points,
        voronoi_mesh: mesh_builder.build(),
        point_count: coords.len(),
        label_count,
    }
}
