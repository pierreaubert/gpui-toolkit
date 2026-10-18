//! Streamline tracing through a vector point field.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, CellType, Cells, DataSet};

pub(crate) use gpui_mesh::tetrahedra as tetrahedralize;

/// Barycentric weights of `p` in a tetrahedron, when inside (within tolerance).
pub(crate) fn barycentric(tet: [[f64; 3]; 4], p: [f64; 3]) -> Option<[f64; 4]> {
    let (a, b, c, d) = (tet[0], tet[1], tet[2], tet[3]);
    let m = [
        [b[0] - a[0], c[0] - a[0], d[0] - a[0]],
        [b[1] - a[1], c[1] - a[1], d[1] - a[1]],
        [b[2] - a[2], c[2] - a[2], d[2] - a[2]],
    ];
    let rhs = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-300 {
        return None;
    }
    // Cramer's rule for the 3x3 system.
    let solve = |col: usize| -> f64 {
        let mut mm = m;
        for row in 0..3 {
            mm[row][col] = rhs[row];
        }
        (mm[0][0] * (mm[1][1] * mm[2][2] - mm[1][2] * mm[2][1])
            - mm[0][1] * (mm[1][0] * mm[2][2] - mm[1][2] * mm[2][0])
            + mm[0][2] * (mm[1][0] * mm[2][1] - mm[1][1] * mm[2][0]))
            / det
    };
    let w = [solve(0), solve(1), solve(2)];
    let weights = [1.0 - w[0] - w[1] - w[2], w[0], w[1], w[2]];
    weights.iter().all(|&x| x >= -1e-9).then_some(weights)
}

pub(crate) struct Tracer<'a> {
    dataset: &'a DataSet,
    vectors: &'a [[f64; 3]],
    pub(crate) tets: Vec<[u32; 4]>,
}

impl<'a> Tracer<'a> {
    /// Builds a tracer over every volume cell of `dataset`.
    pub(crate) fn new(dataset: &'a DataSet, vectors: &'a [[f64; 3]]) -> Self {
        let mut tets = Vec::new();
        for cell in 0..dataset.cell_count() {
            let kind = dataset.cells.types[cell];
            if kind.is_volume() {
                let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
                tets.extend(tetrahedralize(kind, nodes));
            }
        }
        Self {
            dataset,
            vectors,
            tets,
        }
    }

    pub(crate) fn velocity(&self, p: [f64; 3]) -> Option<[f64; 3]> {
        for tet in &self.tets {
            let corners = [
                self.dataset.positions[tet[0] as usize],
                self.dataset.positions[tet[1] as usize],
                self.dataset.positions[tet[2] as usize],
                self.dataset.positions[tet[3] as usize],
            ];
            if let Some(w) = barycentric(corners, p) {
                let mut v = [0.0; 3];
                for (i, weight) in w.iter().enumerate() {
                    let vec = self.vectors[tet[i] as usize];
                    v[0] += weight * vec[0];
                    v[1] += weight * vec[1];
                    v[2] += weight * vec[2];
                }
                return Some(v);
            }
        }
        None
    }
}

/// Traces streamlines from seed points through a vector point field.
///
/// Integration is midpoint (RK2) with fixed arc-length-style steps of
/// `step` in dataset units, at most `max_steps` per direction. Tracing
/// stops when a point leaves the volume or the velocity vanishes.
///
/// # Errors
///
/// Returns an error when the field is missing, not a vector field, or the
/// dataset holds no volume cells.
pub fn streamlines(
    dataset: &DataSet,
    field_name: &str,
    seeds: &[[f64; 3]],
    step: f64,
    max_steps: usize,
) -> Result<DataSet, TransformError> {
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
    let tracer = Tracer::new(dataset, vectors);
    if tracer.tets.is_empty() {
        return Err(TransformError::Degenerate(
            "dataset holds no volume cells".into(),
        ));
    }
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    for seed in seeds {
        for direction in [1.0, -1.0] {
            let mut line: Vec<[f64; 3]> = vec![*seed];
            let mut p = *seed;
            for _ in 0..max_steps {
                let Some(v) = tracer.velocity(p) else { break };
                let speed = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                if speed < 1e-300 {
                    break;
                }
                let dir = [v[0] / speed, v[1] / speed, v[2] / speed];
                let mid = [
                    p[0] + 0.5 * step * direction * dir[0],
                    p[1] + 0.5 * step * direction * dir[1],
                    p[2] + 0.5 * step * direction * dir[2],
                ];
                let Some(vm) = tracer.velocity(mid) else {
                    break;
                };
                let sm = (vm[0] * vm[0] + vm[1] * vm[1] + vm[2] * vm[2]).sqrt();
                if sm < 1e-300 {
                    break;
                }
                let next = [
                    p[0] + step * direction * vm[0] / sm,
                    p[1] + step * direction * vm[1] / sm,
                    p[2] + step * direction * vm[2] / sm,
                ];
                // Stop at the boundary instead of emitting an outside point.
                if tracer.velocity(next).is_none() {
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
            let base = positions.len() as u32;
            positions.extend(line.iter().copied());
            for i in 0..line.len() - 1 {
                cells.push((CellType::Line, vec![base + i as u32, base + i as u32 + 1]));
            }
        }
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    Ok(DataSet::new(
        format!("{}-streamlines", dataset.id),
        positions,
        Cells::from_cells(&refs),
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
///
/// # Errors
///
/// Returns an error when the field is missing, not a vector field, the
/// dataset holds no volume cells, or the step bounds are invalid.
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
    if min_step <= 0.0 || max_step < min_step || tolerance <= 0.0 {
        return Err(TransformError::Degenerate(
            "adaptive tracing requires 0 < min_step <= max_step and tolerance > 0".into(),
        ));
    }
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
    let tracer = Tracer::new(dataset, vectors);
    if tracer.tets.is_empty() {
        return Err(TransformError::Degenerate(
            "dataset holds no volume cells".into(),
        ));
    }
    // One normalised RK2 step of arc length h in `direction`.
    let step_once = |p: [f64; 3], direction: f64, h: f64| -> Option<[f64; 3]> {
        let v = tracer.velocity(p)?;
        let speed = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if speed < 1e-300 {
            return None;
        }
        let mid = [
            p[0] + 0.5 * h * direction * v[0] / speed,
            p[1] + 0.5 * h * direction * v[1] / speed,
            p[2] + 0.5 * h * direction * v[2] / speed,
        ];
        let vm = tracer.velocity(mid)?;
        let sm = (vm[0] * vm[0] + vm[1] * vm[1] + vm[2] * vm[2]).sqrt();
        if sm < 1e-300 {
            return None;
        }
        Some([
            p[0] + h * direction * vm[0] / sm,
            p[1] + h * direction * vm[1] / sm,
            p[2] + h * direction * vm[2] / sm,
        ])
    };
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
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
                if tracer.velocity(next).is_none() {
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
            let base = positions.len() as u32;
            positions.extend(line.iter().copied());
            for i in 0..line.len() - 1 {
                cells.push((CellType::Line, vec![base + i as u32, base + i as u32 + 1]));
            }
        }
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    Ok(DataSet::new(
        format!("{}-streamlines-adaptive", dataset.id),
        positions,
        Cells::from_cells(&refs),
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
