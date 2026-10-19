//! Geometry filters: decimation, smoothing, implicit clipping, merging,
//! and affine transforms.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::threshold::ThresholdMode;
use gpui_mesh::DataSet;
use std::collections::BTreeSet;

pub(crate) fn output_base(points: usize, count: usize) -> Result<u32, TransformError> {
    let error = || TransformError::Degenerate("output exceeds u32 point indices".into());
    let end = points.checked_add(count).ok_or_else(error)?;
    if end == 0 || u32::try_from(end - 1).is_err() {
        return Err(error());
    }
    u32::try_from(points).map_err(|_| error())
}

/// Laplacian smoothing: moves each point toward the average of its
/// edge-connected neighbours by `lambda` per iteration.
///
/// Fields stay attached to their points; only positions move. Boundary
/// behaviour is free (boundary points move with their neighbours).
///
/// # Errors
///
/// Returns an error when the dataset is invalid.
pub fn smooth(
    dataset: &DataSet,
    lambda: f64,
    iterations: usize,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    let mut adjacency: Vec<BTreeSet<u32>> = vec![BTreeSet::new(); dataset.point_count()];
    for cell in 0..dataset.cell_count() {
        let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
        for w in nodes.windows(2) {
            adjacency[w[0] as usize].insert(w[1]);
            adjacency[w[1] as usize].insert(w[0]);
        }
        if nodes.len() > 2
            && let (Some(&first), Some(&last)) = (nodes.first(), nodes.last())
        {
            adjacency[first as usize].insert(last);
            adjacency[last as usize].insert(first);
        }
    }
    let mut positions: Vec<[f64; 3]> = dataset.positions.as_ref().to_vec();
    for _ in 0..iterations {
        let next: Vec<[f64; 3]> = positions
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let neighbours = &adjacency[i];
                if neighbours.is_empty() {
                    return *p;
                }
                let mut avg = [0.0; 3];
                for &n in neighbours {
                    let q = positions[n as usize];
                    avg[0] += q[0];
                    avg[1] += q[1];
                    avg[2] += q[2];
                }
                let inv = 1.0 / neighbours.len() as f64;
                [
                    p[0] + lambda * (avg[0] * inv - p[0]),
                    p[1] + lambda * (avg[1] * inv - p[1]),
                    p[2] + lambda * (avg[2] * inv - p[2]),
                ]
            })
            .collect();
        positions = next;
    }
    let mut out = dataset.clone();
    out.positions = positions.into();
    Ok(out)
}

/// Selects whole cells using a finite sphere and All/Any node semantics.
///
/// Boundary points match both inside and outside selection. No temporary field
/// is added, so user field names cannot collide with implementation state.
/// Fields retain their masks through the common packed cell-selection path.
///
/// # Errors
///
/// Returns an error for invalid data, nonfinite center/radius, negative radius
/// or output index overflow.
pub fn clip_sphere(
    dataset: &DataSet,
    centre: [f64; 3],
    radius: f64,
    keep_inside: bool,
    mode: ThresholdMode,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    if !centre.iter().all(|value| value.is_finite()) || !radius.is_finite() || radius < 0.0 {
        return Err(TransformError::Degenerate(
            "sphere center/radius must be finite and radius nonnegative".into(),
        ));
    }
    let selected = dataset
        .positions
        .iter()
        .map(|point| {
            let distance = crate::calculator::magnitude(std::array::from_fn::<_, 3, _>(|axis| {
                point[axis] - centre[axis]
            }));
            if keep_inside {
                distance <= radius
            } else {
                distance >= radius
            }
        })
        .collect::<Vec<_>>();
    let keep = (0..dataset.cell_count())
        .map(|cell| match mode {
            ThresholdMode::All => dataset
                .cells
                .cell(cell)
                .unwrap()
                .iter()
                .all(|&node| selected[node as usize]),
            ThresholdMode::Any => dataset
                .cells
                .cell(cell)
                .unwrap()
                .iter()
                .any(|&node| selected[node as usize]),
        })
        .collect::<Vec<_>>();
    let mut out = crate::threshold::subset(dataset, &keep)?;
    out.id = format!("{}-sphere-clipped", dataset.id).into();
    Ok(out)
}

/// Applies a row-major affine matrix (`[R|t]` as 3 rows of 4) to positions.
///
/// Fields are carried unchanged: vector fields are *not* rotated, so
/// rotate velocity-like fields separately when that matters.
///
/// # Errors
///
/// Returns an error when the dataset is invalid.
pub fn transform_dataset(
    dataset: &DataSet,
    matrix: [[f64; 4]; 3],
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    let mut out = dataset.clone();
    out.positions = dataset
        .positions
        .iter()
        .map(|p| {
            [
                matrix[0][0] * p[0] + matrix[0][1] * p[1] + matrix[0][2] * p[2] + matrix[0][3],
                matrix[1][0] * p[0] + matrix[1][1] * p[1] + matrix[1][2] * p[2] + matrix[1][3],
                matrix[2][0] * p[0] + matrix[2][1] * p[1] + matrix[2][2] * p[2] + matrix[2][3],
            ]
        })
        .collect::<Vec<_>>()
        .into();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{append_datasets, decimate};

    #[test]
    fn decimate_merges_clustered_points() {
        let ds = DataSet::structured_grid("g", [5, 5, 5], [0.0; 3], [0.25; 3]).unwrap();
        let out = decimate(&ds, 0.6).unwrap();
        assert!(out.point_count() < ds.point_count());
        assert!(out.cell_count() > 0);
        assert_eq!(out.validate(), Ok(()));
    }

    #[test]
    fn smooth_pulls_a_spike_down() {
        let ds = DataSet::surface(
            "s",
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.5, 0.5, 1.0],
            ],
            &[[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]],
        );
        let out = smooth(&ds, 0.5, 3).unwrap();
        assert!(out.positions[4][2] < 1.0);
        assert!(out.positions[4][2] > 0.0);
    }

    #[test]
    fn sphere_clip_and_append_and_transform() {
        let ds = DataSet::structured_grid("g", [4, 4, 4], [-1.5; 3], [1.0; 3]).unwrap();
        let clipped = clip_sphere(&ds, [0.0; 3], 1.2, true, ThresholdMode::All).unwrap();
        assert_eq!(clipped.cell_count(), 1);
        let any = clip_sphere(&ds, [0.0; 3], 1.2, true, ThresholdMode::Any).unwrap();
        assert_eq!(any.cell_count(), ds.cell_count());

        let merged = append_datasets(&ds, &ds).unwrap();
        assert_eq!(merged.point_count(), ds.point_count() * 2);
        assert_eq!(merged.validate(), Ok(()));

        let moved = transform_dataset(
            &ds,
            [
                [1.0, 0.0, 0.0, 10.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
        )
        .unwrap();
        assert_eq!(moved.bounds().unwrap().min[0], 8.5);
    }
}
