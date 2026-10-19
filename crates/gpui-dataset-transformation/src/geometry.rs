//! Geometry filters: decimation, smoothing, implicit clipping, merging,
//! and affine transforms.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::threshold::{ThresholdMode, threshold};
use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData};
use std::collections::{BTreeSet, HashMap};

pub(crate) fn output_base(points: usize, count: usize) -> Result<u32, TransformError> {
    let error = || TransformError::Degenerate("output exceeds u32 point indices".into());
    let end = points.checked_add(count).ok_or_else(error)?;
    if end == 0 || u32::try_from(end - 1).is_err() {
        return Err(error());
    }
    u32::try_from(points).map_err(|_| error())
}

/// Decimates by vertex clustering: points falling into the same
/// `cell_size` grid cube merge into their centroid.
///
/// Point fields are averaged per cluster; cells that collapse (repeated
/// nodes after remapping) are dropped with their cell fields. This is
/// the fast, topology-agnostic decimation used for previews and LOD;
/// it does not preserve features the way quadric decimation does.
///
/// # Errors
///
/// Returns an error for a non-positive cell size.
pub fn decimate(dataset: &DataSet, cell_size: f64) -> Result<DataSet, TransformError> {
    if cell_size <= 0.0 {
        return Err(TransformError::Degenerate(
            "decimation cell size must be positive".into(),
        ));
    }
    let mut clusters: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut assignment: Vec<u32> = Vec::with_capacity(dataset.point_count());
    for p in dataset.positions.iter() {
        let key = (
            (p[0] / cell_size).floor() as i64,
            (p[1] / cell_size).floor() as i64,
            (p[2] / cell_size).floor() as i64,
        );
        let next = clusters.len() as u32;
        let id = *clusters.entry(key).or_insert(next);
        assignment.push(id);
    }
    let count = clusters.len();
    let mut positions = vec![[0.0f64; 3]; count];
    let mut members = vec![0usize; count];
    for (p, &cluster) in dataset.positions.iter().zip(assignment.iter()) {
        let entry = &mut positions[cluster as usize];
        entry[0] += p[0];
        entry[1] += p[1];
        entry[2] += p[2];
        members[cluster as usize] += 1;
    }
    for (p, n) in positions.iter_mut().zip(members.iter()) {
        let inv = 1.0 / *n as f64;
        p[0] *= inv;
        p[1] *= inv;
        p[2] *= inv;
    }
    // Remap cells, dropping collapsed ones.
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    let mut kept: Vec<usize> = Vec::new();
    for cell in 0..dataset.cell_count() {
        let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
        let remapped: Vec<u32> = nodes.iter().map(|&n| assignment[n as usize]).collect();
        let unique: BTreeSet<u32> = remapped.iter().copied().collect();
        if unique.len() == remapped.len() && remapped.len() >= 2 {
            cells.push((dataset.cells.types[cell], remapped));
            kept.push(cell);
        }
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    let mut out = DataSet::new(
        format!("{}-decimated", dataset.id),
        positions,
        Cells::from_cells(&refs),
    );
    for field in &dataset.point_data {
        out.point_data
            .push(average_field(field, &assignment, count));
    }
    for field in &dataset.cell_data {
        out.cell_data.push(subset_field(field, &kept));
    }
    Ok(out)
}

fn average_field(field: &Field, assignment: &[u32], count: usize) -> Field {
    let data = match &field.data {
        FieldData::Scalar(values) => {
            let mut acc = vec![0.0; count];
            let mut n = vec![0.0; count];
            for (v, &c) in values.iter().zip(assignment.iter()) {
                acc[c as usize] += v;
                n[c as usize] += 1.0;
            }
            FieldData::Scalar(
                acc.iter()
                    .zip(n.iter())
                    .map(|(a, c)| if *c > 0.0 { a / c } else { 0.0 })
                    .collect::<Vec<_>>()
                    .into(),
            )
        }
        FieldData::Vector(values) => {
            let mut acc = vec![[0.0; 3]; count];
            let mut n = vec![0.0; count];
            for (v, &c) in values.iter().zip(assignment.iter()) {
                acc[c as usize][0] += v[0];
                acc[c as usize][1] += v[1];
                acc[c as usize][2] += v[2];
                n[c as usize] += 1.0;
            }
            FieldData::Vector(
                acc.iter()
                    .zip(n.iter())
                    .map(|(a, c)| {
                        if *c > 0.0 {
                            [a[0] / c, a[1] / c, a[2] / c]
                        } else {
                            [0.0; 3]
                        }
                    })
                    .collect::<Vec<_>>()
                    .into(),
            )
        }
        other => other.clone(),
    };
    Field {
        name: field.name.clone(),
        label: field.label.clone(),
        unit: field.unit.clone(),
        association: field.association,
        data,
        valid: None,
    }
}

fn subset_field(field: &Field, kept: &[usize]) -> Field {
    let data = match &field.data {
        FieldData::Scalar(values) => {
            FieldData::Scalar(kept.iter().map(|&i| values[i]).collect::<Vec<_>>().into())
        }
        FieldData::Vector(values) => {
            FieldData::Vector(kept.iter().map(|&i| values[i]).collect::<Vec<_>>().into())
        }
        other => other.clone(),
    };
    Field {
        name: field.name.clone(),
        label: field.label.clone(),
        unit: field.unit.clone(),
        association: field.association,
        data,
        valid: None,
    }
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

/// Clips cells by a sphere: keeps cells whose nodes lie inside
/// (`keep_inside`) or outside the sphere, under `mode` node semantics.
///
/// Implemented as a signed-distance field plus [`threshold`], so cells
/// are kept whole — the boundary is not re-triangulated.
///
/// # Errors
///
/// Returns an error when the input is invalid.
pub fn clip_sphere(
    dataset: &DataSet,
    centre: [f64; 3],
    radius: f64,
    keep_inside: bool,
    mode: ThresholdMode,
) -> Result<DataSet, TransformError> {
    let distance: Vec<f64> = dataset
        .positions
        .iter()
        .map(|p| {
            let d = ((p[0] - centre[0]).powi(2)
                + (p[1] - centre[1]).powi(2)
                + (p[2] - centre[2]).powi(2))
            .sqrt();
            if keep_inside { d - radius } else { radius - d }
        })
        .collect();
    let mut staged = dataset.clone();
    staged
        .point_data
        .push(Field::scalar("__sphere_distance", distance));
    let mut out = threshold(
        &staged,
        "__sphere_distance",
        Association::Point,
        f64::MIN,
        0.0,
        mode,
    )?;
    out.point_data
        .retain(|f| f.name.as_ref() != "__sphere_distance");
    out.id = format!("{}-sphere-clipped", dataset.id).into();
    Ok(out)
}

/// Appends two datasets: points and cells concatenated, indices remapped.
///
/// Point and cell fields present in both inputs with the same shape are
/// concatenated; a field missing on one side is filled with zeros there.
/// A shape mismatch is an error.
///
/// # Errors
///
/// Returns an error when a shared field has incompatible shapes.
pub fn append_datasets(a: &DataSet, b: &DataSet) -> Result<DataSet, TransformError> {
    let offset = a.point_count() as u32;
    let mut positions: Vec<[f64; 3]> = a.positions.iter().copied().collect();
    positions.extend(b.positions.iter().copied());
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    for cell in 0..a.cell_count() {
        cells.push((
            a.cells.types[cell],
            a.cells.cell(cell).unwrap_or(&[]).to_vec(),
        ));
    }
    for cell in 0..b.cell_count() {
        cells.push((
            b.cells.types[cell],
            b.cells
                .cell(cell)
                .unwrap_or(&[])
                .iter()
                .map(|n| n + offset)
                .collect(),
        ));
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    let mut out = DataSet::new(
        format!("{}+{}", a.id, b.id),
        positions,
        Cells::from_cells(&refs),
    );
    out.point_data = merge_fields(
        &a.point_data,
        &b.point_data,
        a.point_count(),
        b.point_count(),
    )?;
    out.cell_data = merge_fields(&a.cell_data, &b.cell_data, a.cell_count(), b.cell_count())?;
    Ok(out)
}

fn merge_fields(
    a: &[Field],
    b: &[Field],
    a_count: usize,
    b_count: usize,
) -> Result<Vec<Field>, TransformError> {
    let mut out = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for field in a.iter().chain(b.iter()) {
        if !names.contains(&field.name.to_string()) {
            names.push(field.name.to_string());
        }
    }
    for name in names {
        let fa = a.iter().find(|f| f.name.as_ref() == name);
        let fb = b.iter().find(|f| f.name.as_ref() == name);
        let template = fa.or(fb).unwrap();
        let zeros = |count: usize| -> FieldData {
            match &template.data {
                FieldData::Scalar(_) => FieldData::Scalar(vec![0.0; count].into()),
                FieldData::Vector(_) => FieldData::Vector(vec![[0.0; 3]; count].into()),
                FieldData::Tensor(_) => FieldData::Tensor(vec![[0.0; 6]; count].into()),
                FieldData::Complex(_) => FieldData::Complex(vec![[0.0; 2]; count].into()),
            }
        };
        let data_a = fa.map_or_else(|| zeros(a_count), |f| f.data.clone());
        let data_b = fb.map_or_else(|| zeros(b_count), |f| f.data.clone());
        let data = match (data_a, data_b) {
            (FieldData::Scalar(mut x), FieldData::Scalar(y)) => {
                let merged: Vec<f64> = x.iter().copied().chain(y.iter().copied()).collect();
                x = merged.into();
                FieldData::Scalar(x)
            }
            (FieldData::Vector(x), FieldData::Vector(y)) => FieldData::Vector(
                x.iter()
                    .copied()
                    .chain(y.iter().copied())
                    .collect::<Vec<_>>()
                    .into(),
            ),
            (FieldData::Tensor(x), FieldData::Tensor(y)) => FieldData::Tensor(
                x.iter()
                    .copied()
                    .chain(y.iter().copied())
                    .collect::<Vec<_>>()
                    .into(),
            ),
            (FieldData::Complex(x), FieldData::Complex(y)) => FieldData::Complex(
                x.iter()
                    .copied()
                    .chain(y.iter().copied())
                    .collect::<Vec<_>>()
                    .into(),
            ),
            _ => {
                return Err(TransformError::WrongFieldShape {
                    field: name.clone(),
                    reason: "field shape differs between appended datasets".into(),
                });
            }
        };
        out.push(Field {
            name: template.name.clone(),
            label: template.label.clone(),
            unit: template.unit.clone(),
            association: template.association,
            data,
            valid: None,
        });
    }
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
