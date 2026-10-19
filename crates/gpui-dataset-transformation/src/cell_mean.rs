//! Cell-to-point averages with exact binary-weight reduction and compact incidence.
//!
//! Common weights stay inline, with an optional missing-cell bitmap. Unequal
//! weights and incidence storage are materialized only for the general path.
//! Exact accumulators are reused sequentially on the stack, never per point.

// Rust guideline compliant 2026-02-21

use crate::mean::{FieldSum128, SmallFieldSum, certified_float_rows, exact_average};
use crate::weighted_mean::{ProductPlan, exact_weighted_average};
use gpui_mesh::{DataSet, Field};
use std::ops::AddAssign;

pub(crate) struct ComputedRows<const N: usize> {
    pub(crate) values: Vec<[f64; N]>,
    pub(crate) all_supported: bool,
}

struct CellWeights {
    common: f64,
    dense: Option<Vec<f64>>,
    zeros: Option<Vec<u64>>,
    len: usize,
}

impl CellWeights {
    fn new(len: usize) -> Self {
        Self {
            common: 0.0,
            dense: None,
            zeros: None,
            len,
        }
    }

    fn prepare(dataset: &DataSet, field: &Field) -> Self {
        let mut weights = Self::new(dataset.cell_count());
        for cell in 0..dataset.cell_count() {
            let weight = if field.valid.as_ref().is_some_and(|mask| !mask[cell]) {
                0.0
            } else {
                crate::convert::cell_measure(dataset, cell)
            };
            weights.record(
                cell,
                if weight.is_finite() && weight > 0.0 {
                    weight
                } else {
                    0.0
                },
            );
        }
        weights
    }

    fn record(&mut self, cell: usize, weight: f64) {
        if let Some(dense) = &mut self.dense {
            dense[cell] = weight;
            return;
        }
        if weight == 0.0 {
            let zeros = self
                .zeros
                .get_or_insert_with(|| vec![0; self.len.div_ceil(64)]);
            zeros[cell / 64] |= 1_u64 << (cell % 64);
        } else if self.common == 0.0 {
            self.common = weight;
        } else if weight != self.common {
            let mut dense = vec![self.common; self.len];
            if let Some(zeros) = self.zeros.take() {
                for (cell, value) in dense.iter_mut().enumerate() {
                    if zeros[cell / 64] & (1_u64 << (cell % 64)) != 0 {
                        *value = 0.0;
                    }
                }
            }
            dense[cell] = weight;
            self.dense = Some(dense);
        }
    }

    #[inline]
    fn get(&self, cell: usize) -> f64 {
        if let Some(dense) = &self.dense {
            dense[cell]
        } else if self
            .zeros
            .as_ref()
            .is_some_and(|zeros| zeros[cell / 64] & (1_u64 << (cell % 64)) != 0)
        {
            0.0
        } else {
            self.common
        }
    }
}

struct Contributors {
    weights: CellWeights,
    counts: Vec<usize>,
    dimensions: Option<Vec<u8>>,
}

impl Contributors {
    fn new(dataset: &DataSet, weights: CellWeights, uniform: bool) -> Self {
        // The spare first slot lets counts become CSR offsets in place.
        let mut counts = vec![0; dataset.point_count() + 1];
        let mut dimensions = (!uniform).then(|| vec![0; dataset.point_count()]);
        for cell in 0..dataset.cell_count() {
            let weight = weights.get(cell);
            if weight == 0.0 {
                continue;
            }
            let dimension = dataset.cells.types[cell].dimension();
            for &node in dataset.cells.cell(cell).unwrap() {
                let node = node as usize;
                if let Some(dimensions) = &mut dimensions {
                    if counts[node + 1] != 0 && dimension < dimensions[node] {
                        continue;
                    }
                    if dimension > dimensions[node] {
                        counts[node + 1] = 0;
                    }
                    dimensions[node] = dimension;
                }
                counts[node + 1] += 1;
            }
        }
        Self {
            weights,
            counts,
            dimensions,
        }
    }

    fn includes(&self, dataset: &DataSet, point: usize, cell: usize) -> bool {
        self.dimensions
            .as_ref()
            .is_none_or(|dimensions| dimensions[point] == dataset.cells.types[cell].dimension())
    }
}

pub(crate) fn cell_means<const N: usize>(
    dataset: &DataSet,
    field: &Field,
    values: &[[f64; N]],
    uniform: bool,
) -> ComputedRows<N> {
    if values.is_empty()
        || field
            .valid
            .as_ref()
            .is_some_and(|mask| mask.iter().all(|v| !*v))
    {
        return ComputedRows {
            values: vec![[f64::NAN; N]; dataset.point_count()],
            all_supported: dataset.point_count() == 0,
        };
    }
    let weights = CellWeights::prepare(dataset, field);
    // Connectivity bounds every point's incidence count, including repeated
    // node indices. An inexpensive global proof avoids building point counts.
    let equal = weights.dense.is_none();
    let supported = || {
        values.iter().enumerate().filter_map(|(cell, value)| {
            let weight = weights.get(cell);
            (weight != 0.0).then_some((value, weight))
        })
    };
    let maximum = dataset.cells.connectivity.len();
    let floating = if equal {
        certified_float_rows(supported().map(|(row, _)| row), maximum)
    } else {
        ProductPlan::certify::<N, 53>(supported(), maximum).is_some()
    };
    if floating {
        return floating_rows(dataset, values, weights, uniform, equal);
    }
    let contributors = Contributors::new(dataset, weights, uniform);
    let maximum = contributors.counts.iter().copied().max().unwrap_or(0);
    let all_supported = contributors.counts[1..].iter().all(|&count| count != 0);
    if maximum == 0 {
        return ComputedRows {
            values: vec![[f64::NAN; N]; dataset.point_count()],
            all_supported,
        };
    }
    if contributors.weights.dense.is_none() {
        let supported = || {
            values
                .iter()
                .enumerate()
                .filter(|(cell, _)| contributors.weights.get(*cell) != 0.0)
                .map(|(_, value)| value)
        };
        if certified_float_rows(supported(), maximum) {
            let mut sums = scatter(dataset, values, &contributors, |value| *value);
            for (sum, &count) in sums.iter_mut().zip(&contributors.counts[1..]) {
                *sum = if count == 0 {
                    [f64::NAN; N]
                } else {
                    sum.map(|value| value / count as f64)
                };
            }
            return ComputedRows {
                values: sums,
                all_supported,
            };
        }
        if let Some(plan) = SmallFieldSum::certify(supported(), maximum) {
            let sums = if let Some(scale) = plan.integer_scale() {
                scatter(dataset, values, &contributors, |value| {
                    value.map(|v| (v * scale) as i64)
                })
            } else {
                scatter(dataset, values, &contributors, |value| {
                    value.map(|v| plan.integer(v))
                })
            };
            let values = sums
                .into_iter()
                .zip(contributors.counts.into_iter().skip(1))
                .map(|(sum, count)| {
                    if count == 0 {
                        [f64::NAN; N]
                    } else {
                        plan.averages(sum, count)
                    }
                })
                .collect();
            return ComputedRows {
                values,
                all_supported,
            };
        }
        if let Some(plan) = FieldSum128::certify(supported(), maximum) {
            let sums = scatter(dataset, values, &contributors, |value| {
                value.map(|v| plan.integer(v))
            });
            let values = sums
                .into_iter()
                .zip(contributors.counts.into_iter().skip(1))
                .map(|(sum, count)| {
                    if count == 0 {
                        [f64::NAN; N]
                    } else {
                        plan.averages(sum, count)
                    }
                })
                .collect();
            return ComputedRows {
                values,
                all_supported,
            };
        }
        if dataset.point_count() == 1 {
            // With one point, positive geometric measures can only come from
            // unit vertex cells. Each retained cell contributes exactly once.
            let values = vec![std::array::from_fn(|axis| {
                exact_average(supported().map(|value| value[axis]))
            })];
            return ComputedRows {
                values,
                all_supported,
            };
        }
    }
    let supported = || {
        values.iter().enumerate().filter_map(|(cell, value)| {
            let weight = contributors.weights.get(cell);
            (weight != 0.0).then_some((value, weight))
        })
    };
    if let Some(plan) = ProductPlan::certify::<N, 64>(supported(), maximum) {
        let values = weighted_rows(
            dataset,
            values,
            contributors,
            |value, weight| plan.terms64(value, weight),
            |numerator, denominator| plan.averages64(numerator, denominator),
        );
        return ComputedRows {
            values,
            all_supported,
        };
    }
    if let Some(plan) = ProductPlan::certify::<N, 128>(supported(), maximum) {
        let values = weighted_rows(
            dataset,
            values,
            contributors,
            |value, weight| plan.terms(value, weight),
            |numerator, denominator| plan.averages(numerator, denominator),
        );
        return ComputedRows {
            values,
            all_supported,
        };
    }
    // Most meshes use compact cell indices. Preserve correctness for larger
    // sources with usize indices rather than silently truncating their IDs.
    let values = if u32::try_from(dataset.cell_count()).is_ok() {
        indexed_rows(
            dataset,
            values,
            contributors,
            |index| index as u32,
            |index| index as usize,
        )
    } else {
        indexed_rows(dataset, values, contributors, |index| index, |index| index)
    };
    ComputedRows {
        values,
        all_supported,
    }
}

fn floating_rows<const N: usize>(
    dataset: &DataSet,
    values: &[[f64; N]],
    cell_weights: CellWeights,
    uniform: bool,
    equal: bool,
) -> ComputedRows<N> {
    let mut sums = vec![[0.0; N]; dataset.point_count()];
    let mut weights = vec![0.0; dataset.point_count()];
    let mut dimensions = (!uniform).then(|| vec![0; dataset.point_count()]);
    for (cell, value) in values.iter().enumerate() {
        let weight = cell_weights.get(cell);
        if weight == 0.0 {
            continue;
        }
        let weight = if equal { 1.0 } else { weight };
        let weighted = if equal {
            *value
        } else {
            value.map(|v| v * weight)
        };
        let dimension = dataset.cells.types[cell].dimension();
        for &node in dataset.cells.cell(cell).unwrap() {
            let point = node as usize;
            if let Some(dimensions) = &mut dimensions {
                if weights[point] != 0.0 && dimension < dimensions[point] {
                    continue;
                }
                if dimension > dimensions[point] {
                    weights[point] = 0.0;
                    sums[point] = [0.0; N];
                }
                dimensions[point] = dimension;
            }
            weights[point] += weight;
            for (sum, value) in sums[point].iter_mut().zip(weighted) {
                *sum += value;
            }
        }
    }
    let mut all_supported = true;
    for (sum, weight) in sums.iter_mut().zip(weights) {
        if weight == 0.0 {
            *sum = [f64::NAN; N];
            all_supported = false;
            continue;
        }
        let fraction = weight.to_bits() & ((1_u64 << 52) - 1);
        let exponent = (weight.to_bits() >> 52) & 0x7ff;
        let power_of_two = if exponent == 0 {
            fraction.is_power_of_two()
        } else {
            fraction == 0
        };
        if N > 1 && power_of_two {
            let inverse = 1.0 / weight;
            *sum = if inverse.is_finite() {
                sum.map(|v| v * inverse)
            } else {
                sum.map(|v| v / weight)
            };
        } else {
            *sum = sum.map(|v| v / weight);
        }
    }
    ComputedRows {
        values: sums,
        all_supported,
    }
}

fn scatter<const N: usize, T: Copy + Default + AddAssign>(
    dataset: &DataSet,
    values: &[[f64; N]],
    contributors: &Contributors,
    decode: impl Fn(&[f64; N]) -> [T; N],
) -> Vec<[T; N]> {
    let mut sums = vec![[T::default(); N]; dataset.point_count()];
    for (cell, value) in values.iter().enumerate() {
        if contributors.weights.get(cell) == 0.0 {
            continue;
        }
        let value = decode(value);
        for &node in dataset.cells.cell(cell).unwrap() {
            let point = node as usize;
            if !contributors.includes(dataset, point, cell) {
                continue;
            }
            for (sum, value) in sums[point].iter_mut().zip(value) {
                *sum += value;
            }
        }
    }
    sums
}

fn weighted_rows<const N: usize, T: Copy + Default + AddAssign, W: Copy + Default + AddAssign>(
    dataset: &DataSet,
    values: &[[f64; N]],
    mut contributors: Contributors,
    decode: impl Fn(&[f64; N], f64) -> ([T; N], W),
    average: impl Fn([T; N], W) -> [f64; N],
) -> Vec<[f64; N]> {
    drop(std::mem::take(&mut contributors.counts));
    let mut sums = vec![[T::default(); N]; dataset.point_count()];
    let mut weights = vec![W::default(); dataset.point_count()];
    for (cell, value) in values.iter().enumerate() {
        let weight = contributors.weights.get(cell);
        if weight == 0.0 {
            continue;
        }
        let (numerator, denominator) = decode(value, weight);
        for &node in dataset.cells.cell(cell).unwrap() {
            let point = node as usize;
            if !contributors.includes(dataset, point, cell) {
                continue;
            }
            weights[point] += denominator;
            for (sum, value) in sums[point].iter_mut().zip(numerator) {
                *sum += value;
            }
        }
    }
    sums.into_iter()
        .zip(weights)
        .map(|(sum, weight)| average(sum, weight))
        .collect()
}

fn indexed_rows<const N: usize, I: Copy>(
    dataset: &DataSet,
    values: &[[f64; N]],
    contributors: Contributors,
    encode: impl Fn(usize) -> I,
    decode: impl Fn(I) -> usize,
) -> Vec<[f64; N]> {
    let Contributors {
        weights,
        counts: mut offsets,
        dimensions,
        ..
    } = contributors;
    for index in 1..offsets.len() {
        offsets[index] += offsets[index - 1];
    }
    // Counts sum to at most the validated connectivity length.
    let total = offsets[dataset.point_count()];
    let mut cells = vec![encode(0); total];
    for cell in (0..dataset.cell_count()).rev() {
        if weights.get(cell) == 0.0 {
            continue;
        }
        for &node in dataset.cells.cell(cell).unwrap() {
            let point = node as usize;
            if dimensions.as_ref().is_some_and(|dimensions| {
                dimensions[point] != dataset.cells.types[cell].dimension()
            }) {
                continue;
            }
            offsets[point + 1] -= 1;
            cells[offsets[point + 1]] = encode(cell);
        }
    }
    offsets.copy_within(1.., 0);
    offsets[dataset.point_count()] = total;
    drop(dimensions);
    let equal_weights = weights.dense.is_none();
    (0..dataset.point_count())
        .map(|point| {
            let incident = &cells[offsets[point]..offsets[point + 1]];
            match incident {
                [] => [f64::NAN; N],
                [cell] => values[decode(*cell)].map(|value| if value == 0.0 { 0.0 } else { value }),
                _ => std::array::from_fn(|axis| {
                    if equal_weights {
                        exact_average(incident.iter().map(|&cell| values[decode(cell)][axis]))
                    } else {
                        exact_weighted_average(incident.iter().map(|&cell| {
                            let cell = decode(cell);
                            (values[cell][axis], weights.get(cell))
                        }))
                    }
                }),
            }
        })
        .collect()
}
