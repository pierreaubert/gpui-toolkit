//! Flow diagnostics: velocity gradient, vorticity, Q-criterion, divergence.
//!
//! Gradients are computed per tetrahedron (constant over the element from
//! its shape functions) and averaged to points weighted by element volume.
//! Hexahedra, wedges, and pyramids are tetrahedralized consistently with
//! the isosurface filter, so all diagnostics share one decomposition.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, DataSet, Field, FieldData};

use gpui_mesh::tetrahedra as tetrahedralize;

/// Solves a 3x3 system by Cramer's rule; `None` for a singular matrix.
fn solve3(m: [[f64; 3]; 3], rhs: [f64; 3]) -> Option<[f64; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-300 {
        return None;
    }
    let replace = |col: usize| {
        let mut mm = m;
        for row in 0..3 {
            mm[row][col] = rhs[row];
        }
        (mm[0][0] * (mm[1][1] * mm[2][2] - mm[1][2] * mm[2][1])
            - mm[0][1] * (mm[1][0] * mm[2][2] - mm[1][2] * mm[2][0])
            + mm[0][2] * (mm[1][0] * mm[2][1] - mm[1][1] * mm[2][0]))
            / det
    };
    Some([replace(0), replace(1), replace(2)])
}

/// Per-point velocity Jacobian `J[component][axis]`, volume-weighted.
///
/// Returns `(jacobian, volume)` per point; volume is the accumulated
/// element weight and is zero for points outside every volume cell.
fn point_jacobian(dataset: &DataSet, vectors: &[[f64; 3]]) -> Vec<([[f64; 3]; 3], f64)> {
    // Gather every tetrahedron first; per-tet gradients are computed in
    // parallel, then accumulated sequentially in cell order so results
    // are bit-identical to the sequential version.
    let mut tets: Vec<[u32; 4]> = Vec::new();
    for cell in 0..dataset.cell_count() {
        let kind = dataset.cells.types[cell];
        if !kind.is_volume() {
            continue;
        }
        let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
        tets.extend(tetrahedralize(kind, nodes));
    }
    let gradients = crate::parallel::parallel_map(tets.len(), |index| {
        let tet = tets[index];
        tet_jacobian(dataset, vectors, tet)
    });
    let mut acc = vec![([[0.0; 3]; 3], 0.0f64); dataset.point_count()];
    for (tet, gradient) in tets.iter().zip(gradients.iter()) {
        let Some((jac, volume)) = gradient else {
            continue;
        };
        for &node in tet {
            let entry = &mut acc[node as usize];
            for (component, row) in jac.iter().enumerate() {
                for (axis, value) in row.iter().enumerate() {
                    entry.0[component][axis] += value * volume;
                }
            }
            entry.1 += volume;
        }
    }
    for (jac, volume) in &mut acc {
        if *volume > 0.0 {
            for row in jac.iter_mut() {
                for value in row.iter_mut() {
                    *value /= *volume;
                }
            }
        }
    }
    acc
}

/// Gradient and volume of one tetrahedron; `None` when degenerate.
fn tet_jacobian(
    dataset: &DataSet,
    vectors: &[[f64; 3]],
    tet: [u32; 4],
) -> Option<([[f64; 3]; 3], f64)> {
    let p = [
        dataset.positions[tet[0] as usize],
        dataset.positions[tet[1] as usize],
        dataset.positions[tet[2] as usize],
        dataset.positions[tet[3] as usize],
    ];
    // Edge matrix columns are p_i - p_0; its determinant is 6*volume.
    let m = [
        [p[1][0] - p[0][0], p[2][0] - p[0][0], p[3][0] - p[0][0]],
        [p[1][1] - p[0][1], p[2][1] - p[0][1], p[3][1] - p[0][1]],
        [p[1][2] - p[0][2], p[2][2] - p[0][2], p[3][2] - p[0][2]],
    ];
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let volume = det.abs() / 6.0;
    if volume < 1e-300 {
        return None;
    }
    let mt = [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ];
    let mut jac = [[0.0; 3]; 3];
    for component in 0..3 {
        let v0 = vectors[tet[0] as usize][component];
        let dv = [
            vectors[tet[1] as usize][component] - v0,
            vectors[tet[2] as usize][component] - v0,
            vectors[tet[3] as usize][component] - v0,
        ];
        jac[component] = solve3(mt, dv)?;
    }
    Some((jac, volume))
}

fn velocity_field<'a>(
    dataset: &'a DataSet,
    field_name: &str,
) -> Result<&'a [[f64; 3]], TransformError> {
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    field
        .as_vector()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "flow diagnostics require a 3-vector field".into(),
        })
}

fn append(dataset: &DataSet, field: Field) -> DataSet {
    let mut result = dataset.clone();
    result.point_data.retain(|f| f.name != field.name);
    result.point_data.push(field);
    result
}

/// Appends vorticity `curl(v)` as a vector point field.
///
/// # Errors
///
/// Returns an error when the field is missing, not a vector field, or the
/// dataset holds no volume cells.
pub fn vorticity(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let vectors = velocity_field(dataset, field_name)?;
    let jacobian = point_jacobian(dataset, vectors);
    let values: Vec<[f64; 3]> = jacobian
        .iter()
        .map(|(j, _)| [j[2][1] - j[1][2], j[0][2] - j[2][0], j[1][0] - j[0][1]])
        .collect();
    let mut field = Field::vector(out_name.to_string(), values);
    field.label = "Vorticity".into();
    field.unit = Some("1/s".into());
    Ok(append(dataset, field))
}

/// Appends divergence `tr(J)` as a scalar point field.
///
/// # Errors
///
/// Returns an error when the field is missing or not a vector field.
pub fn divergence(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let vectors = velocity_field(dataset, field_name)?;
    let jacobian = point_jacobian(dataset, vectors);
    let values: Vec<f64> = jacobian
        .iter()
        .map(|(j, _)| j[0][0] + j[1][1] + j[2][2])
        .collect();
    let mut field = Field::scalar(out_name.to_string(), values);
    field.label = "Divergence".into();
    field.unit = Some("1/s".into());
    Ok(append(dataset, field))
}

/// Appends the Q-criterion `0.5*(||Omega||^2 - ||S||^2)` as a scalar field.
///
/// Positive Q marks rotation-dominated regions (vortex cores), which is
/// the standard field for Navier-Stokes vortex movies.
///
/// # Errors
///
/// Returns an error when the field is missing or not a vector field.
pub fn q_criterion(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let vectors = velocity_field(dataset, field_name)?;
    let jacobian = point_jacobian(dataset, vectors);
    let values: Vec<f64> = jacobian
        .iter()
        .map(|(j, _)| {
            let mut omega2 = 0.0;
            let mut strain2 = 0.0;
            for (a, row) in j.iter().enumerate() {
                for (b, &jab) in row.iter().enumerate() {
                    let omega = 0.5 * (jab - j[b][a]);
                    let strain = f64::midpoint(jab, j[b][a]);
                    omega2 += omega * omega;
                    strain2 += strain * strain;
                }
            }
            0.5 * (omega2 - strain2)
        })
        .collect();
    let mut field = Field::scalar(out_name.to_string(), values);
    field.label = "Q-criterion".into();
    field.unit = Some("1/s^2".into());
    Ok(append(dataset, field))
}

/// Appends the lambda-2 vortex criterion as a scalar field.
///
/// Lambda-2 is the middle eigenvalue of `S^2 + Omega^2` (Jeong & Hussain
/// 1995). Vortex cores are the connected regions where it is negative, so
/// extracting a core is `isosurface(lambda2, 0)` — or a small negative
/// value to shrink noise. Unlike Q it rejects pure shear layers.
///
/// # Errors
///
/// Returns an error when the field is missing or not a vector field.
pub fn lambda2(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let vectors = velocity_field(dataset, field_name)?;
    let jacobian = point_jacobian(dataset, vectors);
    let values: Vec<f64> = jacobian
        .iter()
        .map(|(j, _)| {
            // M = S^2 + Omega^2 is symmetric: build it directly.
            let s = [
                [
                    j[0][0],
                    f64::midpoint(j[0][1], j[1][0]),
                    f64::midpoint(j[0][2], j[2][0]),
                ],
                [
                    f64::midpoint(j[1][0], j[0][1]),
                    j[1][1],
                    f64::midpoint(j[1][2], j[2][1]),
                ],
                [
                    f64::midpoint(j[2][0], j[0][2]),
                    f64::midpoint(j[2][1], j[1][2]),
                    j[2][2],
                ],
            ];
            let o = [
                [0.0, 0.5 * (j[0][1] - j[1][0]), 0.5 * (j[0][2] - j[2][0])],
                [0.5 * (j[1][0] - j[0][1]), 0.0, 0.5 * (j[1][2] - j[2][1])],
                [0.5 * (j[2][0] - j[0][2]), 0.5 * (j[2][1] - j[1][2]), 0.0],
            ];
            let mut m = [[0.0; 3]; 3];
            for (a, row) in m.iter_mut().enumerate() {
                for (b, value) in row.iter_mut().enumerate() {
                    *value = (0..3).map(|k| s[a][k] * s[k][b] + o[a][k] * o[k][b]).sum();
                }
            }
            let (mut eigenvalues, _) = crate::eigen::symmetric_eigen(m);
            eigenvalues.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            eigenvalues[1]
        })
        .collect();
    let mut field = Field::scalar(out_name.to_string(), values);
    field.label = "Lambda-2".into();
    field.unit = Some("1/s^2".into());
    Ok(append(dataset, field))
}

/// Appends swirling strength (lambda-ci) as a scalar field.
///
/// The imaginary part of the Jacobian's complex eigenvalue pair: the local
/// swirling rate, zero in strain- and shear-dominated regions. Vortex
/// cores are high-value regions; it complements [`lambda2`].
///
/// # Errors
///
/// Returns an error when the field is missing or not a vector field.
pub fn swirling_strength(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let vectors = velocity_field(dataset, field_name)?;
    let jacobian = point_jacobian(dataset, vectors);
    let values: Vec<f64> = jacobian
        .iter()
        .map(|(j, _)| crate::eigen::swirling_strength(*j))
        .collect();
    let mut field = Field::scalar(out_name.to_string(), values);
    field.label = "Swirling strength".into();
    field.unit = Some("1/s".into());
    Ok(append(dataset, field))
}

/// Appends vorticity magnitude as a scalar field (convenience composition).
///
/// # Errors
///
/// Returns an error when the field is missing or not a vector field.
pub fn vorticity_magnitude(
    dataset: &DataSet,
    field_name: &str,
    out_name: &str,
) -> Result<DataSet, TransformError> {
    let with_vorticity = vorticity(dataset, field_name, "__vorticity_tmp")?;
    let field = with_vorticity
        .field(Association::Point, "__vorticity_tmp")
        .ok_or_else(|| TransformError::FieldNotFound("__vorticity_tmp".into()))?;
    let FieldData::Vector(vectors) = &field.data else {
        return Err(TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "expected a vector field".into(),
        });
    };
    let values: Vec<f64> = vectors
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        .collect();
    let mut result = with_vorticity;
    result
        .point_data
        .retain(|f| f.name.as_ref() != "__vorticity_tmp");
    let mut field = Field::scalar(out_name.to_string(), values);
    field.label = "Vorticity magnitude".into();
    field.unit = Some("1/s".into());
    result.point_data.push(field);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Solid-body rotation v = Omega x r with Omega = (0,0,2):
    /// vorticity is exactly (0,0,4), divergence 0, Q = |Omega|^2 = 4...
    /// For v = (-2y, 2x, 0): J = [[0,-2,0],[2,0,0],[0,0,0]], Omega tensor
    /// equals J antisymmetric part, ||Omega||^2 = 8, S = 0, so Q = 4.
    fn rotating() -> DataSet {
        let mut ds = DataSet::structured_grid("g", [4, 4, 4], [-1.0; 3], [0.5; 3]).unwrap();
        let velocity: Vec<[f64; 3]> = ds
            .positions
            .iter()
            .map(|p| [-2.0 * p[1], 2.0 * p[0], 0.0])
            .collect();
        ds.point_data.push(Field::vector("velocity", velocity));
        ds
    }

    #[test]
    fn lambda2_and_swirling_mark_rotation_not_strain() {
        let ds = rotating();
        let centre = ds
            .positions
            .iter()
            .position(|p| *p == [0.0, 0.0, 0.0])
            .unwrap();
        // Rotation omega = 2: lambda2 = -omega^2 = -4, lambda-ci = 2.
        let l2 = lambda2(&ds, "velocity", "l2").unwrap();
        let l2v = l2
            .field(Association::Point, "l2")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert!((l2v[centre] + 4.0).abs() < 1e-9, "got {}", l2v[centre]);
        let sw = swirling_strength(&ds, "velocity", "sw").unwrap();
        let swv = sw
            .field(Association::Point, "sw")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert!((swv[centre] - 2.0).abs() < 1e-9, "got {}", swv[centre]);

        // Pure strain v = (x, -y, 0): no vortex by either criterion.
        let mut strained = DataSet::structured_grid("g", [4, 4, 4], [-1.0; 3], [0.5; 3]).unwrap();
        let velocity: Vec<[f64; 3]> = strained
            .positions
            .iter()
            .map(|p| [p[0], -p[1], 0.0])
            .collect();
        strained
            .point_data
            .push(Field::vector("velocity", velocity));
        let l2 = lambda2(&strained, "velocity", "l2").unwrap();
        let l2v = l2
            .field(Association::Point, "l2")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert!(l2v.iter().all(|v| *v > 0.0), "strain must not be a core");
        let sw = swirling_strength(&strained, "velocity", "sw").unwrap();
        let swv = sw
            .field(Association::Point, "sw")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert!(swv.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn solid_rotation_has_exact_vorticity_and_q() {
        let ds = rotating();
        let out = vorticity(&ds, "velocity", "vorticity").unwrap();
        let values = out
            .field(Association::Point, "vorticity")
            .unwrap()
            .as_vector()
            .unwrap();
        // Interior points average symmetric tets; check the centre point.
        let centre = ds
            .positions
            .iter()
            .position(|p| *p == [0.0, 0.0, 0.0])
            .unwrap();
        assert!(
            (values[centre][2] - 4.0).abs() < 1e-9,
            "got {:?}",
            values[centre]
        );
        assert!(
            values
                .iter()
                .all(|v| v[2] > 3.5 && v[0].abs() < 0.6 && v[1].abs() < 0.6)
        );

        let q = q_criterion(&ds, "velocity", "q").unwrap();
        let qv = q
            .field(Association::Point, "q")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert!((qv[centre] - 4.0).abs() < 1e-9, "got {}", qv[centre]);

        let div = divergence(&ds, "velocity", "div").unwrap();
        let dv = div
            .field(Association::Point, "div")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert!(dv.iter().all(|v| v.abs() < 1e-9));
    }

    #[test]
    fn pure_strain_has_negative_q() {
        // v = (2x, -2y, 0): strain only, Q = -||S||^2 /... = -4.
        let mut ds = DataSet::structured_grid("g", [4, 4, 4], [-1.0; 3], [0.5; 3]).unwrap();
        let velocity: Vec<[f64; 3]> = ds
            .positions
            .iter()
            .map(|p| [2.0 * p[0], -2.0 * p[1], 0.0])
            .collect();
        ds.point_data.push(Field::vector("velocity", velocity));
        let q = q_criterion(&ds, "velocity", "q").unwrap();
        let qv = q
            .field(Association::Point, "q")
            .unwrap()
            .as_scalar()
            .unwrap();
        let centre = ds
            .positions
            .iter()
            .position(|p| *p == [0.0, 0.0, 0.0])
            .unwrap();
        assert!((qv[centre] + 4.0).abs() < 1e-9, "got {}", qv[centre]);
    }
}
