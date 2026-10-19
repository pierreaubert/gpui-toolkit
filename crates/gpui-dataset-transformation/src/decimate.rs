//! Validity-aware vertex clustering with packed cells and bounded numeric fallback.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use crate::mean::ExactMean;
use gpui_mesh::{Cells, DataSet, Field, FieldData};
use std::collections::{HashMap, HashSet, hash_map::Entry};

/// Clusters vertices, averaging supported samples and dropping collapsed cells.
///
/// Cubes have width `cell_size` and origin zero. Positions use all cluster
/// members; scalar/vector/tensor/complex point fields average only supported
/// members. Empty field support yields masked NaNs. Cell fields are subset with
/// their masks. Vertex cells remain valid. Output order follows first input use.
///
/// # Errors
///
/// Returns an error for invalid data, nonfinite/nonpositive cell size or output
/// indices exceeding u32. This preview/LOD filter does not preserve sharp features.
pub fn decimate(dataset: &DataSet, cell_size: f64) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    if !cell_size.is_finite() || cell_size <= 0.0 {
        return Err(TransformError::Degenerate(
            "decimation cell size must be finite and positive".into(),
        ));
    }
    let mut clusters = HashMap::new();
    let mut assignment = Vec::with_capacity(dataset.point_count());
    for &point in dataset.positions.iter() {
        let next = clusters.len();
        let id = match clusters.entry(key(point, cell_size)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(crate::geometry::output_base(next, 1)?),
        };
        assignment.push(id);
    }
    let count = clusters.len();
    drop(clusters);
    let identity = count == dataset.point_count();
    let mut groups = (!identity).then(|| Groups {
        assignment: &assignment,
        counts: vec![0; count],
        members: None,
    });
    let positions = match &mut groups {
        Some(groups) => groups.rows(&dataset.positions, None).into(),
        None => dataset.positions.clone(),
    };
    // Fixed cells have at most eight nodes. Large polygons reuse one hash set
    // instead of paying quadratic work or allocating a set for every cell.
    const SMALL_CELL: usize = 16;
    let mut seen = HashSet::new();
    let keep = (0..dataset.cell_count())
        .map(|cell| {
            let nodes = dataset.cells.cell(cell).unwrap();
            if nodes.len() > SMALL_CELL {
                seen.clear();
                nodes
                    .iter()
                    .all(|&node| seen.insert(assignment[node as usize]))
            } else {
                nodes.iter().enumerate().all(|(index, &node)| {
                    !nodes[..index]
                        .iter()
                        .any(|&earlier| assignment[earlier as usize] == assignment[node as usize])
                })
            }
        })
        .collect::<Vec<_>>();
    drop(seen);
    if identity && keep.iter().all(|keep| *keep) {
        let mut result = dataset.clone();
        result.id = format!("{}-decimated", dataset.id).into();
        return Ok(result);
    }
    let kept = (0..dataset.cell_count())
        .filter(|&cell| keep[cell])
        .collect::<Vec<_>>();
    drop(keep);
    let cells = if identity && kept.len() == dataset.cell_count() {
        dataset.cells.clone()
    } else {
        let connectivity_count = kept
            .iter()
            .map(|&cell| dataset.cells.cell(cell).unwrap().len())
            .sum();
        let mut types = Vec::with_capacity(kept.len());
        let mut offsets = Vec::with_capacity(kept.len() + 1);
        let mut connectivity = Vec::with_capacity(connectivity_count);
        offsets.push(0);
        for &cell in &kept {
            types.push(dataset.cells.types[cell]);
            connectivity.extend(
                dataset
                    .cells
                    .cell(cell)
                    .unwrap()
                    .iter()
                    .map(|&node| assignment[node as usize]),
            );
            offsets.push(connectivity.len() as u64);
        }
        Cells {
            types: types.into(),
            offsets: offsets.into(),
            connectivity: connectivity.into(),
        }
    };
    let mut out = DataSet::new(format!("{}-decimated", dataset.id), positions, cells);
    out.point_data = dataset
        .point_data
        .iter()
        .map(|field| match &mut groups {
            Some(groups) => groups.field(field),
            None => field.clone(),
        })
        .collect();
    out.cell_data = dataset
        .cell_data
        .iter()
        .map(|field| crate::threshold::pick(field, &kept))
        .collect();
    Ok(out)
}

fn key(point: [f64; 3], size: f64) -> ([u64; 3], u8) {
    let mut bits = [0; 3];
    let mut exact = 0;
    for axis in 0..3 {
        let coordinate = (point[axis] / size).floor();
        // Beyond 2^53 cells, adjacent representable input coordinates are at
        // least one cell apart. Keep their bits rather than saturating an i64
        // cast or collapsing overflowing quotients to the same infinity.
        if coordinate.abs() < (1_u64 << 53) as f64 {
            bits[axis] = if coordinate == 0.0 {
                0
            } else {
                coordinate.to_bits()
            };
        } else {
            bits[axis] = point[axis].to_bits();
            exact |= 1 << axis;
        }
    }
    (bits, exact)
}

struct Members {
    offsets: Vec<usize>,
    points: Vec<usize>,
}

impl Members {
    fn new(assignment: &[u32], groups: usize) -> Self {
        let mut offsets = vec![0; groups + 1];
        for &group in assignment {
            offsets[group as usize + 1] += 1;
        }
        for index in 1..offsets.len() {
            offsets[index] += offsets[index - 1];
        }
        let mut points = vec![0; assignment.len()];
        for (point, &group) in assignment.iter().enumerate().rev() {
            offsets[group as usize + 1] -= 1;
            points[offsets[group as usize + 1]] = point;
        }
        offsets.copy_within(1.., 0);
        offsets[groups] = assignment.len();
        Self { offsets, points }
    }
}

struct Groups<'a> {
    assignment: &'a [u32],
    counts: Vec<usize>,
    members: Option<Members>,
}

impl Groups<'_> {
    fn rows<const N: usize>(
        &mut self,
        values: &[[f64; N]],
        valid: Option<&[bool]>,
    ) -> Vec<[f64; N]> {
        self.counts.fill(0);
        let count = self.counts.len();
        let mut sums = vec![[0.0_f64; N]; count];
        let mut corrections = vec![[0.0; N]; count];
        for (index, (&group, value)) in self.assignment.iter().zip(values).enumerate() {
            if valid.is_some_and(|mask| !mask[index]) {
                continue;
            }
            let group = group as usize;
            self.counts[group] += 1;
            for axis in 0..N {
                let sum = &mut sums[group][axis];
                let next = *sum + value[axis];
                corrections[group][axis] += if sum.abs() >= value[axis].abs() {
                    (*sum - next) + value[axis]
                } else {
                    (value[axis] - next) + *sum
                };
                *sum = next;
            }
        }
        for (group, sum) in sums.iter_mut().enumerate() {
            if self.counts[group] == 0 {
                *sum = [f64::NAN; N];
                continue;
            }
            let result = std::array::from_fn(|axis| {
                (sum[axis] + corrections[group][axis]) / self.counts[group] as f64
            });
            if result.iter().all(|value| value.is_finite()) {
                *sum = result;
                continue;
            }
            // Only an overflowing compensated sum needs exact arithmetic. A
            // lazily built membership table is reused across all field shapes.
            let members = self
                .members
                .get_or_insert_with(|| Members::new(self.assignment, count));
            let mut exact = std::array::from_fn::<_, N, _>(|_| ExactMean::default());
            for &point in &members.points[members.offsets[group]..members.offsets[group + 1]] {
                if valid.is_some_and(|mask| !mask[point]) {
                    continue;
                }
                for (mean, &value) in exact.iter_mut().zip(&values[point]) {
                    mean.add(value);
                }
            }
            *sum = exact.map(ExactMean::finish);
        }
        sums
    }

    fn field(&mut self, source: &Field) -> Field {
        let mask = source.valid.as_deref();
        let data = match &source.data {
            FieldData::Scalar(v) => FieldData::Scalar(
                self.rows(v.as_chunks::<1>().0, mask)
                    .into_flattened()
                    .into(),
            ),
            FieldData::Vector(v) => FieldData::Vector(self.rows(v, mask).into()),
            FieldData::Tensor(v) => FieldData::Tensor(self.rows(v, mask).into()),
            FieldData::Complex(v) => FieldData::Complex(self.rows(v, mask).into()),
        };
        crate::validity::finish(
            Field {
                name: source.name.clone(),
                label: source.label.clone(),
                unit: source.unit.clone(),
                association: source.association,
                data,
                valid: None,
            },
            |_| true,
        )
    }
}
