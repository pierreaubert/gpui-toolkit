//! Validity-aware expression evaluation, scalar summaries and grid resampling.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::expression::Expression;
use crate::mean::AdaptiveMean;
use gpui_mesh::{Association, DataSet, Field};

/// Computes a scalar expression with strict dependency validity.
///
/// Fields bind once before evaluation; one reusable stack serves every point.
/// Missing inputs or nonfinite intermediate results produce masked NaNs, including
/// inside min/max. Coordinates are `x`, `y`, `z`; arithmetic supports `+ - * / ^`
/// and sin/cos/tan/sqrt/exp/log/abs/min/max. Powers associate right and precede
/// unary signs. The output replaces an existing same-name point field.
///
/// # Errors
///
/// Returns an error for invalid datasets, syntax, unknown/non-scalar fields,
/// unknown functions/arity, nonfinite constants, or expressions exceeding 64 KiB,
/// 128 nested levels or 4096 instructions. Binding errors also apply to empty data.
pub fn calculator(
    dataset: &DataSet,
    expression: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    let mut expression = Expression::new(dataset, expression)?;
    let values = dataset
        .positions
        .iter()
        .enumerate()
        .map(|(index, &point)| expression.evaluate(index, point))
        .collect::<Vec<_>>();
    let field = crate::validity::finish(Field::scalar(out_name.to_string(), values), |_| true);
    let mut out = dataset.clone();
    out.point_data.retain(|f| f.name.as_ref() != out_name);
    out.point_data.push(field);
    Ok(out)
}

/// Summary statistics of supported scalar point or cell values.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldStats {
    /// Number of valid finite values.
    pub count: usize,
    /// Smallest valid value.
    pub min: f64,
    /// Largest valid value.
    pub max: f64,
    /// Arithmetic mean.
    pub mean: f64,
    /// Population standard deviation.
    pub std_dev: f64,
    /// Median; even counts average the two middle values.
    pub median: f64,
}

fn scalar_field<'a>(
    dataset: &'a DataSet,
    name: &str,
    association: Association,
) -> Result<(&'a Field, &'a [f64]), TransformError> {
    dataset.validate()?;
    let field = dataset
        .field(association, name)
        .ok_or_else(|| TransformError::FieldNotFound(name.into()))?;
    let values = field
        .as_scalar()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: name.into(),
            reason: "analysis requires a scalar field".into(),
        })?;
    Ok((field, values))
}

fn supported_values<'a>(field: &'a Field, values: &'a [f64]) -> impl Iterator<Item = f64> + 'a {
    // Validation establishes finiteness and matching mask lengths once.
    values.iter().enumerate().filter_map(|(i, &value)| {
        field
            .valid
            .as_ref()
            .is_none_or(|mask| mask[i])
            .then_some(value)
    })
}

#[derive(Default)]
struct Sum {
    value: f64,
    correction: f64,
}

impl Sum {
    fn add(&mut self, value: f64) {
        let next = self.value + value;
        self.correction += if self.value.abs() >= value.abs() {
            (self.value - next) + value
        } else {
            (value - next) + self.value
        };
        self.value = next;
    }
    fn total(&self) -> f64 {
        self.value + self.correction
    }
}

/// Computes validity-aware scalar summaries with linear-time median selection.
///
/// Uses one value-proportional scratch buffer. Centered/scaled compensated sums
/// avoid overflow and preserve small spreads around large means. Missing entries
/// are excluded; a population with no supported values returns an error.
/// Mixed-sign means use exact integer accumulation so cancellation cannot erase
/// tiny residuals; centered variance retains its own precision independently.
///
/// # Errors
///
/// Returns an error for invalid datasets, missing/non-scalar fields or no valid values.
pub fn field_statistics(
    dataset: &DataSet,
    field_name: &str,
    association: Association,
) -> Result<FieldStats, TransformError> {
    let (field, values) = scalar_field(dataset, field_name, association)?;
    let count = field.valid.as_ref().map_or(values.len(), |mask| {
        mask.iter().filter(|&&valid| valid).count()
    });
    if count == 0 {
        return Err(TransformError::Degenerate(
            "field has no valid values".into(),
        ));
    }
    let mut finite = Vec::with_capacity(count);
    finite.extend(supported_values(field, values));
    let min = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let max = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let center = f64::midpoint(min, max);
    let scale = (min - center).abs().max((max - center).abs());
    let (mean, std_dev) = if scale == 0.0 {
        (center, 0.0)
    } else {
        let mut sum = Sum::default();
        let exact_mean = if min < 0.0 && max > 0.0 {
            // Keep the centered mean for variance (even when its rounded raw
            // value cannot represent a small spread). Compute cancellation-prone
            // raw means separately, preserving residuals across the full range.
            let mut mean = AdaptiveMean::default();
            for &value in &finite {
                sum.add((value - center) / scale);
                mean.add(value);
            }
            Some(mean.finish())
        } else {
            for &value in &finite {
                sum.add((value - center) / scale);
            }
            None
        };
        let normalized_mean = (sum.total() / count as f64).clamp(-1.0, 1.0);
        let mut squares = Sum::default();
        for &value in &finite {
            squares.add(((value - center) / scale - normalized_mean).powi(2));
        }
        (
            exact_mean.unwrap_or_else(|| ((normalized_mean * scale) + center).clamp(min, max)),
            (squares.total() / count as f64).sqrt().min(1.0) * scale,
        )
    };
    let middle = count / 2;
    let (lower, median, _) = finite.select_nth_unstable_by(middle, f64::total_cmp);
    let median = if count.is_multiple_of(2) {
        f64::midpoint(
            lower.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            *median,
        )
    } else {
        *median
    };
    Ok(FieldStats {
        count,
        min,
        max,
        mean,
        std_dev,
        median,
    })
}

/// Counts valid point scalars into equal-width bins without sorting values.
///
/// Returns `(center,count)` pairs. The maximum belongs to the final bin; a
/// constant field belongs to the first bin. Centers remain finite for extreme
/// ranges and subnormal widths. At most 1,048,576 bins are accepted, bounding
/// count/result buffers to at most 24 MiB together on supported targets.
///
/// # Errors
///
/// Returns an error for invalid datasets, missing/non-scalar/wholly missing
/// fields, or a bin count outside `1..=1_048_576`.
pub fn histogram(
    dataset: &DataSet,
    field_name: &str,
    bins: usize,
) -> Result<Vec<(f64, usize)>, TransformError> {
    if bins == 0 || bins > 1_048_576 {
        return Err(TransformError::Degenerate(
            "histogram needs 1..=1048576 bins".into(),
        ));
    }
    let (field, values) = scalar_field(dataset, field_name, Association::Point)?;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for value in supported_values(field, values) {
        min = min.min(value);
        max = max.max(value);
    }
    if min > max {
        return Err(TransformError::Degenerate(
            "field has no valid values".into(),
        ));
    }
    let span = max - min;
    let mut counts = vec![0usize; bins];
    for value in supported_values(field, values) {
        let fraction = if min == max {
            0.0
        } else if span.is_finite() {
            (value - min) / span
        } else {
            (value * 0.5 - min * 0.5) / (max * 0.5 - min * 0.5)
        };
        let index = ((fraction * bins as f64) as usize).min(bins - 1);
        counts[index] += 1;
    }
    Ok(counts
        .into_iter()
        .enumerate()
        .map(|(i, count)| {
            (
                crate::convert::blend(min, max, (i as f64 + 0.5) / bins as f64),
                count,
            )
        })
        .collect())
}

/// Resamples scalar point fields while preserving missing support and metadata.
///
/// Points without supported source samples receive `fill` with a false validity
/// bit. Finite placeholders and NaN are permitted fill values. The output keeps
/// each source field name, label and unit.
///
/// # Errors
///
/// Returns an error for invalid source/grid data or infinite fill values.
pub fn resample_to_grid(
    dataset: &DataSet,
    dims: [usize; 3],
    origin: [f64; 3],
    spacing: [f64; 3],
    fill: f64,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    if fill.is_infinite() {
        return Err(TransformError::Degenerate(
            "resampling fill must be finite or NaN".into(),
        ));
    }
    let mut grid =
        DataSet::structured_grid(format!("{}-resampled", dataset.id), dims, origin, spacing)?;
    for source in dataset
        .point_data
        .iter()
        .filter(|f| f.as_scalar().is_some())
    {
        let mut values = Vec::with_capacity(grid.point_count());
        let mut valid = None::<Vec<bool>>;
        for (index, sample) in crate::probe::samples(dataset, source, &grid.positions).enumerate() {
            if sample.is_none() {
                valid.get_or_insert_with(|| vec![true; grid.point_count()])[index] = false;
            }
            values.push(sample.unwrap_or(fill));
        }
        let mut field = Field::scalar(source.name.clone(), values);
        field.label.clone_from(&source.label);
        field.unit.clone_from(&source.unit);
        field.valid = valid.map(Into::into);
        grid.point_data.push(field);
    }
    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> DataSet {
        let ds = DataSet::structured_grid("g", [3, 3, 3], [0.0; 3], [1.0; 3]).unwrap();
        let values: Vec<f64> = ds.positions.iter().map(|p| p[0] + 10.0 * p[1]).collect();
        ds.with_field(Field::scalar("p", values))
    }

    #[test]
    fn calculator_evaluates_fields_and_coordinates() {
        let out = calculator(&grid(), "p * 2 + x - sqrt(y + 1)", "q").unwrap();
        let values = out
            .field(Association::Point, "q")
            .unwrap()
            .as_scalar()
            .unwrap();
        // Point (1, 2, 0): p = 21, q = 42 + 1 - sqrt(3).
        let index = out
            .positions
            .iter()
            .position(|p| *p == [1.0, 2.0, 0.0])
            .unwrap();
        assert!((values[index] - (43.0 - 3.0f64.sqrt())).abs() < 1e-12);
        calculator(&grid(), "nope + 1", "q").unwrap_err();
        calculator(&grid(), "p +", "q").unwrap_err();
    }

    #[test]
    fn statistics_and_histogram_are_exact() {
        let stats = field_statistics(&grid(), "p", Association::Point).unwrap();
        assert_eq!(stats.count, 27);
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 22.0);
        let bins = histogram(&grid(), "p", 2).unwrap();
        assert_eq!(bins.iter().map(|(_, c)| c).sum::<usize>(), 27);
    }

    #[test]
    fn resample_recovers_linear_field() {
        let out = resample_to_grid(&grid(), [5, 5, 5], [0.0; 3], [0.5; 3], -1.0).unwrap();
        let values = out
            .field(Association::Point, "p")
            .unwrap()
            .as_scalar()
            .unwrap();
        for (p, v) in out.positions.iter().zip(values.iter()) {
            assert!((v - (p[0] + 10.0 * p[1])).abs() < 1e-9, "at {p:?}: {v}");
        }
    }
}
