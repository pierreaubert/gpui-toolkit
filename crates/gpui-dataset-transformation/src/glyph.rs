//! Vector glyphs: arrows placed at points, scaled by a vector field.

// Rust guideline compliant 2026-02-21

use crate::TransformError;
use gpui_mesh::{Association, CellType, Cells, DataSet, Field};

/// Glyph shape emitted per point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphKind {
    /// A line from the point along the vector, with a small head.
    Arrow,
    /// A line in both directions (for undirected orientation fields).
    Line,
}

/// Builds a glyph dataset for a vector point field.
///
/// Every `stride`-th point receives one glyph, scaled by `scale`. The
/// output holds only line cells, so it renders cheaply even for dense
/// fields and overlays directly on the source surface.
///
/// # Errors
///
/// Returns an error when the field is missing or not a vector field.
pub fn glyphs(
    dataset: &DataSet,
    field_name: &str,
    scale: f64,
    stride: usize,
    kind: GlyphKind,
) -> Result<DataSet, TransformError> {
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let vectors = field
        .as_vector()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "glyphs require a 3-vector field".into(),
        })?;
    let stride = stride.max(1);
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    let push = |p: [f64; 3], positions: &mut Vec<[f64; 3]>| -> u32 {
        let id = positions.len() as u32;
        positions.push(p);
        id
    };
    for (i, (origin, v)) in dataset.positions.iter().zip(vectors.iter()).enumerate() {
        if i % stride != 0 {
            continue;
        }
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len == 0.0 || !len.is_finite() {
            continue;
        }
        let dir = [v[0] / len, v[1] / len, v[2] / len];
        let tip = [
            origin[0] + scale * v[0],
            origin[1] + scale * v[1],
            origin[2] + scale * v[2],
        ];
        match kind {
            GlyphKind::Arrow => {
                let base = push(*origin, &mut positions);
                let tip_id = push(tip, &mut positions);
                cells.push((CellType::Line, vec![base, tip_id]));
                // Arrow head: two short segments at 150 degrees in an
                // arbitrary plane containing the direction.
                let side = perpendicular(dir);
                let head = scale * len * 0.25;
                for s in [1.0, -1.0] {
                    let back = [
                        tip[0] - dir[0] * head + s * side[0] * head * 0.5,
                        tip[1] - dir[1] * head + s * side[1] * head * 0.5,
                        tip[2] - dir[2] * head + s * side[2] * head * 0.5,
                    ];
                    let back_id = push(back, &mut positions);
                    cells.push((CellType::Line, vec![tip_id, back_id]));
                }
            }
            GlyphKind::Line => {
                let a = [
                    origin[0] - scale * v[0] * 0.5,
                    origin[1] - scale * v[1] * 0.5,
                    origin[2] - scale * v[2] * 0.5,
                ];
                let a_id = push(a, &mut positions);
                let b_id = push(tip, &mut positions);
                cells.push((CellType::Line, vec![a_id, b_id]));
            }
        }
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    Ok(DataSet::new(
        format!("{}-glyphs", dataset.id),
        positions,
        Cells::from_cells(&refs),
    ))
}

/// Radial segments around an arrow shaft and head. Six sides read clearly
/// at demo scale while keeping the triangle count low (24 per arrow).
const ARROW_SIDES: usize = 6;
/// Fraction of the total arrow length taken by the head.
const HEAD_FRACTION: f64 = 0.3;

/// Builds solid 3D arrows for a vector point field.
///
/// Each arrow is a closed triangle mesh: a six-sided shaft covering the
/// first 70% of the length and a six-sided cone head, so arrows shade and
/// clip like any other surface in `gpui-scene3d` (unlike the line arrows
/// from [`glyphs`]). Length is `scale * |v|`; shaft radius is
/// `radius_fraction * length`. A scalar `magnitude` point field is attached
/// so callers can colour arrows by speed. Zero and non-finite vectors are
/// skipped.
///
/// # Errors
///
/// Returns an error when the field is missing or not a vector field.
pub fn solid_arrows(
    dataset: &DataSet,
    field_name: &str,
    scale: f64,
    stride: usize,
    radius_fraction: f64,
) -> Result<DataSet, TransformError> {
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let vectors = field
        .as_vector()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "solid arrows require a 3-vector field".into(),
        })?;
    let stride = stride.max(1);
    // Per-glyph geometry in parallel, merged in point order.
    let glyphs = crate::parallel::parallel_map(dataset.point_count(), |i| {
        if i % stride != 0 {
            return None;
        }
        arrow_geometry(dataset.positions[i], vectors[i], scale, radius_fraction)
    });
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut magnitudes: Vec<f64> = Vec::new();
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    for glyph in glyphs.into_iter().flatten() {
        let (local_positions, triangles, speed) = glyph;
        let base = positions.len() as u32;
        magnitudes.extend(std::iter::repeat_n(speed, local_positions.len()));
        positions.extend(local_positions);
        for tri in triangles {
            cells.push((
                CellType::Triangle,
                vec![base + tri[0], base + tri[1], base + tri[2]],
            ));
        }
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    Ok(DataSet::new(
        format!("{}-arrows", dataset.id),
        positions,
        Cells::from_cells(&refs),
    )
    .with_field(Field::scalar("magnitude", magnitudes)))
}

/// Sphere resolution for tensor glyphs: 8 sectors x 4 stacks reads as a
/// smooth ellipsoid at demo scale for 48 triangles per glyph.
const TENSOR_SECTORS: usize = 8;
/// Stacks (latitude bands) of the tensor-glyph sphere.
const TENSOR_STACKS: usize = 4;

/// Builds ellipsoid glyphs for a symmetric-tensor point field.
///
/// Each glyph is the unit sphere deformed by the tensor's
/// eigendecomposition: principal axes are the eigenvectors, semi-axis
/// lengths are `scale * sqrt(|eigenvalue|)` (stress/strain tensors are
/// indefinite, so magnitudes are used; the sign is not encoded). A scalar
/// `magnitude` point field carries the largest absolute eigenvalue for
/// colouring. Non-finite tensors are skipped.
///
/// # Errors
///
/// Returns an error when the field is missing or not a tensor field.
pub fn tensor_glyphs(
    dataset: &DataSet,
    field_name: &str,
    scale: f64,
    stride: usize,
) -> Result<DataSet, TransformError> {
    let field = dataset
        .field(Association::Point, field_name)
        .ok_or_else(|| TransformError::FieldNotFound(field_name.to_string()))?;
    let tensors = field
        .as_tensor()
        .ok_or_else(|| TransformError::WrongFieldShape {
            field: field_name.to_string(),
            reason: "tensor glyphs require a 6-component tensor field".into(),
        })?;
    let stride = stride.max(1);
    let sphere = unit_sphere();
    // Per-glyph ellipsoids in parallel, merged in point order.
    let glyphs = crate::parallel::parallel_map(dataset.point_count(), |i| {
        if i % stride != 0 {
            return None;
        }
        let t = tensors[i];
        if !t.iter().all(|v| v.is_finite()) {
            return None;
        }
        let origin = dataset.positions[i];
        // Field order is xx, yy, zz, xy, yz, xz.
        let matrix = [[t[0], t[3], t[5]], [t[3], t[1], t[4]], [t[5], t[4], t[2]]];
        let (eigenvalues, vectors) = crate::eigen::symmetric_eigen(matrix);
        let radii: [f64; 3] = eigenvalues.map(|v| scale * v.abs().sqrt());
        let largest = eigenvalues.iter().map(|v| v.abs()).fold(0.0, f64::max);
        let positions: Vec<[f64; 3]> = sphere
            .0
            .iter()
            .map(|dir| {
                let mut p = origin;
                for axis in 0..3 {
                    let scaled = dir[axis] * radii[axis];
                    p[0] += vectors[0][axis] * scaled;
                    p[1] += vectors[1][axis] * scaled;
                    p[2] += vectors[2][axis] * scaled;
                }
                p
            })
            .collect();
        Some((positions, largest))
    });
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut magnitudes: Vec<f64> = Vec::new();
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    for (local_positions, largest) in glyphs.into_iter().flatten() {
        let base = positions.len() as u32;
        magnitudes.extend(std::iter::repeat_n(largest, local_positions.len()));
        positions.extend(local_positions);
        for tri in &sphere.1 {
            cells.push((
                CellType::Triangle,
                vec![base + tri[0], base + tri[1], base + tri[2]],
            ));
        }
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    Ok(DataSet::new(
        format!("{}-tensor-glyphs", dataset.id),
        positions,
        Cells::from_cells(&refs),
    )
    .with_field(Field::scalar("magnitude", magnitudes)))
}

/// Indexed unit sphere (vertices, triangles) for tensor glyphs.
fn unit_sphere() -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
    let mut vertices: Vec<[f64; 3]> = vec![[0.0, 0.0, 1.0]];
    for stack in 1..TENSOR_STACKS {
        let phi = std::f64::consts::PI * stack as f64 / TENSOR_STACKS as f64;
        for sector in 0..TENSOR_SECTORS {
            let theta = sector as f64 / TENSOR_SECTORS as f64 * std::f64::consts::TAU;
            vertices.push([phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos()]);
        }
    }
    let bottom = vertices.len() as u32;
    vertices.push([0.0, 0.0, -1.0]);
    let ring_start = |stack: usize| -> u32 { 1 + ((stack - 1) * TENSOR_SECTORS) as u32 };
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    for s in 0..TENSOR_SECTORS {
        let next = (s + 1) % TENSOR_SECTORS;
        triangles.push([0, ring_start(1) + next as u32, ring_start(1) + s as u32]);
        let last = ring_start(TENSOR_STACKS - 1);
        triangles.push([bottom, last + s as u32, last + next as u32]);
    }
    for stack in 1..TENSOR_STACKS - 1 {
        let (a, b) = (ring_start(stack), ring_start(stack + 1));
        for s in 0..TENSOR_SECTORS {
            let next = (s + 1) % TENSOR_SECTORS;
            let (s, next) = (s as u32, next as u32);
            triangles.push([a + s, b + next, b + s]);
            triangles.push([a + s, a + next, b + next]);
        }
    }
    (vertices, triangles)
}

type ArrowGeometry = (Vec<[f64; 3]>, Vec<[u32; 3]>, f64);

/// One solid arrow: (positions, triangles, speed); `None` for zero or
/// non-finite vectors.
fn arrow_geometry(
    origin: [f64; 3],
    v: [f64; 3],
    scale: f64,
    radius_fraction: f64,
) -> Option<ArrowGeometry> {
    let speed = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if speed == 0.0 || !speed.is_finite() {
        return None;
    }
    let length = scale * speed;
    let dir = [v[0] / speed, v[1] / speed, v[2] / speed];
    let radius = radius_fraction.max(0.0) * length;
    let head_base = length * (1.0 - HEAD_FRACTION);
    let tip = [
        origin[0] + dir[0] * length,
        origin[1] + dir[1] * length,
        origin[2] + dir[2] * length,
    ];
    let u = perpendicular(dir);
    let w = [
        dir[1] * u[2] - dir[2] * u[1],
        dir[2] * u[0] - dir[0] * u[2],
        dir[0] * u[1] - dir[1] * u[0],
    ];
    let mut positions: Vec<[f64; 3]> = Vec::with_capacity(19);
    let ring = |centre: [f64; 3], r: f64, positions: &mut Vec<[f64; 3]>| -> Vec<u32> {
        (0..ARROW_SIDES)
            .map(|s| {
                let a = s as f64 / ARROW_SIDES as f64 * std::f64::consts::TAU;
                let offset = [
                    (u[0] * a.cos() + w[0] * a.sin()) * r,
                    (u[1] * a.cos() + w[1] * a.sin()) * r,
                    (u[2] * a.cos() + w[2] * a.sin()) * r,
                ];
                let id = positions.len() as u32;
                positions.push([
                    centre[0] + offset[0],
                    centre[1] + offset[1],
                    centre[2] + offset[2],
                ]);
                id
            })
            .collect()
    };
    let shaft_top_centre = [
        origin[0] + dir[0] * head_base,
        origin[1] + dir[1] * head_base,
        origin[2] + dir[2] * head_base,
    ];
    let base = ring(origin, radius, &mut positions);
    let top = ring(shaft_top_centre, radius, &mut positions);
    let head = ring(shaft_top_centre, radius * 2.0, &mut positions);
    let tip_id = positions.len() as u32;
    positions.push(tip);
    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(24);
    for s in 0..ARROW_SIDES {
        let s2 = (s + 1) % ARROW_SIDES;
        triangles.push([base[s], base[s2], top[s2]]);
        triangles.push([base[s], top[s2], top[s]]);
        triangles.push([head[s], head[s2], tip_id]);
        triangles.push([top[s], head[s2], head[s]]);
    }
    Some((positions, triangles, speed))
}

fn perpendicular(dir: [f64; 3]) -> [f64; 3] {
    let a = if dir[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let c = [
        dir[1] * a[2] - dir[2] * a[1],
        dir[2] * a[0] - dir[0] * a[2],
        dir[0] * a[1] - dir[1] * a[0],
    ];
    let len = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    if len == 0.0 {
        return [0.0; 3];
    }
    [c[0] / len, c[1] / len, c[2] / len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_mesh::Field;

    #[test]
    fn solid_arrows_are_closed_triangle_meshes() {
        let ds = DataSet::surface(
            "s",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        )
        .with_field(Field::vector(
            "v",
            vec![[2.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        ));
        let out = solid_arrows(&ds, "v", 0.5, 1, 0.05).unwrap();
        // Two non-zero vectors: 24 triangles and 19 vertices each.
        assert_eq!(out.cell_count(), 48);
        assert_eq!(out.point_count(), 38);
        assert_eq!(out.validate(), Ok(()));
        // First arrow points +x with length 0.5 * 2 = 1: tip at x = 1.
        let max_x = out
            .positions
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((max_x - 1.0).abs() < 1e-12);
        let magnitude = out
            .field(Association::Point, "magnitude")
            .unwrap()
            .as_scalar()
            .unwrap();
        assert_eq!(magnitude.len(), 38);
        assert!(magnitude[..19].iter().all(|&m| m == 2.0));
    }

    #[test]
    fn tensor_glyphs_stretch_along_principal_axes() {
        let ds = DataSet::surface("s", vec![[0.0; 3]], &[])
            .with_field(Field::tensor("sigma", vec![[4.0, 1.0, 1.0, 0.0, 0.0, 0.0]]));
        let out = tensor_glyphs(&ds, "sigma", 1.0, 1).unwrap();
        assert_eq!(out.cell_count(), 48);
        assert_eq!(out.validate(), Ok(()));
        let extent = |axis: usize| {
            out.positions
                .iter()
                .map(|p| p[axis].abs())
                .fold(0.0, f64::max)
        };
        // sqrt(4) = 2 along x, sqrt(1) = 1 along y and z.
        assert!((extent(0) - 2.0).abs() < 1e-9);
        assert!((extent(1) - 1.0).abs() < 1e-9);
        assert!((extent(2) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn arrows_emit_three_segments_per_point() {
        let ds = DataSet::surface(
            "s",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        )
        .with_field(Field::vector("v", vec![[1.0, 0.0, 0.0]; 3]));
        let out = glyphs(&ds, "v", 0.5, 1, GlyphKind::Arrow).unwrap();
        assert_eq!(out.cell_count(), 9);
        assert_eq!(out.validate(), Ok(()));
    }
}
