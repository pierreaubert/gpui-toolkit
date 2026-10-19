//! Hyperstreamlines: integration along a tensor field's major axis.
//!
//! At each point the symmetric tensor's eigenvector with the largest
//! absolute eigenvalue defines an unsigned direction; the tracer keeps
//! sign continuity along the path and integrates with normalised RK2,
//! exactly like the vector streamline tracer.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::eigen::symmetric_eigen;
use crate::stencil::{StencilLocator, sample_values};
use gpui_mesh::{Association, Cells, DataSet};

/// Traces hyperstreamlines through a symmetric-tensor point field.
/// All four stencil nodes must be valid. Missing or zero tensors stop paths;
/// wholly missing input yields an empty dataset.
///
/// # Errors
///
/// Returns an error for invalid input/seeds, nonfinite/non-positive step,
/// missing/non-tensor fields, no volume cells or excessive output point indices.
pub fn hyperstreamlines(
    dataset: &DataSet,
    field_name: &str,
    seeds: &[[f64; 3]],
    step: f64,
    max_steps: usize,
) -> Result<DataSet, TransformError> {
    if !step.is_finite() || step <= 0.0 {
        return Err(TransformError::Degenerate(
            "hyperstreamline step must be finite and positive".into(),
        ));
    }
    crate::streamline::validate_seeds(seeds)?;
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
    let locator = StencilLocator::new(dataset, field.valid.as_deref());
    if !locator.has_volume() {
        return Err(TransformError::Degenerate(
            "dataset holds no volume cells".into(),
        ));
    }
    let sample = |p: [f64; 3]| -> Option<[f64; 3]> {
        let (tet, weights) = locator.weights(p)?;
        let t = sample_values(tensors, tet, weights)?;
        // Field order xx, yy, zz, xy, yz, xz.
        let matrix = [[t[0], t[3], t[5]], [t[3], t[1], t[4]], [t[5], t[4], t[2]]];
        let (values, vectors) = symmetric_eigen(matrix);
        if !values
            .iter()
            .chain(vectors.iter().flatten())
            .all(|v| v.is_finite())
            || values.iter().all(|v| *v == 0.0)
        {
            return None;
        }
        let major = (0..3)
            .max_by(|&a, &b| {
                values[a]
                    .abs()
                    .partial_cmp(&values[b].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(0);
        Some([vectors[0][major], vectors[1][major], vectors[2][major]])
    };
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut cells = Vec::new();
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
                if next == p || sample(next).is_none() {
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
            let base = crate::geometry::output_base(positions.len(), line.len())?;
            positions.extend(line.iter().copied());
            for i in 0..line.len() - 1 {
                cells.push([base + i as u32, base + i as u32 + 1]);
            }
        }
    }
    Ok(DataSet::new(
        format!("{}-hyperstreamlines", dataset.id),
        positions,
        Cells::lines(&cells),
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
