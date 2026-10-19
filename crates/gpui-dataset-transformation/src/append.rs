//! Checked dataset concatenation with explicit missing-field support.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Cells, DataSet, Field, FieldData};
use std::collections::HashMap;
use std::sync::Arc;

/// Concatenates datasets, preserving supported fields and marking absent samples missing.
///
/// Points/cells retain input order; second-input connectivity is offset. Shared
/// named fields require identical shape and unit. Field order/metadata follow
/// the first input, followed by second-only fields. Missing sides receive masked
/// NaNs for all scalar/vector/tensor/complex point/cell shapes. Existing masked
/// payloads remain unspecified. Empty-side buffers can share source storage.
///
/// # Errors
///
/// Returns an error for invalid input, incompatible fields, size overflow or
/// combined point indices exceeding u32.
pub fn append_datasets(a: &DataSet, b: &DataSet) -> Result<DataSet, TransformError> {
    a.validate()?;
    b.validate()?;
    let points = total(a.point_count(), b.point_count())?;
    if points > 0 {
        crate::geometry::output_base(0, points)?;
    }
    let cells = total(a.cell_count(), b.cell_count())?;
    let offsets_count = total(cells, 1)?;
    let connectivity_count = total(a.cells.connectivity.len(), b.cells.connectivity.len())?;
    let point_plan = plan(&a.point_data, &b.point_data)?;
    let cell_plan = plan(&a.cell_data, &b.cell_data)?;
    let positions = join_present(&a.positions, &b.positions);
    let cells = if b.cells.is_empty() {
        a.cells.clone()
    } else if a.point_count() == 0 {
        b.cells.clone()
    } else {
        let types = join_present(&a.cells.types, &b.cells.types);
        let mut offsets = Vec::with_capacity(offsets_count);
        offsets.extend_from_slice(&a.cells.offsets);
        offsets.extend(
            b.cells
                .offsets
                .iter()
                .skip(1)
                .map(|&offset| offset + a.cells.connectivity.len() as u64),
        );
        let mut connectivity = Vec::with_capacity(connectivity_count);
        connectivity.extend_from_slice(&a.cells.connectivity);
        connectivity.extend(b.cells.connectivity.iter().map(|&node| {
            u32::try_from(node as usize + a.point_count()).expect("checked combined index bounds")
        }));
        Cells {
            types,
            offsets: offsets.into(),
            connectivity: connectivity.into(),
        }
    };
    let mut out = DataSet::new(format!("{}+{}", a.id, b.id), positions, cells);
    out.point_data = point_plan
        .into_iter()
        .map(|pair| merge(pair, a.point_count(), b.point_count()))
        .collect();
    out.cell_data = cell_plan
        .into_iter()
        .map(|pair| merge(pair, a.cell_count(), b.cell_count()))
        .collect();
    Ok(out)
}

fn total(a: usize, b: usize) -> Result<usize, TransformError> {
    a.checked_add(b)
        .ok_or_else(|| TransformError::Degenerate("appended dataset size overflows usize".into()))
}

#[derive(Clone, Copy)]
struct Pair<'a> {
    a: Option<&'a Field>,
    b: Option<&'a Field>,
}

fn check(a: &Field, b: &Field) -> Result<(), TransformError> {
    if std::mem::discriminant(&a.data) != std::mem::discriminant(&b.data) || a.unit != b.unit {
        Err(TransformError::WrongFieldShape {
            field: a.name.to_string(),
            reason: "appended field shapes and units must match".into(),
        })
    } else {
        Ok(())
    }
}

fn plan<'a>(a: &'a [Field], b: &'a [Field]) -> Result<Vec<Pair<'a>>, TransformError> {
    if a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.name == b.name) {
        return a
            .iter()
            .zip(b)
            .map(|(a, b)| {
                check(a, b)?;
                Ok(Pair {
                    a: Some(a),
                    b: Some(b),
                })
            })
            .collect();
    }
    // Small ordinary schemas avoid hashing; large/reordered schemas avoid
    // quadratic searches. Names remain borrowed throughout planning.
    let index = (a.len() > 16 || b.len() > 16).then(|| {
        b.iter()
            .enumerate()
            .map(|(i, f)| (f.name.as_ref(), i))
            .collect::<HashMap<_, _>>()
    });
    let mut seen = vec![false; b.len()];
    let mut result = Vec::with_capacity(total(a.len(), b.len())?);
    for field in a {
        let matched = match &index {
            Some(index) => index.get(field.name.as_ref()).copied(),
            None => b.iter().position(|b| b.name == field.name),
        };
        let other = matched.map(|i| {
            seen[i] = true;
            &b[i]
        });
        if let Some(other) = other {
            check(field, other)?;
        }
        result.push(Pair {
            a: Some(field),
            b: other,
        });
    }
    result.extend(
        b.iter()
            .zip(seen)
            .filter(|(_, seen)| !*seen)
            .map(|(field, _)| Pair {
                a: None,
                b: Some(field),
            }),
    );
    Ok(result)
}

fn join_present<T: Copy>(a: &Arc<[T]>, b: &Arc<[T]>) -> Arc<[T]> {
    if a.is_empty() {
        return b.clone();
    }
    if b.is_empty() {
        return a.clone();
    }
    let mut values = Vec::with_capacity(
        a.len()
            .checked_add(b.len())
            .expect("checked combined counts"),
    );
    values.extend_from_slice(a);
    values.extend_from_slice(b);
    values.into()
}

fn join<T: Copy>(
    a: Option<&Arc<[T]>>,
    b: Option<&Arc<[T]>>,
    a_count: usize,
    b_count: usize,
    missing: T,
) -> Arc<[T]> {
    if let (Some(a), Some(b)) = (a, b) {
        return join_present(a, b);
    }
    if a_count == 0
        && let Some(b) = b
    {
        return b.clone();
    }
    if b_count == 0
        && let Some(a) = a
    {
        return a.clone();
    }
    let mut values = Vec::with_capacity(a_count.checked_add(b_count).expect("checked field count"));
    if let Some(a) = a {
        values.extend_from_slice(a);
    } else {
        values.resize(a_count, missing);
    }
    if let Some(b) = b {
        values.extend_from_slice(b);
    } else {
        values.resize(a_count + b_count, missing);
    }
    values.into()
}

fn merge(pair: Pair<'_>, a_count: usize, b_count: usize) -> Field {
    let Pair { a, b } = pair;
    let template = a.or(b).expect("at least one field defines the union");
    let data = match &template.data {
        FieldData::Scalar(_) => FieldData::Scalar(join(
            a.and_then(|f| {
                if let FieldData::Scalar(v) = &f.data {
                    Some(v)
                } else {
                    None
                }
            }),
            b.and_then(|f| {
                if let FieldData::Scalar(v) = &f.data {
                    Some(v)
                } else {
                    None
                }
            }),
            a_count,
            b_count,
            f64::NAN,
        )),
        FieldData::Vector(_) => FieldData::Vector(join(
            a.and_then(|f| {
                if let FieldData::Vector(v) = &f.data {
                    Some(v)
                } else {
                    None
                }
            }),
            b.and_then(|f| {
                if let FieldData::Vector(v) = &f.data {
                    Some(v)
                } else {
                    None
                }
            }),
            a_count,
            b_count,
            [f64::NAN; 3],
        )),
        FieldData::Tensor(_) => FieldData::Tensor(join(
            a.and_then(|f| {
                if let FieldData::Tensor(v) = &f.data {
                    Some(v)
                } else {
                    None
                }
            }),
            b.and_then(|f| {
                if let FieldData::Tensor(v) = &f.data {
                    Some(v)
                } else {
                    None
                }
            }),
            a_count,
            b_count,
            [f64::NAN; 6],
        )),
        FieldData::Complex(_) => FieldData::Complex(join(
            a.and_then(|f| {
                if let FieldData::Complex(v) = &f.data {
                    Some(v)
                } else {
                    None
                }
            }),
            b.and_then(|f| {
                if let FieldData::Complex(v) = &f.data {
                    Some(v)
                } else {
                    None
                }
            }),
            a_count,
            b_count,
            [f64::NAN; 2],
        )),
    };
    let valid = if a_count == 0 {
        b.and_then(|f| f.valid.clone())
    } else if b_count == 0 {
        a.and_then(|f| f.valid.clone())
    } else {
        let missing = |f: Option<&Field>| {
            f.is_none_or(|f| {
                f.valid
                    .as_ref()
                    .is_some_and(|mask| mask.iter().any(|v| !*v))
            })
        };
        if !missing(a) && !missing(b) {
            None
        } else {
            let mut mask = Vec::with_capacity(a_count + b_count);
            for (field, count) in [(a, a_count), (b, b_count)] {
                if let Some(valid) = field.and_then(|field| field.valid.as_ref()) {
                    mask.extend_from_slice(valid);
                } else {
                    mask.resize(mask.len() + count, field.is_some());
                }
            }
            Some(mask.into())
        }
    };
    // When a defining field is empty and absent from the nonempty side, that
    // side still needs an explicit all-missing mask.
    let valid = if (a_count > 0 && a.is_none()) || (b_count > 0 && b.is_none()) {
        valid.or_else(|| Some(vec![false; a_count + b_count].into()))
    } else {
        valid
    };
    Field {
        name: template.name.clone(),
        label: template.label.clone(),
        unit: template.unit.clone(),
        association: template.association,
        data,
        valid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combined_size_checks_do_not_allocate() {
        assert_eq!(total(0, 0).unwrap(), 0);
        assert_eq!(total(usize::MAX, 0).unwrap(), usize::MAX);
        total(usize::MAX, 1).unwrap_err();
        if usize::BITS > 32 {
            crate::geometry::output_base(u32::MAX as usize, 1).unwrap();
        }
        crate::geometry::output_base(usize::MAX, 1).unwrap_err();
    }
}
