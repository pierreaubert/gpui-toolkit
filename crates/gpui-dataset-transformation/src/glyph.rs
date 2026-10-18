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
/// Masked, zero or unrepresentable glyphs are skipped. Line glyphs are centered
/// on their source point with endpoints at ±`scale * v / 2`.
///
/// # Errors
///
/// Returns an error for invalid input, a nonfinite scale, missing/non-vector
/// fields or output indices exceeding the dataset representation.
pub fn glyphs(
    dataset: &DataSet,
    field_name: &str,
    scale: f64,
    stride: usize,
    kind: GlyphKind,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    finite_scale(scale)?;
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
    let mut connectivity = Vec::new();
    for i in (0..dataset.point_count()).step_by(stride) {
        if field.valid.as_ref().is_some_and(|mask| !mask[i]) || scale == 0.0 {
            continue;
        }
        let origin = dataset.positions[i];
        let v = vectors[i];
        let len = crate::calculator::magnitude(v);
        if len == 0.0 || !len.is_finite() {
            continue;
        }
        let dir = [v[0] / len, v[1] / len, v[2] / len];
        match kind {
            GlyphKind::Arrow => {
                let tip = std::array::from_fn(|axis| scale.mul_add(v[axis], origin[axis]));
                // Arrow head: two short segments at 150 degrees in an
                // arbitrary plane containing the direction.
                let side = perpendicular(dir);
                let head = scale * len * 0.25;
                let back = |sign: f64| {
                    std::array::from_fn(|axis| {
                        tip[axis] - dir[axis] * head + sign * side[axis] * head * 0.5
                    })
                };
                let local = [origin, tip, back(1.0), back(-1.0)];
                if !local.iter().flatten().all(|v| v.is_finite()) {
                    continue;
                }
                let base = glyph_base(positions.len(), local.len())?;
                positions.extend(local);
                for edge in [[0, 1], [1, 2], [1, 3]] {
                    connectivity.extend(edge.map(|index| base + index));
                }
            }
            GlyphKind::Line => {
                let a = std::array::from_fn(|axis| (-0.5 * scale).mul_add(v[axis], origin[axis]));
                let b = std::array::from_fn(|axis| (0.5 * scale).mul_add(v[axis], origin[axis]));
                if ![a, b].iter().flatten().all(|v| v.is_finite()) {
                    continue;
                }
                let base = glyph_base(positions.len(), 2)?;
                positions.extend([a, b]);
                connectivity.extend([base, base + 1]);
            }
        }
    }
    let count = connectivity.len() / 2;
    Ok(DataSet::new(
        format!("{}-glyphs", dataset.id),
        positions,
        Cells {
            types: vec![CellType::Line; count].into(),
            offsets: (0..=count).map(|i| i as u64 * 2).collect::<Vec<_>>().into(),
            connectivity: connectivity.into(),
        },
    ))
}

/// Radial segments around an arrow shaft and head. Six sides read clearly
/// at demo scale while keeping the triangle count low (34 per closed arrow).
const ARROW_SIDES: usize = 6;
/// Fraction of the total arrow length taken by the head.
const HEAD_FRACTION: f64 = 0.3;

/// Builds solid 3D arrows for a vector point field.
///
/// Each arrow is a closed triangle mesh: a six-sided shaft covering the
/// first 70% of the length and a six-sided cone head, so arrows shade and
/// clip like any other surface in `gpui-scene3d` (unlike the line arrows
/// from [`glyphs`]). Length is `|scale| * |v|`; shaft radius is
/// `radius_fraction * length`. A scalar `magnitude` point field is attached
/// so callers can color arrows by speed. Masked, zero and unrepresentable glyphs
/// are skipped. Negative scale reverses direction; negative radius clamps to zero.
///
/// # Errors
///
/// Returns an error for invalid input, nonfinite scale/radius, missing/non-vector
/// fields or output indices exceeding the dataset representation.
pub fn solid_arrows(
    dataset: &DataSet,
    field_name: &str,
    scale: f64,
    stride: usize,
    radius_fraction: f64,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    finite_scale(scale)?;
    finite_scale(radius_fraction)?;
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
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut magnitudes: Vec<f64> = Vec::new();
    let mut cells = Vec::new();
    for i in (0..dataset.point_count()).step_by(stride) {
        if field.valid.as_ref().is_some_and(|mask| !mask[i]) {
            continue;
        }
        let Some((local_positions, triangles, speed)) =
            arrow_geometry(dataset.positions[i], vectors[i], scale, radius_fraction)
        else {
            continue;
        };
        let base = glyph_base(positions.len(), local_positions.len())?;
        magnitudes.extend(std::iter::repeat_n(speed, local_positions.len()));
        positions.extend(local_positions);
        for tri in triangles {
            cells.push(tri.map(|index| base + index));
        }
    }
    let mut magnitude = Field::scalar("magnitude", magnitudes);
    magnitude.unit.clone_from(&field.unit);
    Ok(DataSet::new(
        format!("{}-arrows", dataset.id),
        positions,
        Cells::triangles(&cells),
    )
    .with_field(magnitude))
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
/// lengths are `|scale| * sqrt(|eigenvalue|)` (stress/strain tensors are
/// indefinite, so magnitudes are used; the sign is not encoded). A scalar
/// `magnitude` point field carries the largest absolute eigenvalue for
/// coloring. Masked or unrepresentable glyphs are skipped. Magnitude units
/// match the source tensor field.
///
/// # Errors
///
/// Returns an error for invalid input, a nonfinite scale, missing/non-tensor
/// fields or output indices exceeding the dataset representation.
pub fn tensor_glyphs(
    dataset: &DataSet,
    field_name: &str,
    scale: f64,
    stride: usize,
) -> Result<DataSet, TransformError> {
    dataset.validate()?;
    finite_scale(scale)?;
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
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut magnitudes: Vec<f64> = Vec::new();
    let mut cells = Vec::new();
    for i in (0..dataset.point_count()).step_by(stride) {
        if field.valid.as_ref().is_some_and(|mask| !mask[i]) {
            continue;
        }
        let t = tensors[i];
        let origin = dataset.positions[i];
        // Field order is xx, yy, zz, xy, yz, xz.
        let matrix = [[t[0], t[3], t[5]], [t[3], t[1], t[4]], [t[5], t[4], t[2]]];
        let (eigenvalues, vectors) = crate::eigen::symmetric_eigen(matrix);
        if !eigenvalues
            .iter()
            .chain(vectors.iter().flatten())
            .all(|v| v.is_finite())
        {
            continue;
        }
        let radii: [f64; 3] = eigenvalues.map(|v| scale.abs() * v.abs().sqrt());
        let largest = eigenvalues.iter().map(|v| v.abs()).fold(0.0, f64::max);
        let local_positions: [[f64; 3]; 2 + (TENSOR_STACKS - 1) * TENSOR_SECTORS] =
            std::array::from_fn(|index| {
                let dir = sphere.0[index];
                let mut p = origin;
                for axis in 0..3 {
                    let scaled = dir[axis] * radii[axis];
                    p[0] += vectors[0][axis] * scaled;
                    p[1] += vectors[1][axis] * scaled;
                    p[2] += vectors[2][axis] * scaled;
                }
                p
            });
        if !local_positions.iter().flatten().all(|v| v.is_finite()) {
            continue;
        }
        let base = glyph_base(positions.len(), local_positions.len())?;
        magnitudes.extend(std::iter::repeat_n(largest, local_positions.len()));
        positions.extend(local_positions);
        for tri in &sphere.1 {
            cells.push(tri.map(|index| base + index));
        }
    }
    let mut magnitude = Field::scalar("magnitude", magnitudes);
    magnitude.unit.clone_from(&field.unit);
    Ok(DataSet::new(
        format!("{}-tensor-glyphs", dataset.id),
        positions,
        Cells::triangles(&cells),
    )
    .with_field(magnitude))
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
    // Sphere indices must face outward for shading and back-face culling.
    for triangle in &mut triangles {
        triangle.swap(1, 2);
    }
    (vertices, triangles)
}

type ArrowGeometry = ([[f64; 3]; 19], [[u32; 3]; 34], f64);

/// One solid arrow: (positions, triangles, speed); `None` for zero or
/// non-finite vectors.
fn arrow_geometry(
    origin: [f64; 3],
    v: [f64; 3],
    scale: f64,
    radius_fraction: f64,
) -> Option<ArrowGeometry> {
    let speed = crate::calculator::magnitude(v);
    if speed == 0.0 || !speed.is_finite() {
        return None;
    }
    let length = scale.abs() * speed;
    if !length.is_finite() || length == 0.0 {
        return None;
    }
    let dir = v.map(|component| component / speed * scale.signum());
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
    const CIRCLE: [[f64; 2]; 6] = [
        [1.0, 0.0],
        [0.5, 0.866_025_403_784_438_6],
        [-0.5, 0.866_025_403_784_438_6],
        [-1.0, 0.0],
        [-0.5, -0.866_025_403_784_438_6],
        [0.5, -0.866_025_403_784_438_6],
    ];
    let radial: [[f64; 3]; 6] =
        CIRCLE.map(|[cos, sin]| std::array::from_fn(|axis| u[axis] * cos + w[axis] * sin));
    let mut positions = [[0.0; 3]; 19];
    let ring =
        |centre: [f64; 3], r: f64, start: usize, positions: &mut [[f64; 3]; 19]| -> [u32; 6] {
            std::array::from_fn(|s| {
                let offset = radial[s].map(|component| component * r);
                let id = start + s;
                positions[id] = [
                    centre[0] + offset[0],
                    centre[1] + offset[1],
                    centre[2] + offset[2],
                ];
                id as u32
            })
        };
    let shaft_top_centre = [
        origin[0] + dir[0] * head_base,
        origin[1] + dir[1] * head_base,
        origin[2] + dir[2] * head_base,
    ];
    let base = ring(origin, radius, 0, &mut positions);
    let top = ring(shaft_top_centre, radius, 6, &mut positions);
    let head = ring(shaft_top_centre, radius * 2.0, 12, &mut positions);
    let tip_id = 18;
    positions[18] = tip;
    if !positions.iter().flatten().all(|v| v.is_finite()) {
        return None;
    }
    let mut triangles = [[0; 3]; 34];
    for s in 0..ARROW_SIDES {
        let s2 = (s + 1) % ARROW_SIDES;
        triangles[2 * s] = [base[s], base[s2], top[s2]];
        triangles[2 * s + 1] = [base[s], top[s2], top[s]];
        triangles[12 + 2 * s] = [top[s], top[s2], head[s2]];
        triangles[12 + 2 * s + 1] = [top[s], head[s2], head[s]];
        triangles[24 + s] = [head[s], head[s2], tip_id];
    }
    for s in 1..ARROW_SIDES - 1 {
        triangles[29 + s] = [base[0], base[s + 1], base[s]];
    }
    Some((positions, triangles, speed))
}

fn finite_scale(scale: f64) -> Result<(), TransformError> {
    if scale.is_finite() {
        Ok(())
    } else {
        Err(TransformError::Degenerate(
            "glyph scale/radius must be finite".into(),
        ))
    }
}

fn glyph_base(points: usize, count: usize) -> Result<u32, TransformError> {
    let error = || TransformError::Degenerate("glyph output exceeds u32 point indices".into());
    let end = points.checked_add(count).ok_or_else(error)?;
    if end == 0 || u32::try_from(end - 1).is_err() {
        return Err(error());
    }
    u32::try_from(points).map_err(|_| error())
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
    fn solid_arrow_edges_are_manifold_and_oriented_for_both_scale_signs() {
        for scale in [1.0, -1.0] {
            let (positions, triangles, _) =
                arrow_geometry([0.0; 3], [1.0, 2.0, 3.0], scale, 0.05).unwrap();
            let mut edges = std::collections::BTreeMap::<(u32, u32), (usize, i32)>::new();
            let mut volume = 0.0;
            for [a, b, c] in triangles {
                for [u, v] in [[a, b], [b, c], [c, a]] {
                    let entry = edges.entry((u.min(v), u.max(v))).or_default();
                    entry.0 += 1;
                    entry.1 += if u < v { 1 } else { -1 };
                }
                let [a, b, c] = [
                    positions[a as usize],
                    positions[b as usize],
                    positions[c as usize],
                ];
                let cross = [
                    b[1] * c[2] - b[2] * c[1],
                    b[2] * c[0] - b[0] * c[2],
                    b[0] * c[1] - b[1] * c[0],
                ];
                volume += a.iter().zip(cross).map(|(a, b)| a * b).sum::<f64>() / 6.0;
            }
            assert!(edges.values().all(|entry| *entry == (2, 0)));
            assert!(volume > 0.0);
            assert_eq!(
                positions.len() as isize - edges.len() as isize + triangles.len() as isize,
                2
            );
        }
    }

    #[test]
    fn glyph_index_ranges_are_checked_without_large_allocations() {
        assert_eq!(glyph_base(10, 19).unwrap(), 10);
        glyph_base(usize::MAX, 19).unwrap_err();
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(glyph_base(u32::MAX as usize, 1).unwrap(), u32::MAX);
            glyph_base(u32::MAX as usize, 2).unwrap_err();
        }
    }

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
        // Two non-zero vectors: 34 triangles and 19 vertices each.
        assert_eq!(out.cell_count(), 68);
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
