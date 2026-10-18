//! Hyperstreamlines: integration along a tensor field's major axis.
//!
//! At each point the symmetric tensor's eigenvector with the largest
//! absolute eigenvalue defines an unsigned direction; the tracer keeps
//! sign continuity along the path and integrates with normalised RK2,
//! exactly like the vector streamline tracer.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::eigen::symmetric_eigen;
use crate::streamline::{barycentric, tetrahedralize};
use gpui_mesh::{Association, CellType, Cells, DataSet};

/// Traces hyperstreamlines through a symmetric-tensor point field.
///
/// # Errors
///
/// Returns an error when the field is missing, not a tensor field, or
/// the dataset holds no volume cells.
pub fn hyperstreamlines(
    dataset: &DataSet,
    field_name: &str,
    seeds: &[[f64; 3]],
    step: f64,
    max_steps: usize,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let tensors = field
        .as_tensor()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "hyperstreamlines require a tensor field".into(),
        })?;
    let mut tets: Vec<[u32; 4]> = Vec::new();
    for cell in 0..dataset.cell_count() {
        let kind = dataset.cells.types[cell];
        if kind.is_volume() {
            let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
            tets.extend(tetrahedralize(kind, nodes));
        }
    }
    if tets.is_empty() {
        return Err(TransformError::Degenerate(
            "dataset holds no volume cells".into(),
        ));
    }
    let sample = |p: [f64; 3]| -> Option<[f64; 3]> {
        for tet in &tets {
            let corners = [
                dataset.positions[tet[0] as usize],
                dataset.positions[tet[1] as usize],
                dataset.positions[tet[2] as usize],
                dataset.positions[tet[3] as usize],
            ];
            if let Some(w) = barycentric(corners, p) {
                let mut t = [0.0f64; 6];
                for (i, weight) in w.iter().enumerate() {
                    let value = tensors[tet[i] as usize];
                    for (c, component) in t.iter_mut().enumerate() {
                        *component += weight * value[c];
                    }
                }
                // Field order xx, yy, zz, xy, yz, xz.
                let matrix = [[t[0], t[3], t[5]], [t[3], t[1], t[4]], [t[5], t[4], t[2]]];
                let (values, vectors) = symmetric_eigen(matrix);
                let major = (0..3)
                    .max_by(|&a, &b| {
                        values[a]
                            .abs()
                            .partial_cmp(&values[b].abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap_or(0);
                return Some([vectors[0][major], vectors[1][major], vectors[2][major]]);
            }
        }
        None
    };
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    for seed in seeds {
        for direction_sign in [1.0, -1.0] {
            let mut line: Vec<[f64; 3]> = vec![*seed];
            let mut p = *seed;
            let mut previous: Option<[f64; 3]> = None;
            for _ in 0..max_steps {
                let Some(mut dir) = sample(p) else { break };
                if let Some(prev) = previous
                    && dir[0] * prev[0] + dir[1] * prev[1] + dir[2] * prev[2] < 0.0
                {
                    dir = [-dir[0], -dir[1], -dir[2]];
                }
                let mid = [
                    p[0] + 0.5 * step * direction_sign * dir[0],
                    p[1] + 0.5 * step * direction_sign * dir[1],
                    p[2] + 0.5 * step * direction_sign * dir[2],
                ];
                let Some(mut mid_dir) = sample(mid) else {
                    break;
                };
                if mid_dir[0] * dir[0] + mid_dir[1] * dir[1] + mid_dir[2] * dir[2] < 0.0 {
                    mid_dir = [-mid_dir[0], -mid_dir[1], -mid_dir[2]];
                }
                let next = [
                    p[0] + step * direction_sign * mid_dir[0],
                    p[1] + step * direction_sign * mid_dir[1],
                    p[2] + step * direction_sign * mid_dir[2],
                ];
                if sample(next).is_none() {
                    break;
                }
                previous = Some(mid_dir);
                p = next;
                line.push(p);
            }
            if direction_sign < 0.0 {
                line.reverse();
            }
            if line.len() < 2 {
                continue;
            }
            let base = positions.len() as u32;
            positions.extend(line.iter().copied());
            for i in 0..line.len() - 1 {
                cells.push((CellType::Line, vec![base + i as u32, base + i as u32 + 1]));
            }
        }
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    Ok(DataSet::new(
        format!("{}-hyperstreamlines", dataset.id),
        positions,
        Cells::from_cells(&refs),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Field;

    #[test]
    fn uniaxial_tensor_traces_straight_lines() {
        let ds = DataSet::structured_grid("g", [4, 4, 4], [0.0; 3], [1.0; 3]).unwrap();
        let count = ds.point_count();
        // sigma = diag(4, 1, 1): major axis everywhere +x.
        let ds = ds.with_field(Field::tensor(
            "s",
            vec![[4.0, 1.0, 1.0, 0.0, 0.0, 0.0]; count],
        ));
        let out = hyperstreamlines(&ds, "s", &[[0.2, 1.5, 1.5]], 0.25, 8).unwrap();
        assert!(out.cell_count() > 0);
        assert!(
            out.positions
                .iter()
                .all(|p| (p[1] - 1.5).abs() < 1e-9 && (p[2] - 1.5).abs() < 1e-9)
        );
    }
}
