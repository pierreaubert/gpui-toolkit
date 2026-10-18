//! Point probing: interpolated field values at arbitrary locations.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, DataSet};

use gpui_mesh::tetrahedra as tetrahedralize;

fn barycentric(tet: [[f64; 3]; 4], p: [f64; 3]) -> Option<[f64; 4]> {
    let (a, b, c, d) = (tet[0], tet[1], tet[2], tet[3]);
    let m = [
        [b[0] - a[0], c[0] - a[0], d[0] - a[0]],
        [b[1] - a[1], c[1] - a[1], d[1] - a[1]],
        [b[2] - a[2], c[2] - a[2], d[2] - a[2]],
    ];
    let rhs = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-300 {
        return None;
    }
    let solve = |col: usize| -> f64 {
        let mut mm = m;
        for row in 0..3 {
            mm[row][col] = rhs[row];
        }
        (mm[0][0] * (mm[1][1] * mm[2][2] - mm[1][2] * mm[2][1])
            - mm[0][1] * (mm[1][0] * mm[2][2] - mm[1][2] * mm[2][0])
            + mm[0][2] * (mm[1][0] * mm[2][1] - mm[1][1] * mm[2][0]))
            / det
    };
    let w = [solve(0), solve(1), solve(2)];
    let weights = [1.0 - w[0] - w[1] - w[2], w[0], w[1], w[2]];
    weights.iter().all(|&x| x >= -1e-9).then_some(weights)
}

/// Interpolates a scalar point field at probe points.
///
/// Volume datasets use barycentric interpolation inside the containing
/// tetrahedron. Surface datasets fall back to the nearest point. Points
/// outside a volume dataset yield `None`. Volume interpolation requires all four
/// tetrahedron nodes to be valid; surface probing returns `None` when the nearest
/// node is missing, rather than substituting a more distant sample. Nonfinite
/// query points or computed results also yield `None`.
///
/// # Errors
///
/// Returns an error for an invalid dataset or a missing/non-scalar field.
pub fn probe(
    dataset: &DataSet,
    field_name: &str,
    points: &[[f64; 3]],
) -> Result<Vec<Option<f64>>, TransformError> {
    dataset.validate()?;
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let values = field
        .as_scalar()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "probe requires a scalar field".into(),
        })?;
    let has_volume = dataset.cells.types.iter().any(|kind| kind.is_volume());
    let mut tets: Vec<[u32; 4]> = Vec::new();
    for cell in 0..dataset.cell_count() {
        let kind = dataset.cells.types[cell];
        if kind.is_volume() {
            let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
            tets.extend(tetrahedralize(kind, nodes).filter(|tet| {
                field.valid.is_none() || tet.iter().all(|&node| field.is_valid(node as usize))
            }));
        }
    }
    let mut out = Vec::with_capacity(points.len());
    for &point in points {
        if !point.iter().all(|v| v.is_finite()) {
            out.push(None);
            continue;
        }
        if !has_volume {
            // Nearest-point fallback for surface datasets.
            let nearest = dataset
                .positions
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    dist2(**a, point)
                        .partial_cmp(&dist2(**b, point))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .and_then(|(i, _)| field.is_valid(i).then_some(values[i]));
            out.push(nearest);
            continue;
        }
        let mut found = None;
        for tet in &tets {
            let corners = [
                dataset.positions[tet[0] as usize],
                dataset.positions[tet[1] as usize],
                dataset.positions[tet[2] as usize],
                dataset.positions[tet[3] as usize],
            ];
            if let Some(w) = barycentric(corners, point) {
                let value: f64 = w
                    .iter()
                    .enumerate()
                    .map(|(i, weight)| weight * values[tet[i] as usize])
                    .sum();
                found = value.is_finite().then_some(value);
                break;
            }
        }
        out.push(found);
    }
    Ok(out)
}

fn dist2(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Field;

    #[test]
    fn probes_a_linear_field_exactly() {
        let mut ds = DataSet::structured_grid("g", [3, 3, 3], [0.0; 3], [1.0; 3]).unwrap();
        let values: Vec<f64> = ds.positions.iter().map(|p| p[0] + 2.0 * p[1]).collect();
        ds.point_data.push(Field::scalar("f", values));
        let out = probe(&ds, "f", &[[0.5, 0.5, 0.5], [9.0, 9.0, 9.0]]).unwrap();
        assert_eq!(out[0], Some(1.5));
        assert_eq!(out[1], None);
    }
}
