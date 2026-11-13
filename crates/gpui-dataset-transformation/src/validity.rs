//! Validity completion for newly computed fields, and the shared on-disk
//! validity convention for mesh IO.
//!
//! Mesh IO convention (VTK, VTU, XDMF): formats have no native validity
//! mask, so a field with any masked entry is written with masked entries
//! canonicalized to NaN plus a companion scalar field `<name>__valid`
//! holding 1.0/0.0 per entry. Readers fold the companion back into
//! [`Field::valid`] and drop it; bare non-finite entries without a
//! companion (foreign files, older exports) are likewise read as masked
//! NaNs so they validate as missing data instead of failing or silently
//! becoming measurements. Geometry-only surface formats (STL/OBJ/PLY)
//! export no fields at all and are outside this convention.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{Field, FieldData};
use std::sync::Arc;

/// Marks missing/nonfinite results and canonicalizes them to masked NaNs.
///
/// Fully valid results need no mask allocation. The predicate expresses each
/// operation's dependency rule; finite placeholder values alone never establish
/// validity. An existing mask is shared when it exactly describes the result.
/// This consumes newly computed storage, avoiding source mutation.
pub(crate) fn finish(mut field: Field, supported: impl Fn(usize) -> bool) -> Field {
    let count = field.data.len();
    let valid = |index| supported(index) && field.data.entry_is_finite(index);
    let Some(first_invalid) = (0..count).find(|&index| !valid(index)) else {
        field.valid = None;
        return field;
    };
    let mask: Arc<[bool]> = if let Some(mask) = field.valid.as_ref().filter(|mask| {
        mask.len() == count
            && mask
                .iter()
                .enumerate()
                .all(|(index, &entry)| entry == valid(index))
    }) {
        mask.clone()
    } else {
        let mut mask = vec![true; count];
        for (index, entry) in mask.iter_mut().enumerate().skip(first_invalid) {
            *entry = valid(index);
        }
        mask.into()
    };
    match &mut field.data {
        FieldData::Scalar(values) => {
            for (value, &valid) in Arc::make_mut(values).iter_mut().zip(mask.iter()) {
                if !valid {
                    *value = f64::NAN;
                }
            }
        }
        FieldData::Vector(values) => {
            for (value, &valid) in Arc::make_mut(values).iter_mut().zip(mask.iter()) {
                if !valid {
                    *value = [f64::NAN; 3];
                }
            }
        }
        FieldData::Tensor(values) => {
            for (value, &valid) in Arc::make_mut(values).iter_mut().zip(mask.iter()) {
                if !valid {
                    *value = [f64::NAN; 6];
                }
            }
        }
        FieldData::Complex(values) => {
            for (value, &valid) in Arc::make_mut(values).iter_mut().zip(mask.iter()) {
                if !valid {
                    *value = [f64::NAN; 2];
                }
            }
        }
    }
    field.valid = Some(mask);
    field
}

/// Suffix naming the on-disk validity companion of a field.
pub(crate) const VALID_SUFFIX: &str = "__valid";

/// Companion field name carrying `field`'s mask on disk.
pub(crate) fn mask_name(field_name: &str) -> String {
    format!("{field_name}{VALID_SUFFIX}")
}

/// The mask worth exporting: present and masking at least one entry.
pub(crate) fn active_mask(field: &Field) -> Option<&[bool]> {
    field
        .valid
        .as_deref()
        .filter(|mask| mask.iter().any(|valid| !valid))
}

/// Whether entry `index` is masked (short masks fail closed, as `is_valid`).
pub(crate) fn is_masked(field: &Field, index: usize) -> bool {
    field
        .valid
        .as_ref()
        .is_some_and(|mask| mask.get(index) != Some(&true))
}

/// Flat component values with masked entries canonicalized to NaN.
pub(crate) fn canonical_flat(field: &Field) -> Vec<f64> {
    let mut flat = Vec::with_capacity(field.data.len() * field.data.components());
    match &field.data {
        FieldData::Scalar(values) => {
            for (index, value) in values.iter().enumerate() {
                flat.push(if is_masked(field, index) {
                    f64::NAN
                } else {
                    *value
                });
            }
        }
        FieldData::Vector(values) => {
            for (index, value) in values.iter().enumerate() {
                if is_masked(field, index) {
                    flat.extend_from_slice(&[f64::NAN; 3]);
                } else {
                    flat.extend_from_slice(value);
                }
            }
        }
        FieldData::Tensor(values) => {
            for (index, value) in values.iter().enumerate() {
                if is_masked(field, index) {
                    flat.extend_from_slice(&[f64::NAN; 6]);
                } else {
                    flat.extend_from_slice(value);
                }
            }
        }
        FieldData::Complex(values) => {
            for (index, value) in values.iter().enumerate() {
                if is_masked(field, index) {
                    flat.extend_from_slice(&[f64::NAN; 2]);
                } else {
                    flat.extend_from_slice(value);
                }
            }
        }
    }
    flat
}

/// Folds `__valid` companions into masks and canonicalizes bare
/// non-finite entries to masked NaNs, in place.
pub(crate) fn restore_masks(fields: &mut Vec<Field>) {
    let mut index = 0;
    while index < fields.len() {
        let Some(base) = fields[index].name.strip_suffix(VALID_SUFFIX) else {
            index += 1;
            continue;
        };
        let base = base.to_string();
        let Some(companion) = fields[index].as_scalar().map(<[f64]>::to_vec) else {
            index += 1;
            continue;
        };
        let Some(target) = fields
            .iter_mut()
            .find(|field| field.name.as_ref() == base && field.data.len() == companion.len())
        else {
            index += 1;
            continue;
        };
        let mask: Vec<bool> = companion
            .iter()
            .map(|value| value.is_finite() && *value != 0.0)
            .collect();
        // finish_read below collapses an all-true mask to None.
        target.valid = Some(mask.into());
        fields.remove(index);
    }
    for field in fields.iter_mut() {
        finish_read(field);
    }
}

/// Canonicalizes one read field: non-finite entries become masked NaNs
/// and an all-true mask collapses to `None`.
fn finish_read(field: &mut Field) {
    let count = field.data.len();
    let mask: Vec<bool> = (0..count)
        .map(|index| !is_masked(field, index) && field.data.entry_is_finite(index))
        .collect();
    if mask.iter().all(|valid| *valid) {
        field.valid = None;
        return;
    }
    match &mut field.data {
        FieldData::Scalar(values) => {
            for (value, &valid) in Arc::make_mut(values).iter_mut().zip(mask.iter()) {
                if !valid {
                    *value = f64::NAN;
                }
            }
        }
        FieldData::Vector(values) => {
            for (value, &valid) in Arc::make_mut(values).iter_mut().zip(mask.iter()) {
                if !valid {
                    *value = [f64::NAN; 3];
                }
            }
        }
        FieldData::Tensor(values) => {
            for (value, &valid) in Arc::make_mut(values).iter_mut().zip(mask.iter()) {
                if !valid {
                    *value = [f64::NAN; 6];
                }
            }
        }
        FieldData::Complex(values) => {
            for (value, &valid) in Arc::make_mut(values).iter_mut().zip(mask.iter()) {
                if !valid {
                    *value = [f64::NAN; 2];
                }
            }
        }
    }
    field.valid = Some(mask.into());
}
