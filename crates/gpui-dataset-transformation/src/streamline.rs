//! Streamline tracing through a vector point field.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, Cells, DataSet};

pub(crate) fn validate_seeds(seeds: &[[f64; 3]]) -> Result<(), TransformError> {
    if seeds.iter().flatten().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(TransformError::Degenerate(
            "trace seeds must be finite".into(),
        ))
    }
}

pub(crate) fn unit_direction(vector: [f64; 3]) -> Option<[f64; 3]> {
    let norm = crate::calculator::magnitude(vector);
    if norm.is_normal() {
        return Some(vector.map(|v| v / norm));
    }
    let scale = vector.iter().map(|v| v.abs()).fold(0.0, f64::max);
    if !scale.is_finite() || scale == 0.0 || vector.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let scaled = vector.map(|v| v / scale);
    let norm = crate::calculator::magnitude(scaled);
    Some(scaled.map(|v| v / norm))
}

pub(crate) struct Tracer<'a> {
    locator: crate::stencil::StencilLocator<'a>,
    vectors: &'a [[f64; 3]],
}

impl<'a> Tracer<'a> {
    pub(crate) fn new(
        dataset: &'a DataSet,
        vectors: &'a [[f64; 3]],
        valid: Option<&'a [bool]>,
    ) -> Self {
        Self {
            locator: crate::stencil::StencilLocator::new(dataset, valid),
            vectors,
        }
    }
    pub(crate) fn has_volume(&self) -> bool {
        self.locator.has_volume()
    }
    pub(crate) fn velocity(&self, p: [f64; 3]) -> Option<[f64; 3]> {
        let (nodes, weights) = self.locator.weights(p)?;
        crate::stencil::sample_values(self.vectors, nodes, weights)
    }
}

/// Traces streamlines from seed points through a vector point field.
///
/// Integration is midpoint (RK2) with fixed arc-length-style steps of
/// `step` in dataset units, at most `max_steps` per direction. Tracing
/// stops when a point leaves the volume or the velocity vanishes.
/// Every contributing tetrahedron requires four valid samples. Missing regions
/// stop paths; wholly missing fields produce an empty dataset.
///
/// # Errors
///
/// Returns an error for invalid input/seeds, a nonfinite/non-positive step,
/// missing/non-vector fields, no volume cells or excessive output point indices.
pub fn streamlines(
    dataset: &DataSet,
    field_name: &str,
    seeds: &[[f64; 3]],
    step: f64,
    max_steps: usize,
) -> Result<DataSet, TransformError> {
    if !step.is_finite() || step <= 0.0 {
        return Err(TransformError::Degenerate(
            "streamline step must be finite and positive".into(),
        ));
    }
    validate_seeds(seeds)?;
    dataset.validate()?;
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let vectors = field
        .as_vector()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "streamlines require a 3-vector field".into(),
        })?;
    let tracer = Tracer::new(dataset, vectors, field.valid.as_deref());
    if !tracer.has_volume() {
        return Err(TransformError::Degenerate(
            "dataset holds no volume cells".into(),
        ));
    }
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut cells = Vec::new();
    for seed in seeds {
        for direction in [1.0, -1.0] {
            let mut line: Vec<[f64; 3]> = vec![*seed];
            let mut p = *seed;
            for _ in 0..max_steps {
                let Some(v) = tracer.velocity(p) else { break };
                let Some(dir) = unit_direction(v) else {
                    break;
                };
                let mid = [
                    p[0] + 0.5 * step * direction * dir[0],
                    p[1] + 0.5 * step * direction * dir[1],
                    p[2] + 0.5 * step * direction * dir[2],
                ];
                let Some(vm) = tracer.velocity(mid) else {
                    break;
                };
                let Some(dir) = unit_direction(vm) else {
                    break;
                };
                let next = [
                    p[0] + step * direction * dir[0],
                    p[1] + step * direction * dir[1],
                    p[2] + step * direction * dir[2],
                ];
                // Stop at the boundary instead of emitting an outside point.
                if next == p || tracer.velocity(next).is_none() {
                    break;
                }
                p = next;
                line.push(p);
            }
            if direction < 0.0 {
                line.reverse();
            }
            if line.len() < 2 {
                continue;
            }
            let base = crate::geometry::output_base(positions.len(), line.len())?;
            positions.extend(line.iter().copied());
            for i in 0..line.len() - 1 {
                cells.push([base + i as u32, base + i as u32 + 1]);
            }
        }
    }
    Ok(DataSet::new(
        format!("{}-streamlines", dataset.id),
        positions,
        Cells::lines(&cells),
    ))
}

/// Traces streamlines with adaptive step-doubling RK2.
///
/// Each step is taken once at `h` and twice at `h/2`; the distance
/// between the results estimates the local error. Steps grow or shrink
/// within `[min_step, max_step]` to keep the error near `tolerance`.
/// This is the tracer to use for fields with strong scale separation
/// (fan tips vs. far field), where fixed steps either waste points or
/// cut corners.
/// Missing stencils stop paths, using the same validity rule as [`streamlines`].
///
/// # Errors
///
/// Returns an error when the field is missing, not a vector field, the
/// dataset holds no volume cells, input/seeds are invalid, step bounds are
/// nonfinite/invalid, or output point indices exceed the model representation.
#[allow(clippy::too_many_arguments)]
pub fn streamlines_adaptive(
    dataset: &DataSet,
    field_name: &str,
    seeds: &[[f64; 3]],
    initial_step: f64,
    min_step: f64,
    max_step: f64,
    tolerance: f64,
    max_steps: usize,
) -> Result<DataSet, TransformError> {
    if ![initial_step, min_step, max_step, tolerance]
        .iter()
        .all(|v| v.is_finite())
        || initial_step <= 0.0
        || min_step <= 0.0
        || max_step < min_step
        || tolerance <= 0.0
    {
        return Err(TransformError::Degenerate(
            "adaptive tracing requires 0 < min_step <= max_step and tolerance > 0".into(),
        ));
    }
    validate_seeds(seeds)?;
    dataset.validate()?;
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let vectors = field
        .as_vector()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "streamlines require a 3-vector field".into(),
        })?;
    let tracer = Tracer::new(dataset, vectors, field.valid.as_deref());
    if !tracer.has_volume() {
        return Err(TransformError::Degenerate(
            "dataset holds no volume cells".into(),
        ));
    }
    // One normalised RK2 step of arc length h in `direction`.
    let step_once = |p: [f64; 3], direction: f64, h: f64| -> Option<[f64; 3]> {
        let v = tracer.velocity(p)?;
        let dir = unit_direction(v)?;
        let mid = [
            p[0] + 0.5 * h * direction * dir[0],
            p[1] + 0.5 * h * direction * dir[1],
            p[2] + 0.5 * h * direction * dir[2],
        ];
        let vm = tracer.velocity(mid)?;
        let dir = unit_direction(vm)?;
        Some([
            p[0] + h * direction * dir[0],
            p[1] + h * direction * dir[1],
            p[2] + h * direction * dir[2],
        ])
    };
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut cells = Vec::new();
    for seed in seeds {
        for direction in [1.0, -1.0] {
            let mut line: Vec<[f64; 3]> = vec![*seed];
            let mut p = *seed;
            let mut h = initial_step.clamp(min_step, max_step);
            for _ in 0..max_steps {
                // Step doubling: full step vs two half steps.
                let (_full, half) = loop {
                    let Some(full) = step_once(p, direction, h) else {
                        break (None, None);
                    };
                    let _ = &full;
                    let Some(mid) = step_once(p, direction, h * 0.5) else {
                        break (None, None);
                    };
                    let Some(half) = step_once(mid, direction, h * 0.5) else {
                        break (None, None);
                    };
                    let error = ((full[0] - half[0]).powi(2)
                        + (full[1] - half[1]).powi(2)
                        + (full[2] - half[2]).powi(2))
                    .sqrt();
                    if error <= tolerance || h <= min_step {
                        break (Some(full), Some(half));
                    }
                    h = (h * 0.5).max(min_step);
                };
                let Some(next) = half else { break };
                if next == p || tracer.velocity(next).is_none() {
                    break;
                }
                p = next;
                line.push(p);
                // Grow cautiously after an accepted step.
                h = (h * 1.3).min(max_step);
            }
            if direction < 0.0 {
                line.reverse();
            }
            if line.len() < 2 {
                continue;
            }
            let base = crate::geometry::output_base(positions.len(), line.len())?;
            positions.extend(line.iter().copied());
            for i in 0..line.len() - 1 {
                cells.push([base + i as u32, base + i as u32 + 1]);
            }
        }
    }
    Ok(DataSet::new(
        format!("{}-streamlines-adaptive", dataset.id),
        positions,
        Cells::lines(&cells),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Field;

    #[test]
    fn traces_through_uniform_flow() {
        let mut ds = DataSet::structured_grid("g", [4, 2, 2], [0.0; 3], [1.0; 3]).unwrap();
        let n = ds.point_count();
        ds.point_data
            .push(Field::vector("velocity", vec![[1.0, 0.0, 0.0]; n]));
        let out = streamlines(&ds, "velocity", &[[0.1, 0.5, 0.5]], 0.25, 100).unwrap();
        assert!(out.cell_count() > 4);
        assert_eq!(out.validate(), Ok(()));
        assert!(
            out.positions
                .iter()
                .all(|p| p[0] >= 0.0 && p[0] <= 3.0 + 1e-9)
        );
    }
}
