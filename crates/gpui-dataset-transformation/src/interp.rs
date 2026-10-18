//! Point-field interpolation from sparse per-point weights.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{DataSet, Field, FieldData};

/// Sparse interpolation weights for one output point.
pub(crate) type Weights = Vec<(u32, f64)>;

/// Single-source weights for an unmodified dataset point.
pub(crate) fn single(point: u32) -> Weights {
    vec![(point, 1.0)]
}

/// Blends two weight sets at factor `t` (0 = `a`, 1 = `b`).
pub(crate) fn blend(a: &Weights, b: &Weights, t: f64) -> Weights {
    let mut out: Weights = a.iter().map(|(i, w)| (*i, w * (1.0 - t))).collect();
    for (i, w) in b {
        let scaled = w * t;
        if let Some(entry) = out.iter_mut().find(|(j, _)| j == i) {
            entry.1 += scaled;
        } else {
            out.push((*i, scaled));
        }
    }
    out
}

fn interpolate<'a, const N: usize>(
    field: &'a Field,
    values: &'a [[f64; N]],
    weights: &'a [Weights],
) -> impl Iterator<Item = [f64; N]> + 'a {
    weights.iter().map(|weights| {
        let mut supported = false;
        for &(index, weight) in weights {
            if weight == 0.0 {
                continue;
            }
            if !weight.is_finite() || !field.is_valid(index as usize) {
                return [f64::NAN; N];
            }
            supported = true;
        }
        if !supported {
            return [f64::NAN; N];
        }
        let mut out = [0.0; N];
        for &(index, weight) in weights {
            // In particular, never evaluate 0 * NaN at an exact endpoint.
            if weight != 0.0 {
                for (target, source) in out.iter_mut().zip(values[index as usize]) {
                    *target += weight * source;
                }
            }
        }
        out
    })
}

/// Interpolates point fields, requiring every nonzero contributor to be valid.
///
/// Invalid support or nonfinite output becomes a masked NaN. Zero weights do
/// not consult missing source entries, preserving exact endpoint values. This
/// helper produces only point fields; cell fields need separate cell provenance.
pub(crate) fn interpolate_point_fields(dataset: &DataSet, weights: &[Weights]) -> Vec<Field> {
    dataset
        .point_data
        .iter()
        .map(|field| {
            let data = match &field.data {
                FieldData::Scalar(values) => FieldData::Scalar(
                    interpolate(field, values.as_chunks::<1>().0, weights)
                        .map(|entry| entry[0])
                        .collect::<Vec<_>>()
                        .into(),
                ),
                FieldData::Vector(values) => FieldData::Vector(
                    interpolate(field, values, weights)
                        .collect::<Vec<_>>()
                        .into(),
                ),
                FieldData::Tensor(values) => FieldData::Tensor(
                    interpolate(field, values, weights)
                        .collect::<Vec<_>>()
                        .into(),
                ),
                FieldData::Complex(values) => FieldData::Complex(
                    interpolate(field, values, weights)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            };
            crate::validity::finish(
                Field {
                    name: field.name.clone(),
                    label: field.label.clone(),
                    unit: field.unit.clone(),
                    association: field.association,
                    data,
                    valid: None,
                },
                |_| true,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Cells;

    #[test]
    fn zero_weights_preserve_endpoints_and_missing_support_stays_missing() {
        let mut dataset = DataSet::new("interpolation", vec![[0.0; 3]; 2], Cells::from_cells(&[]));
        for data in [
            FieldData::Scalar(vec![10.0, f64::NAN].into()),
            FieldData::Vector(vec![[10.0; 3], [f64::NAN; 3]].into()),
            FieldData::Tensor(vec![[10.0; 6], [f64::NAN; 6]].into()),
            FieldData::Complex(vec![[10.0; 2], [f64::NAN; 2]].into()),
        ] {
            let mut field = Field::scalar(
                format!("field-{}", dataset.point_data.len()),
                Vec::<f64>::new(),
            );
            field.data = data;
            field.valid = Some(vec![true, false].into());
            dataset.point_data.push(field);
        }
        dataset.validate().unwrap();
        let fields = interpolate_point_fields(
            &dataset,
            &[
                vec![(0, 1.0), (1, 0.0)],
                vec![(0, 0.5), (1, 0.5)],
                vec![(1, 1.0)],
                vec![],
            ],
        );
        let mut output = DataSet::new("out", vec![[0.0; 3]; 4], Cells::from_cells(&[]));
        output.point_data = fields;
        output.validate().unwrap();
        for field in output.point_data {
            assert!(field.is_valid(0));
            assert!((1..4).all(|index| !field.is_valid(index)));
            assert_eq!(
                field.valid.as_deref(),
                Some([true, false, false, false].as_slice())
            );
        }
    }
}
