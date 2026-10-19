//! Scalar probing through compact volume stencils or nearest surface points.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::stencil::StencilLocator;
use gpui_mesh::{Association, DataSet, Field};
use std::cmp::Ordering;

/// Interpolates a scalar point field at arbitrary finite probe points.
///
/// Volume interpolation requires all four tetrahedron nodes to be valid. A
/// compact cell index preserves original-cell selection priority. Outside a
/// volume or missing support yields `None`. Surface data uses the nearest point,
/// including its validity, without substituting a more distant valid sample.
/// Nonfinite queries/results yield `None`.
///
/// # Errors
///
/// Returns an error for invalid datasets or missing/non-scalar fields.
pub fn probe(
    dataset: &DataSet,
    field_name: &str,
    points: &[[f64; 3]],
) -> Result<Vec<Option<f64>>, TransformError> {
    dataset.validate()?;
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.into()))?;
    if field.as_scalar().is_none() {
        return Err(TransformError::WrongFieldShape {
            field: field_name.into(),
            reason: "probe requires a scalar field".into(),
        });
    }
    Ok(samples(dataset, field, points).collect())
}

/// Borrows a validated scalar field and streams samples without an output array.
pub(crate) fn samples<'a>(
    dataset: &'a DataSet,
    field: &'a Field,
    points: &'a [[f64; 3]],
) -> impl Iterator<Item = Option<f64>> + 'a {
    let values = field.as_scalar().expect("validated scalar probe field");
    // Small batches cannot amortize index construction. Bounded paired probes
    // exercise both sides of this cutoff; the linear path needs no index storage.
    const LINEAR_QUERIES: usize = 64;
    let all_missing = field
        .valid
        .as_ref()
        .is_some_and(|mask| mask.iter().all(|v| !v));
    let has_volume = dataset.cells.types.iter().any(|kind| kind.is_volume());
    let locator = (has_volume && !all_missing && points.len() > LINEAR_QUERIES)
        .then(|| StencilLocator::new(dataset, field.valid.as_deref()));
    points.iter().map(move |&point| {
        if all_missing || !point.iter().all(|v| v.is_finite()) {
            return None;
        }
        if has_volume {
            let (nodes, weights) = match &locator {
                Some(locator) => locator.weights(point),
                None => crate::stencil::linear_weights(dataset, field.valid.as_deref(), point),
            }?;
            crate::stencil::sample_values(values.as_chunks::<1>().0, nodes, weights)
                .map(|value| value[0])
        } else {
            dataset
                .positions
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| compare_distance(**a, **b, point))
                .and_then(|(index, _)| field.is_valid(index).then_some(values[index]))
        }
    })
}

fn compare_distance(a: [f64; 3], b: [f64; 3], point: [f64; 3]) -> Ordering {
    let mut da = std::array::from_fn::<_, 3, _>(|i| a[i] - point[i]);
    let mut db = std::array::from_fn::<_, 3, _>(|i| b[i] - point[i]);
    let squared = |delta: [f64; 3]| delta.iter().map(|v| v * v).sum::<f64>();
    let sa = squared(da);
    let sb = squared(db);
    let ordinary =
        |value: f64, delta: [f64; 3]| value.is_normal() || (value == 0.0 && delta == [0.0; 3]);
    if ordinary(sa, da) && ordinary(sb, db) {
        return sa.total_cmp(&sb);
    }
    // Normalize both candidates together. Only an overflowing subtraction needs
    // half scaling; ordinary/subnormal coordinate differences retain precision.
    if da.iter().chain(&db).any(|v| !v.is_finite()) {
        da = std::array::from_fn(|i| a[i] * 0.5 - point[i] * 0.5);
        db = std::array::from_fn(|i| b[i] * 0.5 - point[i] * 0.5);
    }
    let scale = da
        .iter()
        .chain(&db)
        .fold(0.0_f64, |scale, v| scale.max(v.abs()));
    if scale == 0.0 {
        return Ordering::Equal;
    }
    let norm = |delta: [f64; 3]| {
        delta
            .into_iter()
            .fold(0.0_f64, |length, v| length.hypot(v / scale))
    };
    norm(da).total_cmp(&norm(db))
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
