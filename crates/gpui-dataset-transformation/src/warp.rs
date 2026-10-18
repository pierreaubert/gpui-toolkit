//! Geometry warping by fields.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, DataSet, FieldData};

/// Displaces points along a vector field: `p' = p + scale * v`.
/// Masked samples keep their original positions and source validity mask.
///
/// # Errors
///
/// Returns an error for invalid input, nonfinite scale, missing/non-vector
/// fields, or unrepresentable output positions.
pub fn warp_by_vector(
    dataset: &DataSet,
    field_name: &str,
    scale: f64,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    validate_scale(scale)?;
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let vectors = field
        .as_vector()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "warp requires a 3-vector field".into(),
        })?;
    if scale == 0.0
        || field
            .valid
            .as_ref()
            .is_some_and(|mask| mask.iter().all(|v| !v))
    {
        return Ok(dataset.clone());
    }
    let positions: Vec<[f64; 3]> = dataset
        .positions
        .iter()
        .zip(vectors.iter())
        .enumerate()
        .map(|(index, (p, v))| {
            if field.valid.as_ref().is_some_and(|mask| !mask[index]) {
                return Ok(*p);
            }
            finite_position([
                scale.mul_add(v[0], p[0]),
                scale.mul_add(v[1], p[1]),
                scale.mul_add(v[2], p[2]),
            ])
        })
        .collect::<Result<_, _>>()?;
    let mut result = dataset.clone();
    result.positions = positions.into();
    Ok(result)
}

/// Displaces points along their (area-weighted) normals by a scalar field.
///
/// Normals come from the boundary triangles of the dataset, so this works
/// for surface and volume datasets alike: `p' = p + scale * value * n`.
/// Masked samples and points without a surface normal keep their positions.
///
/// # Errors
///
/// Returns an error for invalid input, nonfinite scale, missing/non-scalar
/// fields, or unrepresentable normals/output positions.
pub fn warp_by_scalar(
    dataset: &DataSet,
    field_name: &str,
    scale: f64,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    validate_scale(scale)?;
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let FieldData::Scalar(values) = &field.data else {
        return Err(TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "warp requires a scalar field".into(),
        });
    };
    if scale == 0.0
        || values.iter().enumerate().all(|(index, value)| {
            field.valid.as_ref().is_some_and(|mask| !mask[index]) || *value == 0.0
        })
    {
        return Ok(dataset.clone());
    }
    let triangles = gpui_mesh::surface_chunks(dataset, usize::MAX);
    let mut normals = vec![[0.0f64; 3]; dataset.point_count()];
    for chunk in &triangles {
        for tri in chunk.indices.as_chunks::<3>().0 {
            let ids = [
                chunk.point_ids[tri[0] as usize] as usize,
                chunk.point_ids[tri[1] as usize] as usize,
                chunk.point_ids[tri[2] as usize] as usize,
            ];
            let [a, b, c] = [
                dataset.positions[ids[0]],
                dataset.positions[ids[1]],
                dataset.positions[ids[2]],
            ];
            let n = cross(sub(b, a), sub(c, a));
            for id in ids {
                normals[id][0] += n[0];
                normals[id][1] += n[1];
                normals[id][2] += n[2];
            }
        }
    }
    let positions: Vec<[f64; 3]> = dataset
        .positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if field.valid.as_ref().is_some_and(|mask| !mask[i]) {
                return Ok(*p);
            }
            let n = normals[i];
            let len = crate::calculator::magnitude(n);
            if !len.is_finite() {
                return Err(TransformError::Degenerate(
                    "warp surface normal is not representable".into(),
                ));
            }
            if len == 0.0 {
                return Ok(*p);
            }
            let displacement = scale * values[i];
            finite_position(std::array::from_fn(|axis| {
                let normal = n[axis] / len;
                if displacement.is_finite() {
                    normal.mul_add(displacement, p[axis])
                } else {
                    // FMA can retain a finite result when the full product
                    // overflows before adding an oppositely signed position.
                    (scale * normal).mul_add(values[i], p[axis])
                }
            }))
        })
        .collect::<Result<_, _>>()?;
    let mut result = dataset.clone();
    result.positions = positions.into();
    Ok(result)
}

fn validate_scale(scale: f64) -> Result<(), TransformError> {
    if scale.is_finite() {
        Ok(())
    } else {
        Err(TransformError::Degenerate(
            "warp scale must be finite".into(),
        ))
    }
}

fn finite_position(position: [f64; 3]) -> Result<[f64; 3], TransformError> {
    if position.iter().all(|v| v.is_finite()) {
        Ok(position)
    } else {
        Err(TransformError::Degenerate(
            "warped position is not representable".into(),
        ))
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Field;

    #[test]
    fn warps_along_vectors() {
        let ds = DataSet::surface(
            "s",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        )
        .with_field(Field::vector("d", vec![[0.0, 0.0, 1.0]; 3]));
        let out = warp_by_vector(&ds, "d", 2.0).unwrap();
        assert_eq!(out.positions[0], [0.0, 0.0, 2.0]);
    }
}
