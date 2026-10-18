//! Volume raycasting over tetrahedralized datasets.
//!
//! Rays are marched through the volume and composited front-to-back with
//! the actor transfer function. Scalar sampling uses the exact P1
//! (barycentric) interpolation inside the containing tetrahedron; a
//! uniform grid over tetrahedron bounds keeps sampling near-constant-time
//! on large meshes instead of scanning every element per step.

// Rust guideline compliant 2026-02-21

use crate::camera::Camera;
use crate::scene::ClipPlane;
use crate::transfer::TransferFunction;
use crate::{Frame, SceneError};
use gpui_mesh::{Association, CellType, DataSet};

fn tetrahedralize(kind: CellType, n: &[u32]) -> Vec<[u32; 4]> {
    match kind {
        CellType::Tetra => vec![[n[0], n[1], n[2], n[3]]],
        CellType::Hexahedron => vec![
            [n[0], n[1], n[2], n[6]],
            [n[0], n[2], n[3], n[6]],
            [n[0], n[3], n[7], n[6]],
            [n[0], n[7], n[4], n[6]],
            [n[0], n[4], n[5], n[6]],
            [n[0], n[5], n[1], n[6]],
        ],
        CellType::Wedge => vec![
            [n[0], n[1], n[2], n[3]],
            [n[1], n[2], n[3], n[4]],
            [n[2], n[3], n[4], n[5]],
        ],
        CellType::Pyramid => vec![[n[0], n[1], n[2], n[4]], [n[0], n[2], n[3], n[4]]],
        _ => Vec::new(),
    }
}

fn barycentric(tet: [[f64; 3]; 4], p: [f64; 3]) -> Option<[f64; 4]> {
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

/// Uniform grid indexing tetrahedra by their bounds, for fast sampling.
#[derive(Debug)]
struct TetGrid {
    tets: Vec<[u32; 4]>,
    origin: [f64; 3],
    cell: [f64; 3],
    dims: [usize; 3],
    entries: Vec<Vec<u32>>,
}

impl TetGrid {
    fn build(dataset: &DataSet, bounds: gpui_mesh::Bounds) -> Self {
        let mut tets = Vec::new();
        for cell in 0..dataset.cell_count() {
            let kind = dataset.cells.types[cell];
            if kind.is_volume() {
                let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
                tets.extend(tetrahedralize(kind, nodes));
            }
        }
        // About 8 grid cells per axis per 1000 tets, clamped to sane sizes.
        let resolution = ((tets.len() as f64).cbrt().ceil() as usize).clamp(1, 64);
        let dims = [resolution, resolution, resolution];
        let mut cell = [0.0; 3];
        for axis in 0..3 {
            let extent = bounds.max[axis] - bounds.min[axis];
            cell[axis] = if extent > 0.0 {
                extent / dims[axis] as f64
            } else {
                1.0
            };
        }
        let mut entries: Vec<Vec<u32>> = vec![Vec::new(); dims[0] * dims[1] * dims[2]];
        for (index, tet) in tets.iter().enumerate() {
            let mut lo = [usize::MAX; 3];
            let mut hi = [0usize; 3];
            for &node in tet {
                let p = dataset.positions[node as usize];
                for axis in 0..3 {
                    let c =
                        (((p[axis] - bounds.min[axis]) / cell[axis]) as usize).min(dims[axis] - 1);
                    lo[axis] = lo[axis].min(c);
                    hi[axis] = hi[axis].max(c);
                }
            }
            for k in lo[2]..=hi[2] {
                for j in lo[1]..=hi[1] {
                    for i in lo[0]..=hi[0] {
                        entries[k * dims[1] * dims[0] + j * dims[0] + i].push(index as u32);
                    }
                }
            }
        }
        Self {
            tets,
            origin: bounds.min,
            cell,
            dims,
            entries,
        }
    }

    fn sample(&self, dataset: &DataSet, scalars: &[f64], point: [f64; 3]) -> Option<f64> {
        let mut grid_index = [0usize; 3];
        for axis in 0..3 {
            let c = ((point[axis] - self.origin[axis]) / self.cell[axis]).floor();
            if c < 0.0 || c >= self.dims[axis] as f64 {
                return None;
            }
            grid_index[axis] = c as usize;
        }
        let bucket = &self.entries[grid_index[2] * self.dims[1] * self.dims[0]
            + grid_index[1] * self.dims[0]
            + grid_index[0]];
        for &tet_index in bucket {
            let tet = self.tets[tet_index as usize];
            let corners = [
                dataset.positions[tet[0] as usize],
                dataset.positions[tet[1] as usize],
                dataset.positions[tet[2] as usize],
                dataset.positions[tet[3] as usize],
            ];
            if let Some(w) = barycentric(corners, point) {
                return Some(
                    w.iter()
                        .enumerate()
                        .map(|(i, weight)| weight * scalars[tet[i] as usize])
                        .sum(),
                );
            }
        }
        None
    }
}

/// Options for volume raycasting.
#[derive(Debug, Clone)]
pub struct VolumeOptions {
    /// Marching step as a fraction of the bounds diagonal (smaller is finer).
    pub step_fraction: f64,
    /// Planes that cut the volume away (positive side is kept).
    pub clip_planes: Vec<ClipPlane>,
    /// Background RGBA in `[0,1]`.
    pub background: [f32; 4],
}

impl Default for VolumeOptions {
    fn default() -> Self {
        Self {
            step_fraction: 0.01,
            clip_planes: Vec::new(),
            background: [0.08, 0.09, 0.11, 1.0],
        }
    }
}

/// A raycaster with a retained tetrahedron grid for one dataset and field.
///
/// Building the grid is the expensive step; retaining it makes per-frame
/// raycasting (movies, interactive camera moves) pay only for marching.
#[derive(Debug)]
pub struct VolumeRenderer {
    dataset: DataSet,
    scalars: Vec<f64>,
    bounds: gpui_mesh::Bounds,
    grid: TetGrid,
}

impl VolumeRenderer {
    /// Builds the retained grid for a scalar point field.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing or non-scalar field, or a dataset
    /// without volume cells.
    pub fn new(dataset: &DataSet, field_name: &str) -> Result<Self, SceneError> {
        let field = dataset
            .field(Association::Point, field_name)
            .ok_or_else(|| SceneError::FieldNotFound(field_name.to_string()))?;
        let scalars = field
            .as_scalar()
            .ok_or_else(|| SceneError::FieldNotFound(field_name.to_string()))?
            .to_vec();
        let bounds = dataset
            .bounds()
            .ok_or_else(|| SceneError::FieldNotFound(field_name.to_string()))?;
        let grid = TetGrid::build(dataset, bounds);
        if grid.tets.is_empty() {
            return Err(SceneError::FieldNotFound(format!(
                "dataset '{}' holds no volume cells",
                dataset.id
            )));
        }
        Ok(Self {
            dataset: dataset.clone(),
            scalars,
            bounds,
            grid,
        })
    }

    /// Raycasts one frame from a camera.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid dimensions, exceeded default memory limits,
    /// allocation failure, or a singular camera matrix.
    pub fn render(
        &self,
        transfer: &TransferFunction,
        camera: &Camera,
        width: u32,
        height: u32,
        options: &VolumeOptions,
    ) -> Result<Frame, SceneError> {
        self.render_with_limits(
            transfer,
            camera,
            width,
            height,
            options,
            crate::RenderLimits::default(),
        )
    }

    /// Raycasts a frame within explicit working-memory limits.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid dimensions, exceeded limits, allocation
    /// failure, or a singular camera matrix.
    pub fn render_with_limits(
        &self,
        transfer: &TransferFunction,
        camera: &Camera,
        width: u32,
        height: u32,
        options: &VolumeOptions,
        limits: crate::RenderLimits,
    ) -> Result<Frame, SceneError> {
        let pixels = limits.pixel_count(width, height, 4)?;
        let dataset = &self.dataset;
        let scalars: &[f64] = &self.scalars;
        let bounds = self.bounds;
        let grid = &self.grid;
        let aspect = width as f32 / height as f32;
        let view_projection = camera.view_projection(aspect);
        let inverse = invert4(view_projection)
            .ok_or_else(|| SceneError::FieldNotFound("camera matrix is singular".into()))?;
        let eye = camera.eye();
        let eye64 = [f64::from(eye[0]), f64::from(eye[1]), f64::from(eye[2])];
        let diagonal = bounds.diagonal();
        let step = (diagonal * options.step_fraction).max(diagonal / 4096.0);
        let bg = options.background;

        let mut rgba = crate::limits::frame_buffer(pixels * 4, 0u8)?;
        for y in 0..height {
            for x in 0..width {
                let idx = (y as usize * width as usize + x as usize) * 4;
                // Unproject the pixel centre at the near and far planes.
                let ndc_x = (x as f32 + 0.5) / width as f32 * 2.0 - 1.0;
                let ndc_y = 1.0 - (y as f32 + 0.5) / height as f32 * 2.0;
                let Some(far_point) = unproject(&inverse, [ndc_x, ndc_y, 1.0]) else {
                    write_pixel(&mut rgba, idx, bg);
                    continue;
                };
                let mut dir = [
                    far_point[0] - eye64[0],
                    far_point[1] - eye64[1],
                    far_point[2] - eye64[2],
                ];
                let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
                if len < 1e-300 {
                    write_pixel(&mut rgba, idx, bg);
                    continue;
                }
                dir = [dir[0] / len, dir[1] / len, dir[2] / len];
                let Some((t_enter, t_exit)) = ray_bounds(eye64, dir, bounds) else {
                    write_pixel(&mut rgba, idx, bg);
                    continue;
                };
                // Front-to-back compositing with early termination.
                let mut accumulated = [0.0f32; 4];
                let mut t = t_enter;
                while t < t_exit && accumulated[3] < 0.98 {
                    let point = [
                        eye64[0] + dir[0] * t,
                        eye64[1] + dir[1] * t,
                        eye64[2] + dir[2] * t,
                    ];
                    t += step;
                    if options
                        .clip_planes
                        .iter()
                        .any(|plane| plane.distance(point) < 0.0)
                    {
                        continue;
                    }
                    let Some(value) = grid.sample(dataset, scalars, point) else {
                        continue;
                    };
                    let colour = transfer.map(value);
                    if colour[3] <= 0.0 {
                        continue;
                    }
                    // Opacity corrected for step length relative to the diagonal.
                    let alpha =
                        (f64::from(colour[3]) * (step / diagonal) * 64.0).clamp(0.0, 1.0) as f32;
                    let weight = (1.0 - accumulated[3]) * alpha;
                    accumulated[0] += weight * colour[0];
                    accumulated[1] += weight * colour[1];
                    accumulated[2] += weight * colour[2];
                    accumulated[3] += weight;
                }
                let out = [
                    accumulated[0] + (1.0 - accumulated[3]) * bg[0],
                    accumulated[1] + (1.0 - accumulated[3]) * bg[1],
                    accumulated[2] + (1.0 - accumulated[3]) * bg[2],
                    1.0,
                ];
                rgba[idx] = (out[0].clamp(0.0, 1.0) * 255.0) as u8;
                rgba[idx + 1] = (out[1].clamp(0.0, 1.0) * 255.0) as u8;
                rgba[idx + 2] = (out[2].clamp(0.0, 1.0) * 255.0) as u8;
                rgba[idx + 3] = 255;
            }
        }
        Ok(Frame {
            width,
            height,
            rgba,
        })
    }
}

/// Raycasts a volume dataset coloured by a scalar point field.
///
/// One-shot convenience over [`VolumeRenderer`]; retain a renderer when
/// raycasting more than one frame of the same dataset.
///
/// # Errors
///
/// Returns an error for a missing or non-scalar field, a zero size, or a
/// dataset without volume cells.
pub fn raycast_volume(
    dataset: &DataSet,
    field_name: &str,
    transfer: &TransferFunction,
    camera: &Camera,
    width: u32,
    height: u32,
    options: &VolumeOptions,
) -> Result<Frame, SceneError> {
    VolumeRenderer::new(dataset, field_name)?.render(transfer, camera, width, height, options)
}

fn write_pixel(rgba: &mut [u8], idx: usize, colour: [f32; 4]) {
    rgba[idx] = (colour[0].clamp(0.0, 1.0) * 255.0) as u8;
    rgba[idx + 1] = (colour[1].clamp(0.0, 1.0) * 255.0) as u8;
    rgba[idx + 2] = (colour[2].clamp(0.0, 1.0) * 255.0) as u8;
    rgba[idx + 3] = (colour[3].clamp(0.0, 1.0) * 255.0) as u8;
}

fn unproject(inverse: &[f32; 16], ndc: [f32; 3]) -> Option<[f64; 3]> {
    let clip = crate::camera::transform(inverse, ndc);
    if clip[3].abs() < 1e-12 {
        return None;
    }
    Some([
        f64::from(clip[0] / clip[3]),
        f64::from(clip[1] / clip[3]),
        f64::from(clip[2] / clip[3]),
    ])
}

/// Slab intersection of a ray with the dataset bounds.
fn ray_bounds(origin: [f64; 3], dir: [f64; 3], bounds: gpui_mesh::Bounds) -> Option<(f64, f64)> {
    let mut t_min = 0.0f64;
    let mut t_max = f64::INFINITY;
    for axis in 0..3 {
        if dir[axis].abs() < 1e-300 {
            if origin[axis] < bounds.min[axis] || origin[axis] > bounds.max[axis] {
                return None;
            }
            continue;
        }
        let mut t1 = (bounds.min[axis] - origin[axis]) / dir[axis];
        let mut t2 = (bounds.max[axis] - origin[axis]) / dir[axis];
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        if t_min > t_max {
            return None;
        }
    }
    Some((t_min, t_max))
}

/// Inverts a column-major 4x4 matrix; `None` when singular.
#[must_use]
pub fn invert4(m: [f32; 16]) -> Option<[f32; 16]> {
    // Augment with identity and Gauss-Jordan eliminate in f64.
    let mut a = [[0.0f64; 8]; 4];
    for row in 0..4 {
        for col in 0..4 {
            a[row][col] = f64::from(m[col * 4 + row]);
        }
        a[row][4 + row] = 1.0;
    }
    for col in 0..4 {
        let pivot = (col..4).max_by(|&r1, &r2| {
            a[r1][col]
                .abs()
                .partial_cmp(&a[r2][col].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        let divisor = a[col][col];
        for value in &mut a[col] {
            *value /= divisor;
        }
        for row in 0..4 {
            if row != col {
                let factor = a[row][col];
                let pivot_row = a[col];
                for (j, value) in a[row].iter_mut().enumerate() {
                    *value -= factor * pivot_row[j];
                }
            }
        }
    }
    let mut out = [0.0f32; 16];
    for row in 0..4 {
        for col in 0..4 {
            out[col * 4 + row] = a[row][4 + col] as f32;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::multiply;
    use gpui_mesh::Field;

    fn gaussian_room() -> DataSet {
        let mut ds = DataSet::structured_grid("room", [10, 10, 10], [-1.0; 3], [0.2; 3]).unwrap();
        let values: Vec<f64> = ds
            .positions
            .iter()
            .map(|p| {
                let d2 = p[0] * p[0] + p[1] * p[1] + p[2] * p[2];
                (-2.0 * d2).exp()
            })
            .collect();
        ds.point_data.push(Field::scalar("pressure", values));
        ds
    }

    #[test]
    fn raycast_sees_the_gaussian_core() {
        let ds = gaussian_room();
        // Transfer: transparent at 0, opaque white-ish at 1 via viridis alpha 1.
        let transfer = TransferFunction::viridis([0.0, 1.0]);
        let camera = Camera::fitted(ds.bounds().unwrap());
        let frame = raycast_volume(
            &ds,
            "pressure",
            &transfer,
            &camera,
            48,
            48,
            &VolumeOptions::default(),
        )
        .unwrap();
        // Centre pixel accumulated far more than a corner pixel.
        let centre = ((24 * 48 + 24) * 4) as usize;
        let corner = 0usize;
        assert!(
            frame.rgba[centre] > frame.rgba[corner],
            "centre {} vs corner {}",
            frame.rgba[centre],
            frame.rgba[corner]
        );
    }

    #[test]
    fn clipping_half_the_volume_darkens_the_centre_ray() {
        let ds = gaussian_room();
        let transfer = TransferFunction::viridis([0.0, 1.0]);
        let camera = Camera::fitted(ds.bounds().unwrap());
        let full = raycast_volume(
            &ds,
            "pressure",
            &transfer,
            &camera,
            32,
            32,
            &VolumeOptions::default(),
        )
        .unwrap();
        let clipped = raycast_volume(
            &ds,
            "pressure",
            &transfer,
            &camera,
            32,
            32,
            &VolumeOptions {
                clip_planes: vec![ClipPlane {
                    origin: [0.0, 0.0, 0.0],
                    normal: [-1.0, 0.0, 0.0],
                }],
                ..VolumeOptions::default()
            },
        )
        .unwrap();
        let sum = |f: &Frame| -> u64 { f.rgba.iter().map(|b| u64::from(*b)).sum() };
        assert_ne!(sum(&clipped), sum(&full));
    }

    #[test]
    fn invert_round_trips() {
        let camera = Camera::default();
        let m = camera.view_projection(1.5);
        let inverse = invert4(m).unwrap();
        let product = multiply(inverse, m);
        for i in 0..4 {
            assert!(
                (product[i * 4 + i] - 1.0).abs() < 1e-4,
                "diag {i}: {}",
                product[i * 4 + i]
            );
        }
    }
}
