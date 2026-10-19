//! Validated time axes and immutable sampling plans for changing datasets.

// Rust guideline compliant 2026-02-21

use crate::dataset::{DataSet, MeshError};
use crate::field::{Association, Field, FieldData};
use std::sync::Arc;

/// Dataset states at increasing finite times.
///
/// States may deform but share topology and point counts. Fields absent from
/// one step follow nearest-step selection. Shared fields interpolate by name,
/// requiring compatible shapes and units when sampled.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSeries {
    /// State at each time step.
    pub steps: Vec<DataSet>,
    /// Physical times, finite and strictly increasing.
    pub times: Vec<f64>,
}

fn invalid(reason: impl Into<String>) -> MeshError {
    MeshError::InvalidTimeSeries {
        reason: reason.into(),
    }
}

/// An immutable, validated view of a time series' timestamps.
///
/// Obtain it through [`TimeSeries::time_axis`] for repeated lookup without
/// rescanning timestamps. The borrow prevents mutation of the source axis.
///
/// ```compile_fail
/// use gpui_mesh::{Cells,DataSet,TimeSeries};
/// let data=DataSet::new("point",vec![[0.0;3]],Cells::from_cells(&[]));
/// let mut series=TimeSeries::new(vec![data],vec![0.0]).unwrap();
/// let axis=series.time_axis().unwrap();
/// series.steps.clear();
/// axis.locate(0.0).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TimeAxis<'a> {
    times: &'a [f64],
}

impl TimeAxis<'_> {
    /// First and last physical time, including single-step axes.
    #[must_use]
    pub fn range(self) -> [f64; 2] {
        [self.times[0], self.times[self.times.len() - 1]]
    }

    /// Locates finite physical time, clamping to the axis endpoints.
    ///
    /// # Errors
    ///
    /// Returns an error for NaN or infinite query time.
    pub fn locate(self, time: f64) -> Result<(usize, usize, f64), MeshError> {
        if !time.is_finite() {
            return Err(invalid("query time must be finite"));
        }
        let [first, last] = self.range();
        if time <= first {
            return Ok((0, 0, 0.0));
        }
        let end = self.times.len() - 1;
        if time >= last {
            return Ok((end, end, 0.0));
        }
        let hi = self.times.partition_point(|&t| t <= time);
        let lo = hi - 1;
        let span = self.times[hi] - self.times[lo];
        let fraction = if span.is_finite() {
            (time - self.times[lo]) / span
        } else {
            (time * 0.5 - self.times[lo] * 0.5) / (self.times[hi] * 0.5 - self.times[lo] * 0.5)
        };
        Ok((lo, hi, fraction))
    }

    /// Maps a normalized fraction to finite physical time.
    ///
    /// # Errors
    ///
    /// Returns an error unless the fraction is finite and in `[0,1]`.
    pub fn at_fraction(self, fraction: f64) -> Result<f64, MeshError> {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(invalid("time fraction must be finite and in [0,1]"));
        }
        let [first, last] = self.range();
        Ok(lerp(first, last, fraction))
    }
}

impl TimeSeries {
    /// Creates and validates datasets and timestamps.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed datasets, times or shared topology.
    pub fn new(steps: Vec<DataSet>, times: Vec<f64>) -> Result<Self, MeshError> {
        let series = Self { steps, times };
        series.validate()?;
        Ok(series)
    }

    /// Validates timestamps, all datasets, point counts and topology.
    ///
    /// # Errors
    ///
    /// Returns an error for empty/inconsistent times, invalid datasets or
    /// incompatible topology. Sampling plans separately check named-field
    /// compatibility, allowing a sequence to represent different field sets.
    pub fn validate(&self) -> Result<(), MeshError> {
        self.time_axis()?;
        let first = &self.steps[0];
        for step in &self.steps {
            step.validate()?;
            validate_topology(first, step)?;
        }
        Ok(())
    }

    /// Validates timestamps and returns a reusable immutable axis.
    ///
    /// # Errors
    ///
    /// Returns an error for empty, mismatched, nonfinite or unordered times.
    pub fn time_axis(&self) -> Result<TimeAxis<'_>, MeshError> {
        if self.steps.is_empty() || self.steps.len() != self.times.len() {
            return Err(invalid(
                "steps/times must be nonempty and have equal lengths",
            ));
        }
        if self.times.iter().any(|t| !t.is_finite()) || self.times.windows(2).any(|w| w[1] <= w[0])
        {
            return Err(invalid("times must be finite and strictly increasing"));
        }
        Ok(TimeAxis { times: &self.times })
    }

    /// Number of steps.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the series is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Validates the axis and locates a finite query time.
    ///
    /// Retain [`Self::time_axis`] to avoid repeated validation scans.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid timestamps or a nonfinite query.
    pub fn locate(&self, time: f64) -> Result<(usize, usize, f64), MeshError> {
        self.time_axis()?.locate(time)
    }

    /// Samples finite time, validating the axis and selected datasets.
    ///
    /// Shared fields require valid support from both steps. Missing computed
    /// entries receive masked NaNs. Fields present on only one side follow the
    /// nearer step (ties choose the later step). Order/labels follow that step.
    /// Unchanged storage may be shared; raw masked values remain unspecified.
    /// Endpoints and clamped queries share the selected source storage.
    ///
    /// Retain [`Self::sampler`] or [`Self::prepare`] for repeated sampling.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid queries/selected data, incompatible topology,
    /// or same-name fields with different shapes or units.
    pub fn sample(&self, time: f64) -> Result<DataSet, MeshError> {
        let (lo, hi, fraction) = self.locate(time)?;
        let a = &self.steps[lo];
        a.validate()?;
        if lo == hi || fraction == 0.0 {
            return Ok(a.clone());
        }
        let b = &self.steps[hi];
        b.validate()?;
        validate_topology(a, b)?;
        if fraction == 1.0 {
            return Ok(b.clone());
        }
        let pair = PairPlan::new(a, b)?;
        Ok(sample_pair(a, b, &pair, fraction))
    }

    /// Builds a reusable plan borrowing immutable source state.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid data, axes, topology, shapes or units.
    ///
    /// # Examples
    ///
    /// ```
    /// use gpui_mesh::{Cells,DataSet,TimeSeries};
    /// let data=DataSet::new("point",vec![[0.0;3]],Cells::from_cells(&[]));
    /// let series=TimeSeries::new(vec![data.clone(),data],vec![0.0,1.0]).unwrap();
    /// let sampler=series.sampler().unwrap();
    /// assert_eq!(sampler.sample(0.5).unwrap().point_count(),1);
    /// ```
    pub fn sampler(&self) -> Result<TimeSeriesSampler<'_>, MeshError> {
        self.validate()?;
        Ok(TimeSeriesSampler {
            series: self,
            plan: SamplingPlan::new(self)?,
        })
    }

    /// Takes ownership and prepares immutable playback state without array copies.
    ///
    /// # Errors
    ///
    /// Returns the validation/compatibility errors from [`Self::sampler`].
    pub fn prepare(self) -> Result<PreparedTimeSeries, MeshError> {
        PreparedTimeSeries::new(self)
    }
}

fn validate_topology(a: &DataSet, b: &DataSet) -> Result<(), MeshError> {
    if a.point_count() != b.point_count() {
        return Err(invalid("steps have different point counts"));
    }
    if a.cells != b.cells {
        return Err(invalid("steps have different topology"));
    }
    Ok(())
}

/// A prepared sampler borrowing immutable source datasets.
///
/// Validation/schema matching run once. Drop the sampler before changing source
/// state and prepare again.
///
/// ```compile_fail
/// use gpui_mesh::{Cells,DataSet,TimeSeries};
/// let data=DataSet::new("point",vec![[0.0;3]],Cells::from_cells(&[]));
/// let mut series=TimeSeries::new(vec![data],vec![0.0]).unwrap();
/// let sampler=series.sampler().unwrap();
/// series.times[0]=f64::NAN;
/// sampler.sample(0.0).unwrap();
/// ```
#[derive(Debug)]
pub struct TimeSeriesSampler<'a> {
    series: &'a TimeSeries,
    plan: SamplingPlan,
}

impl TimeSeriesSampler<'_> {
    /// Samples finite time without repeated validation or schema matching.
    ///
    /// # Errors
    ///
    /// Returns an error for a nonfinite query time.
    pub fn sample(&self, time: f64) -> Result<DataSet, MeshError> {
        self.plan.sample(self.series, time)
    }
}

/// Shared immutable datasets and reusable interpolation plans.
///
/// Cloning shares datasets/plans. Source state cannot be mutated through this
/// owner, so samples need no validation rescan.
#[derive(Debug, Clone)]
pub struct PreparedTimeSeries {
    state: Arc<PreparedState>,
}

#[derive(Debug)]
struct PreparedState {
    series: TimeSeries,
    plan: SamplingPlan,
}

impl PreparedTimeSeries {
    /// Takes ownership and prepares a sequence without dataset-array copies.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid data, axes, topology, field shapes or units.
    pub fn new(series: TimeSeries) -> Result<Self, MeshError> {
        series.validate()?;
        let plan = SamplingPlan::new(&series)?;
        Ok(Self {
            state: Arc::new(PreparedState { series, plan }),
        })
    }

    /// Returns the immutable source sequence.
    #[must_use]
    pub fn as_series(&self) -> &TimeSeries {
        &self.state.series
    }

    /// Returns a validated axis without rescanning timestamps.
    #[must_use]
    pub fn time_axis(&self) -> TimeAxis<'_> {
        TimeAxis {
            times: &self.state.series.times,
        }
    }

    /// Samples finite time without repeated input validation.
    ///
    /// # Errors
    ///
    /// Returns an error for a nonfinite query time.
    pub fn sample(&self, time: f64) -> Result<DataSet, MeshError> {
        self.state.plan.sample(&self.state.series, time)
    }
}

#[derive(Debug)]
struct SamplingPlan {
    pairs: Vec<PairPlan>,
}

impl SamplingPlan {
    fn new(series: &TimeSeries) -> Result<Self, MeshError> {
        Ok(Self {
            pairs: series
                .steps
                .windows(2)
                .map(|p| PairPlan::new(&p[0], &p[1]))
                .collect::<Result<_, _>>()?,
        })
    }
    fn sample(&self, series: &TimeSeries, time: f64) -> Result<DataSet, MeshError> {
        let (lo, hi, fraction) = TimeAxis {
            times: &series.times,
        }
        .locate(time)?;
        if lo == hi || fraction == 0.0 {
            return Ok(series.steps[lo].clone());
        }
        if fraction == 1.0 {
            return Ok(series.steps[hi].clone());
        }
        Ok(sample_pair(
            &series.steps[lo],
            &series.steps[hi],
            &self.pairs[lo],
            fraction,
        ))
    }
}

#[derive(Debug, Clone, Copy)]
enum MaskPlan {
    None,
    A,
    B,
    Intersection,
}

#[derive(Debug, Clone, Copy)]
struct FieldPlan {
    other: usize,
    data_shared: bool,
    mask: MaskPlan,
    fast: bool,
}

#[derive(Debug)]
struct PairPlan {
    positions_shared: bool,
    positions_fast: bool,
    points_a: Vec<FieldPlan>,
    points_b: Vec<FieldPlan>,
    cells_a: Vec<FieldPlan>,
    cells_b: Vec<FieldPlan>,
}

impl PairPlan {
    fn new(a: &DataSet, b: &DataSet) -> Result<Self, MeshError> {
        let (points_a, points_b) = field_plans(&a.point_data, &b.point_data)?;
        let (cells_a, cells_b) = field_plans(&a.cell_data, &b.cell_data)?;
        let positions_shared =
            Arc::ptr_eq(&a.positions, &b.positions) || a.positions == b.positions;
        Ok(Self {
            positions_shared,
            positions_fast: positions_shared || safe_rows(&a.positions, &b.positions, None, None),
            points_a,
            points_b,
            cells_a,
            cells_b,
        })
    }
}

fn field_plans(
    base: &[Field],
    other: &[Field],
) -> Result<(Vec<FieldPlan>, Vec<FieldPlan>), MeshError> {
    let aligned =
        base.len() == other.len() && base.iter().zip(other).all(|(a, b)| a.name == b.name);
    let mut sorted = if aligned {
        Vec::new()
    } else {
        (0..other.len()).collect::<Vec<_>>()
    };
    sorted.sort_unstable_by(|&a, &b| other[a].name.cmp(&other[b].name));
    let forward: Vec<FieldPlan> = base
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let matched = if aligned {
                Some(index)
            } else {
                sorted
                    .binary_search_by(|&i| other[i].name.cmp(&field.name))
                    .ok()
                    .map(|i| sorted[i])
            };
            let Some(index) = matched else {
                return Ok(FieldPlan {
                    other: usize::MAX,
                    data_shared: true,
                    mask: MaskPlan::None,
                    fast: true,
                });
            };
            let second = &other[index];
            if std::mem::discriminant(&field.data) != std::mem::discriminant(&second.data)
                || field.data.len() != second.data.len()
            {
                return Err(invalid(format!(
                    "field '{}' changes shape between steps",
                    field.name
                )));
            }
            if field.unit != second.unit {
                return Err(invalid(format!(
                    "field '{}' changes units between steps",
                    field.name
                )));
            }
            let shared = shares_data(&field.data, &second.data) || field.data == second.data;
            let mask = mask_plan(field, second);
            let fast = shared || safe_field(field, second);
            Ok(FieldPlan {
                other: index,
                data_shared: shared,
                mask,
                fast,
            })
        })
        .collect::<Result<_, _>>()?;
    let mut reverse = vec![
        FieldPlan {
            other: usize::MAX,
            data_shared: true,
            mask: MaskPlan::None,
            fast: true,
        };
        other.len()
    ];
    for (index, plan) in forward.iter().enumerate() {
        if plan.other != usize::MAX {
            reverse[plan.other] = FieldPlan {
                other: index,
                ..*plan
            };
        }
    }
    Ok((forward, reverse))
}

fn shares_data(a: &FieldData, b: &FieldData) -> bool {
    match (a, b) {
        (FieldData::Scalar(a), FieldData::Scalar(b)) => Arc::ptr_eq(a, b),
        (FieldData::Vector(a), FieldData::Vector(b)) => Arc::ptr_eq(a, b),
        (FieldData::Tensor(a), FieldData::Tensor(b)) => Arc::ptr_eq(a, b),
        (FieldData::Complex(a), FieldData::Complex(b)) => Arc::ptr_eq(a, b),
        _ => false,
    }
}

fn mask_plan(a: &Field, b: &Field) -> MaskPlan {
    match (&a.valid, &b.valid) {
        (None, None) => MaskPlan::None,
        (Some(_), None) => MaskPlan::A,
        (None, Some(_)) => MaskPlan::B,
        (Some(a), Some(b)) => {
            if Arc::ptr_eq(a, b) {
                return MaskPlan::A;
            }
            let mut a_subset = true;
            let mut b_subset = true;
            for (&a, &b) in a.iter().zip(b.iter()) {
                a_subset &= !a || b;
                b_subset &= !b || a;
            }
            if a_subset {
                MaskPlan::A
            } else if b_subset {
                MaskPlan::B
            } else {
                MaskPlan::Intersection
            }
        }
    }
}

fn safe_value(value: f64) -> bool {
    // Preparation proves a conservative normal range once. The ordinary
    // kernel can then vectorize without per-component overflow checks.
    value == 0.0 || (1e-150..=1e150).contains(&value.abs())
}

fn safe_rows<const N: usize>(
    a: &[[f64; N]],
    b: &[[f64; N]],
    ma: Option<&[bool]>,
    mb: Option<&[bool]>,
) -> bool {
    a.iter().zip(b).enumerate().all(|(index, (a, b))| {
        ma.is_some_and(|mask| !mask[index])
            || mb.is_some_and(|mask| !mask[index])
            || a.iter().chain(b).all(|&v| safe_value(v))
    })
}

fn safe_field(a: &Field, b: &Field) -> bool {
    let ma = a.valid.as_deref();
    let mb = b.valid.as_deref();
    match (&a.data, &b.data) {
        (FieldData::Scalar(a), FieldData::Scalar(b)) => {
            safe_rows(a.as_chunks::<1>().0, b.as_chunks::<1>().0, ma, mb)
        }
        (FieldData::Vector(a), FieldData::Vector(b)) => safe_rows(a, b, ma, mb),
        (FieldData::Tensor(a), FieldData::Tensor(b)) => safe_rows(a, b, ma, mb),
        (FieldData::Complex(a), FieldData::Complex(b)) => safe_rows(a, b, ma, mb),
        _ => false,
    }
}

fn sample_pair(a: &DataSet, b: &DataSet, plan: &PairPlan, t: f64) -> DataSet {
    let near_b = t >= 0.5;
    let positions = if plan.positions_shared {
        a.positions.clone()
    } else {
        blend_rows(&a.positions, &b.positions, t, None, plan.positions_fast).into()
    };
    let mut result = DataSet::new(
        if near_b { b.id.clone() } else { a.id.clone() },
        positions,
        a.cells.clone(),
    );
    for association in [Association::Point, Association::Cell] {
        let (fa, fb, pa, pb) = match association {
            Association::Point => (&a.point_data, &b.point_data, &plan.points_a, &plan.points_b),
            Association::Cell => (&a.cell_data, &b.cell_data, &plan.cells_a, &plan.cells_b),
        };
        let (base, other, plans) = if near_b { (fb, fa, pb) } else { (fa, fb, pa) };
        let target = match association {
            Association::Point => &mut result.point_data,
            Association::Cell => &mut result.cell_data,
        };
        target.reserve(base.len());
        for (field, plan) in base.iter().zip(plans) {
            if plan.other == usize::MAX {
                target.push(field.clone());
                continue;
            }
            let other = &other[plan.other];
            let (a, b) = if near_b {
                (other, field)
            } else {
                (field, other)
            };
            let mask: Option<Arc<[bool]>> = match plan.mask {
                MaskPlan::None => None,
                MaskPlan::A => a.valid.clone(),
                MaskPlan::B => b.valid.clone(),
                MaskPlan::Intersection => Some(
                    a.valid
                        .as_ref()
                        .unwrap()
                        .iter()
                        .zip(b.valid.as_ref().unwrap().iter())
                        .map(|(&a, &b)| a && b)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            };
            let data = if plan.data_shared {
                // Keep payload identity stable across the interval even when
                // the later step stores an equal independent allocation.
                a.data.clone()
            } else {
                match (&a.data, &b.data) {
                    (FieldData::Scalar(a), FieldData::Scalar(b)) => FieldData::Scalar(
                        blend_rows(
                            a.as_chunks::<1>().0,
                            b.as_chunks::<1>().0,
                            t,
                            mask.as_deref(),
                            plan.fast,
                        )
                        .into_flattened()
                        .into(),
                    ),
                    (FieldData::Vector(a), FieldData::Vector(b)) => {
                        FieldData::Vector(blend_rows(a, b, t, mask.as_deref(), plan.fast).into())
                    }
                    (FieldData::Tensor(a), FieldData::Tensor(b)) => {
                        FieldData::Tensor(blend_rows(a, b, t, mask.as_deref(), plan.fast).into())
                    }
                    (FieldData::Complex(a), FieldData::Complex(b)) => {
                        FieldData::Complex(blend_rows(a, b, t, mask.as_deref(), plan.fast).into())
                    }
                    _ => unreachable!("validated sampling plan preserves field shapes"),
                }
            };
            target.push(Field {
                name: field.name.clone(),
                label: field.label.clone(),
                unit: field.unit.clone(),
                association,
                data,
                valid: mask,
            });
        }
    }
    result
}

fn blend_rows<const N: usize>(
    a: &[[f64; N]],
    b: &[[f64; N]],
    t: f64,
    mask: Option<&[bool]>,
    fast: bool,
) -> Vec<[f64; N]> {
    if fast {
        // Choose the nearer endpoint once per field, preserving tiny values
        // near either endpoint without introducing a per-component branch.
        if t <= 0.5 {
            blend_rows_with(a, b, mask, |a, b| {
                std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
            })
        } else {
            let remaining = 1.0 - t;
            blend_rows_with(a, b, mask, |a, b| {
                std::array::from_fn(|i| b[i] + (a[i] - b[i]) * remaining)
            })
        }
    } else {
        blend_rows_with(a, b, mask, |a, b| {
            std::array::from_fn(|i| lerp(a[i], b[i], t))
        })
    }
}

fn blend_rows_with<const N: usize>(
    a: &[[f64; N]],
    b: &[[f64; N]],
    mask: Option<&[bool]>,
    row: impl Fn(&[f64; N], &[f64; N]) -> [f64; N],
) -> Vec<[f64; N]> {
    match mask {
        None => a.iter().zip(b).map(|(a, b)| row(a, b)).collect(),
        Some(mask) => a
            .iter()
            .zip(b)
            .zip(mask)
            .map(|((a, b), &valid)| if valid { row(a, b) } else { [f64::NAN; N] })
            .collect(),
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    if t == 0.0 || a == b {
        return a;
    }
    if t == 1.0 {
        return b;
    }
    let value = a * (1.0 - t) + b * t;
    if value.is_normal() {
        return value;
    }
    let scale = a.abs().max(b.abs());
    ((a / scale) * (1.0 - t) + (b / scale) * t).clamp(-1.0, 1.0) * scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::Field;

    fn step(z: f64, p: f64) -> DataSet {
        DataSet::surface(
            "s",
            vec![[0.0, 0.0, z], [1.0, 0.0, z], [0.0, 1.0, z]],
            &[[0, 1, 2]],
        )
        .with_field(Field::scalar("pressure", vec![p, p, p]))
    }

    #[test]
    fn locates_and_clamps_time() {
        let series = TimeSeries::new(vec![step(0.0, 0.0), step(1.0, 2.0)], vec![0.0, 2.0]).unwrap();
        assert_eq!(series.locate(-1.0).unwrap(), (0, 0, 0.0));
        assert_eq!(series.locate(1.0).unwrap(), (0, 1, 0.5));
        assert_eq!(series.locate(9.0).unwrap(), (1, 1, 0.0));
    }

    #[test]
    fn rejects_topology_change() {
        let other = DataSet::surface(
            "s",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
            &[[0, 1, 2], [1, 3, 2]],
        );
        TimeSeries::new(vec![step(0.0, 0.0), other], vec![0.0, 1.0]).unwrap_err();
    }
}

#[cfg(test)]
mod sample_tests {
    use super::*;
    use crate::field::Field;

    #[test]
    fn sample_interpolates_positions_and_fields() {
        let a = DataSet::structured_grid("g", [2, 2, 2], [0.0; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("p", vec![0.0; 8]));
        let b = DataSet::structured_grid("g", [2, 2, 2], [2.0, 0.0, 0.0], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("p", vec![10.0; 8]));
        let series = TimeSeries::new(vec![a, b], vec![0.0, 2.0]).unwrap();
        let mid = series.sample(1.0).unwrap();
        assert_eq!(mid.bounds().unwrap().min[0], 1.0);
        let p = mid
            .field(Association::Point, "p")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert!(p.iter().all(|v| *v == 5.0));
        // Clamping: outside the range returns the end step.
        assert_eq!(series.sample(99.0).unwrap().bounds().unwrap().min[0], 2.0);
    }

    #[test]
    fn rectilinear_curvilinear_and_polygon_validate() {
        let rect =
            DataSet::rectilinear_grid("r", &[0.0, 0.5, 2.0], &[0.0, 1.0], &[0.0, 1.0]).unwrap();
        assert_eq!(rect.cell_count(), 2);
        assert_eq!(rect.validate(), Ok(()));
        let positions: Vec<[f64; 3]> = rect.positions.iter().copied().collect();
        let curved = DataSet::curvilinear_grid("c", [3, 2, 2], positions).unwrap();
        assert_eq!(curved.validate(), Ok(()));
        let poly = DataSet::new(
            "p",
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.5, 0.5, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            crate::dataset::Cells::from_cells(&[(
                crate::dataset::CellType::Polygon,
                &[0u32, 1, 2, 3, 4][..],
            )]),
        );
        assert_eq!(poly.validate(), Ok(()));
        assert_eq!(crate::boundary_triangles(&poly).len(), 3);
    }
}
