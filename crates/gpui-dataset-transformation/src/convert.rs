//! Validity-aware point/cell conversion with bounded per-point scratch.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, CellType, DataSet, Field, FieldData, ValidatedDataSet};

pub(crate) fn blend(a: f64, b: f64, fraction: f64) -> f64 {
    if fraction == 1.0 || a == b {
        return b;
    }
    if fraction == 0.0 {
        return a;
    }
    let result = a * (1.0 - fraction) + b * fraction;
    if result.is_normal() {
        return result;
    }
    // Finite convex inputs must remain bounded. Scaling also retains
    // subnormal results when the individual weighted products underflow.
    let scale = a.abs().max(b.abs());
    ((a / scale) * (1.0 - fraction) + (b / scale) * fraction).clamp(-1.0, 1.0) * scale
}

fn point_means<const N: usize>(
    dataset: &DataSet,
    field: &Field,
    values: &[[f64; N]],
) -> Vec<[f64; N]> {
    if dataset.cells.is_empty() {
        return Vec::new();
    }
    let all_missing = field
        .valid
        .as_ref()
        .is_some_and(|mask| mask.iter().all(|v| !*v));
    if all_missing {
        return vec![[f64::NAN; N]; dataset.cell_count()];
    }
    // A dense mesh reuses a field sign certificate; sparse selections inspect
    // referenced values only. Within mixed fields, track actual cell signs so
    // that a distant zero crossing cannot force every cell into exact arithmetic.
    let mixed = (dataset.cells.connectivity.len() >= values.len()).then(|| {
        crate::mean::has_mixed_rows(
            values
                .iter()
                .enumerate()
                .filter(|(i, _)| field.valid.as_ref().is_none_or(|mask| mask[*i]))
                .map(|(_, value)| value),
        )
    });
    // Quantized data (for example f32 promoted to f64) often has an exact
    // floating sum even with both signs. Certify the bounded fixed cell sizes
    // once rather than paying for exact accumulation in every crossing cell.
    let exact_float = mixed == Some(true)
        && !dataset.cells.types.contains(&CellType::Polygon)
        && exact_field_sums(values, field.valid.as_deref(), 8);
    let small_sum =
        (mixed == Some(true) && !exact_float && !dataset.cells.types.contains(&CellType::Polygon))
            .then(|| {
                crate::mean::SmallFieldSum::certify(
                    values
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| field.valid.as_ref().is_none_or(|mask| mask[*i]))
                        .map(|(_, row)| row),
                    8,
                )
            })
            .flatten();
    // Dispatch once so each collection loop has one arithmetic strategy.
    if let Some(plan) = small_sum {
        if let Some(scale) = plan.integer_scale() {
            collect_point_means(dataset, |nodes| {
                integer_point_mean(nodes, values, field.valid.as_deref(), plan, |value| {
                    (value * scale) as i64
                })
            })
        } else {
            collect_point_means(dataset, |nodes| {
                integer_point_mean(nodes, values, field.valid.as_deref(), plan, |value| {
                    plan.integer(value)
                })
            })
        }
    } else if mixed == Some(false) || exact_float {
        // A certified sum needs direct division for arbitrary contributor
        // counts. With complete power-of-two cells, reciprocal multiplication
        // is exact and the loop can omit this per-cell arithmetic decision.
        let direct = exact_float
            && (field.valid.is_some()
                || dataset
                    .cells
                    .types
                    .iter()
                    .any(|kind| !kind.node_count().is_power_of_two()));
        if direct {
            collect_point_means(dataset, |nodes| {
                point_mean::<N, false, true>(nodes, values, field.valid.as_deref())
            })
        } else {
            collect_point_means(dataset, |nodes| {
                point_mean::<N, false, false>(nodes, values, field.valid.as_deref())
            })
        }
    } else {
        collect_point_means(dataset, |nodes| {
            point_mean::<N, true, false>(nodes, values, field.valid.as_deref())
        })
    }
}

fn collect_point_means<const N: usize>(
    dataset: &DataSet,
    average: impl Fn(&[u32]) -> [f64; N],
) -> Vec<[f64; N]> {
    (0..dataset.cell_count())
        .map(|cell| average(dataset.cells.cell(cell).unwrap()))
        .collect()
}

#[inline]
fn integer_point_mean<const N: usize>(
    nodes: &[u32],
    values: &[[f64; N]],
    valid: Option<&[bool]>,
    plan: crate::mean::SmallFieldSum,
    decode: impl Fn(f64) -> i64,
) -> [f64; N] {
    let mut sums = [0; N];
    let mut include = |node: u32| {
        for (sum, &value) in sums.iter_mut().zip(&values[node as usize]) {
            *sum += decode(value);
        }
    };
    let count = if let Some(mask) = valid {
        let mut count = 0;
        for &node in nodes {
            if mask[node as usize] {
                include(node);
                count += 1;
            }
        }
        count
    } else {
        for &node in nodes {
            include(node);
        }
        nodes.len()
    };
    if count == 0 {
        [f64::NAN; N]
    } else {
        plan.averages(sums, count)
    }
}

#[inline]
fn point_mean<const N: usize, const TRACK_SIGNS: bool, const DIRECT_DIVISION: bool>(
    nodes: &[u32],
    values: &[[f64; N]],
    valid: Option<&[bool]>,
) -> [f64; N] {
    let (sum, count, signs) = sum_nodes::<N, TRACK_SIGNS>(nodes, values, valid);
    if count == 0 {
        return [f64::NAN; N];
    }
    let inverse = 1.0 / count as f64;
    if !crate::mean::has_mixed_signs(signs) && sum.iter().all(|v| v.is_finite()) {
        if N == 1 || DIRECT_DIVISION {
            sum.map(|v| v / count as f64)
        } else {
            sum.map(|v| v * inverse)
        }
    } else {
        std::array::from_fn(|axis| {
            if ((signs >> (2 * axis)) & 3) != 3 && sum[axis].is_finite() {
                if N == 1 {
                    sum[axis] / count as f64
                } else {
                    sum[axis] * inverse
                }
            } else {
                let contributors = nodes
                    .iter()
                    .filter(|&&node| valid.is_none_or(|mask| mask[node as usize]))
                    .map(|&node| values[node as usize][axis]);
                crate::mean::exact_average(contributors)
            }
        })
    }
}

fn exact_field_sums<const N: usize>(
    values: &[[f64; N]],
    valid: Option<&[bool]>,
    max_nodes: usize,
) -> bool {
    crate::mean::certified_float_rows(
        values
            .iter()
            .enumerate()
            .filter(|(index, _)| valid.is_none_or(|mask| mask[*index]))
            .map(|(_, row)| row),
        max_nodes,
    )
}

#[inline]
fn sum_nodes<const N: usize, const TRACK_SIGNS: bool>(
    nodes: &[u32],
    values: &[[f64; N]],
    valid: Option<&[bool]>,
) -> ([f64; N], usize, u16) {
    let mut sum = [0.0; N];
    let mut count = 0;
    let mut signs = 0;
    let mut include = |node: u32| {
        count += 1;
        for axis in 0..N {
            let value = values[node as usize][axis];
            sum[axis] += value;
            if TRACK_SIGNS {
                signs |= crate::mean::component_sign(value, axis);
            }
        }
    };
    if let Some(mask) = valid {
        for &node in nodes {
            if mask[node as usize] {
                include(node);
            }
        }
    } else {
        for &node in nodes {
            include(node);
        }
    }
    (sum, count, signs)
}

#[derive(Clone, Copy, Default)]
struct WeightSum {
    scale: f64,
    sum: f64,
    dimension: u8,
}

impl WeightSum {
    fn fraction(&mut self, weight: f64, dimension: u8) -> Option<f64> {
        if self.sum > 0.0 && dimension < self.dimension {
            return None;
        }
        if self.sum == 0.0 || dimension > self.dimension {
            *self = Self {
                scale: weight,
                sum: 1.0,
                dimension,
            };
            return Some(1.0);
        }
        if weight > self.scale {
            self.sum = self.sum * (self.scale / weight) + 1.0;
            self.scale = weight;
            Some(1.0 / self.sum)
        } else {
            let scaled = weight / self.scale;
            self.sum += scaled;
            Some(scaled / self.sum)
        }
    }
}

#[derive(Clone, Copy, Default)]
struct AccumulatedWeight {
    sum: f64,
    dimension: u8,
}

enum WeightUpdate {
    Add,
    Replace,
    Skip,
}

trait WeightAccumulator: Copy + Default {
    fn include(&mut self, weight: f64, dimension: u8) -> WeightUpdate;
    fn sum(self) -> f64;
}

impl WeightAccumulator for f64 {
    fn include(&mut self, weight: f64, _: u8) -> WeightUpdate {
        *self += weight;
        WeightUpdate::Add
    }
    fn sum(self) -> f64 {
        self
    }
}

impl WeightAccumulator for AccumulatedWeight {
    fn include(&mut self, weight: f64, dimension: u8) -> WeightUpdate {
        if self.sum > 0.0 && dimension < self.dimension {
            WeightUpdate::Skip
        } else if self.sum == 0.0 || dimension > self.dimension {
            *self = Self {
                sum: weight,
                dimension,
            };
            WeightUpdate::Replace
        } else {
            self.sum += weight;
            WeightUpdate::Add
        }
    }
    fn sum(self) -> f64 {
        self.sum
    }
}

struct ComputedRows<const N: usize> {
    values: Vec<[f64; N]>,
    all_supported: bool,
}

fn cell_means<const N: usize>(
    dataset: &DataSet,
    field: &Field,
    values: &[[f64; N]],
    uniform: bool,
) -> ComputedRows<N> {
    if uniform {
        cell_means_fast::<N, f64>(dataset, field, values)
    } else {
        cell_means_fast::<N, AccumulatedWeight>(dataset, field, values)
    }
}

fn cell_means_fast<const N: usize, W: WeightAccumulator>(
    dataset: &DataSet,
    field: &Field,
    values: &[[f64; N]],
) -> ComputedRows<N> {
    if values.is_empty()
        || field
            .valid
            .as_ref()
            .is_some_and(|mask| mask.iter().all(|&valid| !valid))
    {
        return ComputedRows {
            values: vec![[f64::NAN; N]; dataset.point_count()],
            all_supported: dataset.point_count() == 0,
        };
    }
    let mut sums = vec![[0.0; N]; dataset.point_count()];
    let mut weights = vec![W::default(); dataset.point_count()];
    for (cell, value) in values.iter().enumerate() {
        if field.valid.as_ref().is_some_and(|mask| !mask[cell]) {
            continue;
        }
        let weight = cell_measure(dataset, cell);
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let weighted = value.map(|component| component * weight);
        if value
            .iter()
            .zip(&weighted)
            .any(|(&value, product)| value != 0.0 && !product.is_normal())
        {
            drop(sums);
            drop(weights);
            return cell_means_robust(dataset, field, values);
        }
        let dimension = dataset.cells.types[cell].dimension();
        for &node in dataset.cells.cell(cell).unwrap_or(&[]) {
            let state = &mut weights[node as usize];
            match state.include(weight, dimension) {
                WeightUpdate::Skip => {}
                WeightUpdate::Replace => sums[node as usize] = weighted,
                WeightUpdate::Add => {
                    for (sum, value) in sums[node as usize].iter_mut().zip(weighted) {
                        *sum += value;
                    }
                }
            }
        }
    }
    let mut needs_fallback = false;
    let mut all_supported = true;
    for (sum, weight) in sums.iter_mut().zip(&weights) {
        let weight = weight.sum();
        if weight == 0.0 {
            all_supported = false;
            *sum = [f64::NAN; N];
            continue;
        }
        if !weight.is_normal() || sum.iter().any(|v| !v.is_finite() || v.is_subnormal()) {
            needs_fallback = true;
            break;
        }
        let inverse = 1.0 / weight;
        for value in sum.iter_mut() {
            *value = if N > 1 && inverse.is_normal() {
                *value * inverse
            } else {
                *value / weight
            };
        }
        if sum.iter().any(|v| !v.is_finite()) {
            needs_fallback = true;
            break;
        }
    }
    if needs_fallback {
        drop(sums);
        drop(weights);
        cell_means_robust(dataset, field, values)
    } else {
        ComputedRows {
            values: sums,
            all_supported,
        }
    }
}

fn cell_means_robust<const N: usize>(
    dataset: &DataSet,
    field: &Field,
    values: &[[f64; N]],
) -> ComputedRows<N> {
    if values.is_empty()
        || field
            .valid
            .as_ref()
            .is_some_and(|mask| mask.iter().all(|&valid| !valid))
    {
        return ComputedRows {
            values: vec![[f64::NAN; N]; dataset.point_count()],
            all_supported: dataset.point_count() == 0,
        };
    }
    let mut mean = vec![[0.0; N]; dataset.point_count()];
    let mut weights = vec![WeightSum::default(); dataset.point_count()];
    for (cell, value) in values.iter().enumerate() {
        if field.valid.as_ref().is_some_and(|mask| !mask[cell]) {
            continue;
        }
        let weight = cell_measure(dataset, cell);
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let dimension = dataset.cells.types[cell].dimension();
        for &node in dataset.cells.cell(cell).unwrap_or(&[]) {
            let Some(fraction) = weights[node as usize].fraction(weight, dimension) else {
                continue;
            };
            for (mean, &value) in mean[node as usize].iter_mut().zip(value) {
                *mean = blend(*mean, value, fraction);
            }
        }
    }
    let mut all_supported = true;
    for (mean, weight) in mean.iter_mut().zip(&weights) {
        if weight.sum == 0.0 {
            all_supported = false;
            *mean = [f64::NAN; N];
        }
    }
    ComputedRows {
        values: mean,
        all_supported,
    }
}

fn append(
    dataset: &DataSet,
    mut field: Field,
    source: &Field,
    association: Association,
    all_supported: bool,
) -> DataSet {
    field.association = association;
    field.label.clone_from(&source.label);
    field.unit.clone_from(&source.unit);
    let field = if all_supported {
        field
    } else {
        crate::validity::finish(field, |_| true)
    };
    let mut output = dataset.clone();
    let fields = match association {
        Association::Point => &mut output.point_data,
        Association::Cell => &mut output.cell_data,
    };
    fields.retain(|existing| existing.name != field.name);
    fields.push(field);
    output
}

/// Averages valid point values into cells, component by component.
///
/// Scalar, vector, tensor and complex fields use equal valid-node weights.
/// Mixed-sign components and overflowing sums use an exact bounded average;
/// same-sign ordinary sums retain the vectorized floating-point path.
/// Cells with no valid contributors receive masked NaNs. Labels/units survive;
/// an existing output name in cell data is replaced.
///
/// # Errors
///
/// Returns an error for an invalid dataset or a missing point field.
pub fn point_data_to_cell_data(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    point_data_to_cell_data_validated(dataset.validated()?, field_name, out_name)
}

/// Converts a point field using an existing immutable validation view.
///
/// Has the same averaging/validity semantics as [`point_data_to_cell_data`],
/// without repeating input validation. The borrow prevents source mutation.
///
/// # Errors
///
/// Returns an error when the source field does not exist.
pub fn point_data_to_cell_data_validated(
    dataset: ValidatedDataSet<'_>,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let dataset = dataset.as_dataset();
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let converted = match &field.data {
        FieldData::Scalar(values) => Field::scalar(
            out_name,
            point_means(dataset, field, values.as_chunks::<1>().0).into_flattened(),
        ),
        FieldData::Vector(values) => Field::vector(out_name, point_means(dataset, field, values)),
        FieldData::Tensor(values) => Field::tensor(out_name, point_means(dataset, field, values)),
        FieldData::Complex(values) => Field::complex(out_name, point_means(dataset, field, values)),
    };
    Ok(append(
        dataset,
        converted,
        field,
        Association::Cell,
        field.valid.is_none(),
    ))
}

/// Averages valid incident cells into points by positive geometric measure.
///
/// At each point only the highest supported cell dimension contributes: volume,
/// area, length, then unit vertex weight. Degenerate/nonrepresentable measures
/// and masked cells do not contribute. No supported contribution means a masked
/// NaN. All field shapes average componentwise; labels/units survive and an
/// existing output name in point data is replaced.
///
/// # Errors
///
/// Returns an error for an invalid dataset or a missing cell field.
pub fn cell_data_to_point_data(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    cell_data_to_point_data_validated(dataset.validated()?, field_name, out_name)
}

/// Converts a cell field using an existing immutable validation view.
///
/// Has the same averaging/validity semantics as [`cell_data_to_point_data`],
/// without repeating input validation. The borrow prevents source mutation.
///
/// # Errors
///
/// Returns an error when the source field does not exist.
pub fn cell_data_to_point_data_validated(
    dataset: ValidatedDataSet<'_>,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let uniform = dataset.uniform_cell_dimension().is_some();
    let dataset = dataset.as_dataset();
    let field = dataset
        .field(Association::Cell, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let (converted, all_supported) = match &field.data {
        FieldData::Scalar(values) => {
            let rows = cell_means(dataset, field, values.as_chunks::<1>().0, uniform);
            (
                Field::scalar(
                    out_name,
                    rows.values
                        .into_iter()
                        .map(|row| row[0])
                        .collect::<Vec<_>>(),
                ),
                rows.all_supported,
            )
        }
        FieldData::Vector(values) => {
            let rows = cell_means(dataset, field, values, uniform);
            (Field::vector(out_name, rows.values), rows.all_supported)
        }
        FieldData::Tensor(values) => {
            let rows = cell_means(dataset, field, values, uniform);
            (Field::tensor(out_name, rows.values), rows.all_supported)
        }
        FieldData::Complex(values) => {
            let rows = cell_means(dataset, field, values, uniform);
            (Field::complex(out_name, rows.values), rows.all_supported)
        }
    };
    Ok(append(
        dataset,
        converted,
        field,
        Association::Point,
        all_supported,
    ))
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|axis| a[axis] - b[axis])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn edge_scale(vectors: &[[f64; 3]]) -> f64 {
    vectors
        .iter()
        .flatten()
        .map(|v| v.abs())
        .fold(0.0, f64::max)
}
fn determinant(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let bc = cross(b, c);
    a[0] * bc[0] + a[1] * bc[1] + a[2] * bc[2]
}

fn triangle_area(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let area = 0.5 * crate::calculator::magnitude(cross(ab, ac));
    if area.is_finite() && area > 0.0 {
        return area;
    }
    let scale = edge_scale(&[ab, ac]);
    if scale == 0.0 || !scale.is_finite() {
        return area;
    }
    let normalized = cross(ab.map(|v| v / scale), ac.map(|v| v / scale));
    (0.5 * crate::calculator::magnitude(normalized) * scale) * scale
}

fn polygon_area(dataset: &DataSet, nodes: &[u32]) -> f64 {
    let origin = dataset.positions[nodes[0] as usize];
    let mut sum = [0.0; 3];
    let mut scale = 0.0f64;
    for pair in nodes[1..].windows(2) {
        let ab = sub(dataset.positions[pair[0] as usize], origin);
        let ac = sub(dataset.positions[pair[1] as usize], origin);
        scale = scale.max(edge_scale(&[ab, ac]));
        for (value, component) in sum.iter_mut().zip(cross(ab, ac)) {
            *value += component;
        }
    }
    let area = 0.5 * crate::calculator::magnitude(sum);
    if area.is_finite() && area > 0.0 || scale == 0.0 || !scale.is_finite() {
        return area;
    }
    let mut sum = [0.0; 3];
    for pair in nodes[1..].windows(2) {
        let ab = sub(dataset.positions[pair[0] as usize], origin).map(|v| v / scale);
        let ac = sub(dataset.positions[pair[1] as usize], origin).map(|v| v / scale);
        for (value, component) in sum.iter_mut().zip(cross(ab, ac)) {
            *value += component;
        }
    }
    (0.5 * crate::calculator::magnitude(sum) * scale) * scale
}

fn tet_volume(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> f64 {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ad = sub(d, a);
    let volume = determinant(ab, ac, ad).abs() / 6.0;
    if volume.is_finite() && volume > 0.0 {
        return volume;
    }
    let scale = edge_scale(&[ab, ac, ad]);
    if scale == 0.0 || !scale.is_finite() {
        return volume;
    }
    let det = determinant(
        ab.map(|v| v / scale),
        ac.map(|v| v / scale),
        ad.map(|v| v / scale),
    );
    ((det.abs() * scale / 6.0) * scale) * scale
}

fn cell_measure(dataset: &DataSet, cell: usize) -> f64 {
    let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
    let p = |index: usize| dataset.positions[nodes[index] as usize];
    match dataset.cells.types[cell] {
        CellType::Vertex => 1.0,
        CellType::Line => crate::calculator::magnitude(sub(p(1), p(0))),
        CellType::Triangle => triangle_area(p(0), p(1), p(2)),
        CellType::Quad => triangle_area(p(0), p(1), p(2)) + triangle_area(p(0), p(2), p(3)),
        CellType::Polygon => polygon_area(dataset, nodes),
        kind => gpui_mesh::tetrahedra(kind, nodes)
            .map(|tet| {
                tet_volume(
                    dataset.positions[tet[0] as usize],
                    dataset.positions[tet[1] as usize],
                    dataset.positions[tet[2] as usize],
                    dataset.positions[tet[3] as usize],
                )
            })
            .sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_recovers_cell_average() {
        let ds = DataSet::surface(
            "s",
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        )
        .with_field(Field::scalar("p", vec![0.0, 2.0, 4.0]));
        let cells = point_data_to_cell_data(&ds, "p", "p_cell").unwrap();
        let values = cells
            .field(Association::Cell, "p_cell")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert_eq!(values, &[2.0]);
        let back = cell_data_to_point_data(&cells, "p_cell", "p_back").unwrap();
        let values = back
            .field(Association::Point, "p_back")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert_eq!(values, &[2.0, 2.0, 2.0]);
    }
}
