//! Validity-aware cell selection with packed output storage.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, Cells, DataSet, Field, FieldData};

/// How point values decide whether a cell survives a threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdMode {
    /// Every point (or the cell value) must lie in range.
    All,
    /// At least one point (or the cell value) must lie in range.
    Any,
}

/// Keeps cells whose supported scalar values lie in `[lower,upper]`.
///
/// Invalid nodes never match; All rejects any invalid node. Fields and points
/// are subset consistently, preserving first-use point order and source masks.
/// Infinite bounds are allowed. NaN/reversed bounds return an error.
///
/// # Errors
///
/// Returns an error for invalid datasets, bounds, missing/non-scalar fields or
/// output indices beyond the u32 representation.
pub fn threshold(
    dataset: &DataSet,
    field_name: &str,
    association: Association,
    lower: f64,
    upper: f64,
    mode: ThresholdMode,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    if lower.is_nan() || upper.is_nan() || lower > upper {
        return Err(TransformError::Degenerate(
            "threshold bounds must be ordered and not NaN".into(),
        ));
    }
    let field = dataset
        .field(association, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.into()))?;
    let values = field
        .as_scalar()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.into(),
            reason: "threshold requires a scalar field".into(),
        })?;
    let matches =
        |index: usize| field.is_valid(index) && values[index] >= lower && values[index] <= upper;
    let keep = (0..dataset.cell_count())
        .map(|cell| match association {
            Association::Cell => matches(cell),
            Association::Point => match mode {
                ThresholdMode::All => dataset
                    .cells
                    .cell(cell)
                    .unwrap()
                    .iter()
                    .all(|&node| matches(node as usize)),
                ThresholdMode::Any => dataset
                    .cells
                    .cell(cell)
                    .unwrap()
                    .iter()
                    .any(|&node| matches(node as usize)),
            },
        })
        .collect::<Vec<_>>();
    subset(dataset, &keep)
}

/// Extracts kept cells from validated input, with one shared point-order map.
pub(crate) fn subset(dataset: &DataSet, keep: &[bool]) -> Result<DataSet, TransformError> {
    debug_assert_eq!(keep.len(), dataset.cell_count());
    // A common identity selection shares all storage. A first-visit sequence
    // reaches the next index exactly; lower indices were already encountered.
    if keep.iter().all(|keep| *keep) {
        let mut next = 0usize;
        let ordered = dataset.cells.connectivity.iter().all(|&node| {
            let node = node as usize;
            if node == next {
                next += 1;
                true
            } else {
                node < next
            }
        });
        if ordered && next == dataset.point_count() {
            return Ok(dataset.clone());
        }
    }
    let count = keep.iter().filter(|&&kept| kept).count();
    if count == 0 {
        let mut result = DataSet::new(dataset.id.clone(), Vec::new(), Cells::from_cells(&[]));
        result.point_data = dataset
            .point_data
            .iter()
            .map(|field| pick(field, &[]))
            .collect();
        result.cell_data = dataset
            .cell_data
            .iter()
            .map(|field| pick(field, &[]))
            .collect();
        return Ok(result);
    }
    let mut remap = vec![usize::MAX; dataset.point_count()];
    let mut order = Vec::new();
    let mut kept = Vec::with_capacity(count);
    let connectivity_count = keep
        .iter()
        .enumerate()
        .filter(|(_, kept)| **kept)
        .map(|(cell, _)| dataset.cells.cell(cell).unwrap().len())
        .sum();
    let mut types = Vec::with_capacity(count);
    let mut offsets = Vec::with_capacity(count + 1);
    let mut connectivity = Vec::with_capacity(connectivity_count);
    offsets.push(0);
    for (cell, &keep) in keep.iter().enumerate() {
        if !keep {
            continue;
        }
        kept.push(cell);
        types.push(dataset.cells.types[cell]);
        for &node in dataset.cells.cell(cell).unwrap() {
            let source = node as usize;
            if remap[source] == usize::MAX {
                crate::geometry::output_base(order.len(), 1)?;
                remap[source] = order.len();
                order.push(source);
            }
            connectivity.push(u32::try_from(remap[source]).expect("checked output index"));
        }
        offsets.push(connectivity.len() as u64);
    }
    let positions = order
        .iter()
        .map(|&index| dataset.positions[index])
        .collect::<Vec<_>>();
    let mut result = DataSet::new(
        dataset.id.clone(),
        positions,
        Cells {
            types: types.into(),
            offsets: offsets.into(),
            connectivity: connectivity.into(),
        },
    );
    result.point_data = dataset
        .point_data
        .iter()
        .map(|field| pick(field, &order))
        .collect();
    result.cell_data = dataset
        .cell_data
        .iter()
        .map(|field| pick(field, &kept))
        .collect();
    Ok(result)
}

/// Selects field entries without changing support; identity selections share data.
pub(crate) fn pick(field: &Field, order: &[usize]) -> Field {
    if order.len() == field.data.len() && order.iter().enumerate().all(|(i, &index)| i == index) {
        return field.clone();
    }
    let data = match &field.data {
        FieldData::Scalar(v) => {
            FieldData::Scalar(order.iter().map(|&i| v[i]).collect::<Vec<_>>().into())
        }
        FieldData::Vector(v) => {
            FieldData::Vector(order.iter().map(|&i| v[i]).collect::<Vec<_>>().into())
        }
        FieldData::Tensor(v) => {
            FieldData::Tensor(order.iter().map(|&i| v[i]).collect::<Vec<_>>().into())
        }
        FieldData::Complex(v) => {
            FieldData::Complex(order.iter().map(|&i| v[i]).collect::<Vec<_>>().into())
        }
    };
    Field {
        name: field.name.clone(),
        label: field.label.clone(),
        unit: field.unit.clone(),
        association: field.association,
        data,
        valid: field
            .valid
            .as_ref()
            .map(|mask| order.iter().map(|&i| mask[i]).collect::<Vec<_>>().into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Field;

    #[test]
    fn thresholds_cells_by_point_field() {
        let ds = DataSet::surface(
            "s",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [2.0, 0.0, 0.0]],
            &[[0, 1, 2], [1, 3, 2]],
        )
        .with_field(Field::scalar("p", vec![0.0, 1.0, 2.0, 10.0]));
        let out = threshold(&ds, "p", Association::Point, 0.0, 2.0, ThresholdMode::All).unwrap();
        assert_eq!(out.cell_count(), 1);
        assert_eq!(out.validate(), Ok(()));
    }

    #[test]
    fn invalid_placeholders_never_match_either_threshold_mode() {
        for association in [Association::Point, Association::Cell] {
            let count = if association == Association::Point {
                3
            } else {
                1
            };
            let mut field = Field::scalar("p", vec![0.5; count]);
            field.association = association;
            field.valid = Some(vec![false; count].into());
            let ds = DataSet::surface(
                "masked",
                vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                &[[0, 1, 2]],
            )
            .with_field(field);
            for mode in [ThresholdMode::All, ThresholdMode::Any] {
                assert_eq!(
                    threshold(&ds, "p", association, 0.0, 1.0, mode)
                        .unwrap()
                        .cell_count(),
                    0
                );
            }
        }
    }

    #[test]
    fn any_requires_one_valid_value_and_preserves_subset_masks() {
        let mut field = Field::scalar("p", vec![0.5; 3]);
        field.valid = Some(vec![false, true, false].into());
        let ds = DataSet::surface(
            "partial",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        )
        .with_field(field);
        assert_eq!(
            threshold(&ds, "p", Association::Point, 0.0, 1.0, ThresholdMode::All)
                .unwrap()
                .cell_count(),
            0
        );
        let selected =
            threshold(&ds, "p", Association::Point, 0.0, 1.0, ThresholdMode::Any).unwrap();
        assert_eq!(selected.cell_count(), 1);
        assert_eq!(
            selected.point_data[0].valid.as_deref(),
            Some([false, true, false].as_slice())
        );
    }
}
