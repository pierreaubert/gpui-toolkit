//! Derived-field calculators.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, DataSet, Field, FieldData};

pub(crate) fn magnitude<const N: usize>(components: [f64; N]) -> f64 {
    let squared: f64 = components.iter().map(|v| v * v).sum();
    if squared.is_normal() || components.iter().all(|&v| v == 0.0) {
        squared.sqrt()
    } else {
        components.into_iter().fold(0.0, f64::hypot)
    }
}

fn point_field<'a>(dataset: &'a DataSet, name: &str) -> Result<&'a Field, TransformError> {
    dataset.validate()?;
    dataset
        .field(Association::Point, name)
        .ok_or_else(|| TransformError::FieldNotFound(name.to_string()))
}

fn with_point_scalar(dataset: &DataSet, name: &str, values: Vec<f64>, out_name: &str) -> DataSet {
    let mut result = dataset.clone();
    result.point_data.retain(|f| f.name.as_ref() != out_name);
    let mut field = Field::scalar(out_name.to_string(), values);
    if let Some(source) = dataset.field(Association::Point, name) {
        field.label = source.label.clone();
        field.unit.clone_from(&source.unit);
    }
    let source = dataset
        .field(Association::Point, name)
        .expect("checked source field");
    field.valid.clone_from(&source.valid);
    result
        .point_data
        .push(crate::validity::finish(field, |index| {
            source.is_valid(index)
        }));
    result
}

/// Appends vector magnitude, preserving missing entries as masked NaNs.
///
/// # Errors
///
/// Returns an error for an invalid dataset or a missing/non-vector field.
pub fn vector_magnitude(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let field = point_field(dataset, field_name)?;
    let vectors = field
        .as_vector()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "expected a 3-vector field".into(),
        })?;
    let values: Vec<f64> = vectors
        .iter()
        .enumerate()
        .map(|(index, v)| {
            if field.is_valid(index) {
                magnitude(*v)
            } else {
                f64::NAN
            }
        })
        .collect();
    Ok(with_point_scalar(dataset, field_name, values, out_name))
}

/// Appends complex magnitude, preserving missing entries as masked NaNs.
///
/// # Errors
///
/// Returns an error for an invalid dataset or a missing/non-complex field.
pub fn complex_magnitude(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let field = point_field(dataset, field_name)?;
    let FieldData::Complex(values) = &field.data else {
        return Err(TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "expected a complex field".into(),
        });
    };
    let values: Vec<f64> = values
        .iter()
        .enumerate()
        .map(|(index, c)| {
            if field.is_valid(index) {
                magnitude(*c)
            } else {
                f64::NAN
            }
        })
        .collect();
    Ok(with_point_scalar(dataset, field_name, values, out_name))
}

/// Appends complex phase in radians, preserving missing entries as masked NaNs.
///
/// # Errors
///
/// Returns an error for an invalid dataset or a missing/non-complex field.
pub fn complex_phase(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let field = point_field(dataset, field_name)?;
    let FieldData::Complex(values) = &field.data else {
        return Err(TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "expected a complex field".into(),
        });
    };
    let values: Vec<f64> = values
        .iter()
        .enumerate()
        .map(|(index, c)| {
            if field.is_valid(index) {
                c[1].atan2(c[0])
            } else {
                f64::NAN
            }
        })
        .collect();
    let mut output = with_point_scalar(dataset, field_name, values, out_name);
    output
        .point_data
        .last_mut()
        .expect("derived field appended")
        .unit = Some("rad".into());
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magnitude_of_velocity() {
        let ds = DataSet::surface(
            "s",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        )
        .with_field(Field::vector("velocity", vec![[3.0, 4.0, 0.0]; 3]));
        let out = vector_magnitude(&ds, "velocity", "speed").unwrap();
        let speed = out.field(Association::Point, "speed").unwrap();
        assert_eq!(speed.as_scalar().unwrap(), &[5.0, 5.0, 5.0]);
    }
}
