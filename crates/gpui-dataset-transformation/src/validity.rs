//! Validity completion for newly computed fields.

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
