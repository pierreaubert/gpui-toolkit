//! Time-varying datasets with shared or deforming topology.

// Rust guideline compliant 2026-02-21

use crate::dataset::{DataSet, MeshError};
use crate::field::{Association, Field, FieldData};

/// A sequence of dataset states at increasing times.
///
/// States may deform (positions change between steps, as in Navier-Stokes
/// or crash simulations) but must share cell storage, which lets a renderer
/// upload connectivity once and stream only positions and fields per step.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSeries {
    /// State at each time step.
    pub steps: Vec<DataSet>,
    /// Physical time of each step, strictly increasing.
    pub times: Vec<f64>,
}

impl TimeSeries {
    /// Creates a time series.
    ///
    /// # Errors
    ///
    /// Returns an error for empty/inconsistent times, invalid datasets or
    /// inconsistent point counts/topology. See [`Self::validate`].
    pub fn new(steps: Vec<DataSet>, times: Vec<f64>) -> Result<Self, MeshError> {
        let series = Self { steps, times };
        series.validate()?;
        Ok(series)
    }

    /// Validates times, each dataset, point counts and shared topology.
    ///
    /// Call this again after directly mutating public steps or timestamps.
    ///
    /// # Errors
    ///
    /// Returns an error for empty/inconsistent times, malformed datasets or
    /// inconsistent point counts/topology.
    pub fn validate(&self) -> Result<(), MeshError> {
        let Self { steps, times } = self;
        if steps.len() != times.len() || steps.is_empty() {
            return Err(MeshError::InvalidDims {
                dims: [steps.len(), times.len(), 0],
            });
        }
        if times.windows(2).any(|w| w[1] <= w[0]) || times.iter().any(|t| !t.is_finite()) {
            return Err(MeshError::InvalidDims {
                dims: [times.len(), 0, 0],
            });
        }
        if let Some(first) = steps.first() {
            for step in steps {
                step.validate()?;
                if step.point_count() != first.point_count() {
                    return Err(MeshError::InvalidTimeSeries {
                        reason: "time steps have different point counts".into(),
                    });
                }
                if step.cells.types != first.cells.types
                    || step.cells.offsets != first.cells.offsets
                    || step.cells.connectivity != first.cells.connectivity
                {
                    return Err(MeshError::InvalidOffsets);
                }
            }
        }
        Ok(())
    }

    /// Number of steps.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the series has no steps (unreachable through `new`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Step index and blend factor for a physical time, clamped to the range.
    #[must_use]
    pub fn locate(&self, time: f64) -> (usize, usize, f64) {
        if time <= self.times[0] {
            return (0, 0, 0.0);
        }
        let last = self.times.len() - 1;
        if time >= self.times[last] {
            return (last, last, 0.0);
        }
        let upper = self.times.partition_point(|t| *t <= time);
        let lo = upper - 1;
        let span = self.times[upper] - self.times[lo];
        let fraction = if span.is_finite() {
            (time - self.times[lo]) / span
        } else {
            (time * 0.5 - self.times[lo] * 0.5) / (self.times[upper] * 0.5 - self.times[lo] * 0.5)
        };
        (lo, upper, fraction)
    }
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
        assert_eq!(series.locate(-1.0), (0, 0, 0.0));
        assert_eq!(series.locate(1.0), (0, 1, 0.5));
        assert_eq!(series.locate(9.0), (1, 1, 0.0));
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

impl TimeSeries {
    /// Samples the series at `time`, interpolating between steps.
    ///
    /// Positions and every numeric field (scalar, vector, tensor,
    /// complex) are linearly interpolated between the bracketing steps;
    /// fields present in only one step are taken from the nearer step.
    /// Topology comes from the earlier step. Times outside the range
    /// clamp to the end steps (a clone of that step, no extrapolation).
    ///
    /// # Errors
    ///
    /// Returns an error when the bracketing steps have different point
    /// counts (shared topology is violated).
    pub fn sample(&self, time: f64) -> Result<DataSet, MeshError> {
        let (lo, hi, frac) = self.locate(time);
        let (a, b) = (&self.steps[lo], &self.steps[hi]);
        if lo == hi || frac == 0.0 {
            return Ok(a.clone());
        }
        if a.point_count() != b.point_count() {
            return Err(MeshError::InvalidTimeSeries {
                reason: "time steps have different point counts".into(),
            });
        }
        let lerp = |x: f64, y: f64| x + (y - x) * frac;
        let positions: Vec<[f64; 3]> = a
            .positions
            .iter()
            .zip(b.positions.iter())
            .map(|(p, q)| [lerp(p[0], q[0]), lerp(p[1], q[1]), lerp(p[2], q[2])])
            .collect();
        let mut out = DataSet::new(a.id.clone(), positions, a.cells.clone());
        for association in [Association::Point, Association::Cell] {
            let (fa, fb) = match association {
                Association::Point => (&a.point_data, &b.point_data),
                Association::Cell => (&a.cell_data, &b.cell_data),
            };
            let target = match association {
                Association::Point => &mut out.point_data,
                Association::Cell => &mut out.cell_data,
            };
            for field_a in fa {
                let Some(field_b) = fb.iter().find(|f| f.name == field_a.name) else {
                    target.push(field_a.clone());
                    continue;
                };
                let data = match (&field_a.data, &field_b.data) {
                    (FieldData::Scalar(x), FieldData::Scalar(y)) if x.len() == y.len() => {
                        FieldData::Scalar(
                            x.iter()
                                .zip(y.iter())
                                .map(|(u, v)| lerp(*u, *v))
                                .collect::<Vec<_>>()
                                .into(),
                        )
                    }
                    (FieldData::Vector(x), FieldData::Vector(y)) if x.len() == y.len() => {
                        FieldData::Vector(
                            x.iter()
                                .zip(y.iter())
                                .map(|(u, v)| {
                                    [lerp(u[0], v[0]), lerp(u[1], v[1]), lerp(u[2], v[2])]
                                })
                                .collect::<Vec<_>>()
                                .into(),
                        )
                    }
                    (FieldData::Tensor(x), FieldData::Tensor(y)) if x.len() == y.len() => {
                        FieldData::Tensor(
                            x.iter()
                                .zip(y.iter())
                                .map(|(u, v)| {
                                    let mut t = [0.0; 6];
                                    for (i, value) in t.iter_mut().enumerate() {
                                        *value = lerp(u[i], v[i]);
                                    }
                                    t
                                })
                                .collect::<Vec<_>>()
                                .into(),
                        )
                    }
                    (FieldData::Complex(x), FieldData::Complex(y)) if x.len() == y.len() => {
                        FieldData::Complex(
                            x.iter()
                                .zip(y.iter())
                                .map(|(u, v)| [lerp(u[0], v[0]), lerp(u[1], v[1])])
                                .collect::<Vec<_>>()
                                .into(),
                        )
                    }
                    _ => field_a.data.clone(),
                };
                target.push(Field {
                    name: field_a.name.clone(),
                    label: field_a.label.clone(),
                    unit: field_a.unit.clone(),
                    association,
                    data,
                    valid: field_a.valid.clone(),
                });
            }
            for field_b in fb {
                if !fa.iter().any(|f| f.name == field_b.name) {
                    target.push(field_b.clone());
                }
            }
        }
        Ok(out)
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
