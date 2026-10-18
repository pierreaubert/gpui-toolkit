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
use gpui_mesh::{Association, CellType, DataSet, Field, FieldData};
use std::sync::Arc;

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
    fn build(dataset: &DataSet, field: &Field, bounds: gpui_mesh::Bounds) -> Self {
        let mut tets = Vec::new();
        for cell in 0..dataset.cell_count() {
            let kind = dataset.cells.types[cell];
            if kind.is_volume() {
                let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
                tets.extend(tetrahedralize(kind, nodes).into_iter().filter(|tet| {
                    field.valid.is_none() || tet.iter().all(|&node| field.is_valid(node as usize))
                }));
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

    fn sample(&self, positions: &[[f64; 3]], scalars: &[f64], point: [f64; 3]) -> Option<f64> {
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
                positions[tet[0] as usize],
                positions[tet[1] as usize],
                positions[tet[2] as usize],
                positions[tet[3] as usize],
            ];
            if let Some(w) = barycentric(corners, point) {
                let value: f64 = w
                    .iter()
                    .enumerate()
                    .map(|(i, weight)| weight * scalars[tet[i] as usize])
                    .sum();
                return value.is_finite().then_some(value);
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
/// Positions and selected scalars share immutable source storage; unrelated
/// fields are not retained. Only tetrahedra with four valid scalar nodes are
/// indexed, so wholly missing fields render as background.
#[derive(Debug)]
pub struct VolumeRenderer {
    positions: Arc<[[f64; 3]]>,
    scalars: Arc<[f64]>,
    bounds: gpui_mesh::Bounds,
    grid: TetGrid,
}

impl VolumeRenderer {
    /// Builds the retained grid for a scalar point field.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid dataset, a missing/non-scalar field,
    /// or a dataset without volume cells.
    pub fn new(dataset: &DataSet, field_name: &str) -> Result<Self, SceneError> {
        dataset.validate()?;
        let field = dataset
            .field(Association::Point, field_name)
            .ok_or_else(|| SceneError::FieldNotFound(field_name.to_string()))?;
        let FieldData::Scalar(scalars) = &field.data else {
            return Err(SceneError::FieldNotFound(field_name.to_string()));
        };
        let bounds = dataset
            .bounds()
            .ok_or_else(|| SceneError::FieldNotFound(field_name.to_string()))?;
        if !dataset.cells.types.iter().any(|kind| kind.is_volume()) {
            return Err(SceneError::FieldNotFound(format!(
                "dataset '{}' holds no volume cells",
                dataset.id
            )));
        }
        let grid = TetGrid::build(dataset, field, bounds);
        Ok(Self {
            positions: dataset.positions.clone(),
            scalars: scalars.clone(),
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
        let transfer = transfer.prepare()?;
        crate::scene::validate_rgba(options.background)?;
        let pixels = limits.pixel_count(width, height, 4)?;
        let positions = &self.positions;
        let scalars: &[f64] = &self.scalars;
        let bounds = self.bounds;
        let grid = &self.grid;
        let aspect = width as f32 / height as f32;
        let origin = bounds.center();
        let rays = camera.ray_grid_relative_to(origin, aspect)?;
        let local_bounds = gpui_mesh::Bounds {
            min: std::array::from_fn(|axis| bounds.min[axis] - origin[axis]),
            max: std::array::from_fn(|axis| bounds.max[axis] - origin[axis]),
        };
        let planes = crate::scene::local_planes(&options.clip_planes, origin)?;
        let diagonal = bounds.diagonal();
        let step = (diagonal * options.step_fraction).max(diagonal / 4096.0);
        let bg = options.background;

        let mut rgba = crate::limits::frame_buffer(pixels * 4, 0u8)?;
        if grid.tets.is_empty() {
            for index in (0..rgba.len()).step_by(4) {
                write_pixel(&mut rgba, index, bg);
            }
            return Ok(Frame {
                width,
                height,
                rgba,
            });
        }
        for y in 0..height {
            for x in 0..width {
                let idx = (y as usize * width as usize + x as usize) * 4;
                // Analytic near/far-plane rays preserve both orthographic
                // projection and local precision, without per-pixel inversion.
                let ndc_x = (x as f32 + 0.5) / width as f32 * 2.0 - 1.0;
                let ndc_y = 1.0 - (y as f32 + 0.5) / height as f32 * 2.0;
                let [near_point, far_point] = rays.segment([ndc_x, ndc_y]);
                let mut dir =
                    std::array::from_fn::<_, 3, _>(|axis| far_point[axis] - near_point[axis]);
                let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
                if len < 1e-300 {
                    write_pixel(&mut rgba, idx, bg);
                    continue;
                }
                dir = [dir[0] / len, dir[1] / len, dir[2] / len];
                let Some((t_enter, t_exit)) = ray_bounds(near_point, dir, local_bounds) else {
                    write_pixel(&mut rgba, idx, bg);
                    continue;
                };
                let t_exit = t_exit.min(len);
                // Front-to-back compositing with early termination.
                let mut accumulated = [0.0f32; 4];
                let mut t = t_enter;
                while t < t_exit && accumulated[3] < 0.98 {
                    let point = [
                        near_point[0] + dir[0] * t,
                        near_point[1] + dir[1] * t,
                        near_point[2] + dir[2] * t,
                    ];
                    t += step;
                    if planes.iter().any(|plane| plane.distance(point) < 0.0) {
                        continue;
                    }
                    let Some(value) = grid.sample(
                        positions,
                        scalars,
                        std::array::from_fn(|axis| point[axis] + origin[axis]),
                    ) else {
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
                let remaining = 1.0 - accumulated[3];
                let alpha = accumulated[3] + remaining * bg[3];
                let mut out = [0.0; 4];
                if alpha > 0.0 {
                    for channel in 0..3 {
                        out[channel] =
                            (accumulated[channel] + remaining * bg[channel] * bg[3]) / alpha;
                    }
                }
                out[3] = alpha;
                write_pixel(&mut rgba, idx, out);
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

    #[test]
    fn missing_volume_support_excludes_finite_placeholders_and_masked_nans() {
        let malformed = DataSet::structured_grid("short", [2; 3], [0.0; 3], [1.0; 3])
            .unwrap()
            .with_field(gpui_mesh::Field::scalar("p", vec![0.5; 7]));
        assert!(matches!(
            VolumeRenderer::new(&malformed, "p"),
            Err(SceneError::Mesh(
                gpui_mesh::MeshError::FieldLengthMismatch { .. }
            ))
        ));
        for placeholder in [123.0, f64::NAN] {
            let mut field = gpui_mesh::Field::scalar("p", vec![placeholder; 8]);
            field.valid = Some(vec![false; 8].into());
            let dataset = DataSet::structured_grid("missing", [2; 3], [0.0; 3], [1.0; 3])
                .unwrap()
                .with_field(field);
            let renderer = VolumeRenderer::new(&dataset, "p").unwrap();
            assert!(renderer.grid.tets.is_empty());
            let options = VolumeOptions {
                background: [0.2, 0.4, 0.6, 1.0],
                ..Default::default()
            };
            let frame = renderer
                .render(
                    &TransferFunction::viridis([0.0, 1.0]),
                    &Camera::fitted(dataset.bounds().unwrap()),
                    24,
                    24,
                    &options,
                )
                .unwrap();
            assert!(
                frame
                    .rgba
                    .chunks_exact(4)
                    .all(|pixel| pixel == [51, 102, 153, 255])
            );
        }
        let mut field = gpui_mesh::Field::scalar("p", vec![0.5; 8]);
        field.valid = Some(vec![true, false, true, true, true, true, true, true].into());
        let dataset = DataSet::structured_grid("partial", [2; 3], [0.0; 3], [1.0; 3])
            .unwrap()
            .with_field(field);
        let renderer = VolumeRenderer::new(&dataset, "p").unwrap();
        assert_eq!(renderer.grid.tets.len(), 4);
        assert_eq!(
            renderer
                .grid
                .sample(&renderer.positions, &renderer.scalars, [0.9, 0.1, 0.1]),
            None
        );
        assert!(
            renderer
                .grid
                .sample(&renderer.positions, &renderer.scalars, [0.1, 0.9, 0.5])
                .is_some()
        );
    }

    #[test]
    fn retained_volume_shares_geometry_and_scalars_without_retaining_other_fields() {
        let dataset = DataSet::structured_grid("shared", [2; 3], [0.0; 3], [1.0; 3])
            .unwrap()
            .with_field(gpui_mesh::Field::scalar("p", vec![0.5; 8]))
            .with_field(gpui_mesh::Field::vector("unrelated", vec![[1.0; 3]; 8]));
        let renderer = VolumeRenderer::new(&dataset, "p").unwrap();
        let FieldData::Scalar(scalars) = &dataset.point_data[0].data else {
            unreachable!()
        };
        let FieldData::Vector(unrelated) = &dataset.point_data[1].data else {
            unreachable!()
        };
        assert!(Arc::ptr_eq(&renderer.positions, &dataset.positions));
        assert!(Arc::ptr_eq(&renderer.scalars, scalars));
        let weak_selected = Arc::downgrade(scalars);
        let weak_unrelated = Arc::downgrade(unrelated);
        drop(dataset);
        assert!(weak_selected.upgrade().is_some());
        assert!(weak_unrelated.upgrade().is_none());
        drop(renderer);
        assert!(weak_selected.upgrade().is_none());
    }

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
    fn volume_plane_equations_ignore_normal_scale_and_tangential_origins() {
        let ds = gaussian_room();
        let renderer = VolumeRenderer::new(&ds, "pressure").unwrap();
        let camera = Camera::fitted(ds.bounds().unwrap());
        let transfer = TransferFunction::viridis([0.0, 1.0]);
        let mut options = VolumeOptions {
            clip_planes: vec![ClipPlane {
                origin: [0.125, 0.0, 0.0],
                normal: [1.0, 1.0, 0.0],
            }],
            ..VolumeOptions::default()
        };
        let reference = renderer
            .render(&transfer, &camera, 24, 24, &options)
            .unwrap();
        for magnitude in [1e-300, 1e300] {
            options.clip_planes[0] = ClipPlane {
                origin: [1e12 + 0.125, -1e12, 1e250],
                normal: [magnitude, magnitude, 0.0],
            };
            assert_eq!(
                reference.rgba,
                renderer
                    .render(&transfer, &camera, 24, 24, &options)
                    .unwrap()
                    .rgba
            );
        }
        options.clip_planes[0].normal[0] = f64::NAN;
        assert!(matches!(
            renderer.render(&transfer, &camera, 24, 24, &options),
            Err(SceneError::InvalidClipPlane(_))
        ));
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

    #[test]
    fn orthographic_volume_size_is_distance_invariant_and_obeys_near_far() {
        let dataset = DataSet::structured_grid("box", [3; 3], [-1.0; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("p", vec![0.5; 27]));
        let renderer = VolumeRenderer::new(&dataset, "p").unwrap();
        let transfer = TransferFunction::from_stops([0.0, 1.0], &[(0.0, [1.0, 0.0, 0.0, 0.2])]);
        let options = VolumeOptions {
            background: [0.0, 0.0, 0.0, 1.0],
            ..VolumeOptions::default()
        };
        let camera = Camera {
            azimuth: 0.0,
            elevation: 0.0,
            distance: 2.5,
            near: 0.1,
            far: 8.0,
            projection: crate::Projection::Orthographic { half_height: 1.5 },
            ..Camera::default()
        };
        let draw = |camera: &Camera| {
            renderer
                .render(&transfer, camera, 24, 24, &options)
                .unwrap()
        };
        let reference = draw(&camera);
        assert!(
            reference
                .rgba
                .chunks_exact(4)
                .filter(|pixel| pixel[0] > 10)
                .count()
                > 100
        );
        assert_eq!(
            reference.rgba,
            draw(&Camera {
                distance: 5.5,
                ..camera
            })
            .rgba
        );
        for clipped in [
            Camera {
                distance: 3.0,
                far: 1.5,
                ..camera
            },
            Camera {
                distance: 3.0,
                near: 4.5,
                ..camera
            },
        ] {
            assert!(
                draw(&clipped)
                    .rgba
                    .chunks_exact(4)
                    .all(|pixel| pixel == [0, 0, 0, 255])
            );
        }
    }

    #[test]
    fn volume_compositing_retains_background_and_sample_alpha() {
        let dataset = DataSet::structured_grid("alpha", [3; 3], [-1.0; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("p", vec![0.5; 27]));
        let renderer = VolumeRenderer::new(&dataset, "p").unwrap();
        let camera = Camera {
            azimuth: 0.0,
            elevation: 0.0,
            distance: 3.0,
            projection: crate::Projection::Orthographic { half_height: 1.5 },
            ..Camera::default()
        };
        let transfer =
            |alpha| TransferFunction::from_stops([0.0, 1.0], &[(0.0, [1.0, 0.0, 0.0, alpha])]);
        let options = |background| VolumeOptions {
            background,
            ..VolumeOptions::default()
        };
        let clear = renderer
            .render(
                &transfer(0.0),
                &camera,
                24,
                24,
                &options([0.2, 0.4, 0.6, 0.25]),
            )
            .unwrap();
        assert!(
            clear
                .rgba
                .chunks_exact(4)
                .all(|pixel| pixel == [51, 102, 153, 63])
        );
        let red = renderer
            .render(
                &transfer(0.01),
                &camera,
                24,
                24,
                &options([0.0, 0.0, 1.0, 0.0]),
            )
            .unwrap();
        let center = &red.rgba[((12 * 24 + 12) * 4) as usize..][..4];
        assert_eq!(&center[..3], &[255, 0, 0]);
        assert!(center[3] > 0 && center[3] < 255);
        let mixed = renderer
            .render(
                &transfer(0.01),
                &camera,
                24,
                24,
                &options([0.0, 0.0, 1.0, 0.5]),
            )
            .unwrap();
        let center = &mixed.rgba[((12 * 24 + 12) * 4) as usize..][..4];
        assert!(center[0] > 0 && center[2] > 0 && center[3] > 127 && center[3] < 255);
        assert!((u16::from(center[0]) + u16::from(center[2])).abs_diff(255) <= 2);
    }

    #[test]
    fn invalid_volume_materials_fail_instead_of_entering_the_ray_loop() {
        let ds = gaussian_room();
        let renderer = VolumeRenderer::new(&ds, "pressure").unwrap();
        let camera = Camera::fitted(ds.bounds().unwrap());
        assert!(matches!(
            renderer.render(
                &TransferFunction::viridis([f64::NAN, 1.0]),
                &camera,
                16,
                16,
                &VolumeOptions::default()
            ),
            Err(SceneError::InvalidMaterial(_))
        ));
        let options = VolumeOptions {
            background: [f32::NAN; 4],
            ..VolumeOptions::default()
        };
        assert!(matches!(
            renderer.render(
                &TransferFunction::viridis([0.0, 1.0]),
                &camera,
                16,
                16,
                &options
            ),
            Err(SceneError::InvalidMaterial(_))
        ));
    }

    #[test]
    fn volume_rays_preserve_coverage_after_large_fractional_translation() {
        let transfer = TransferFunction::from_stops([0.0, 1.0], &[(0.0, [1.0, 0.0, 0.0, 0.2])]);
        let options = VolumeOptions {
            background: [0.0, 0.0, 0.0, 1.0],
            ..VolumeOptions::default()
        };
        let mut reference: Option<Vec<u8>> = None;
        for offset in [0.0, 1e9 + 0.125, 1e12 + 0.125] {
            let dataset =
                DataSet::structured_grid("translated", [3; 3], [offset - 1.0; 3], [1.0; 3])
                    .unwrap()
                    .with_field(Field::scalar("p", vec![0.5; 27]));
            let camera = Camera::fitted(dataset.bounds().unwrap());
            let frame = VolumeRenderer::new(&dataset, "p")
                .unwrap()
                .render(&transfer, &camera, 24, 24, &options)
                .unwrap();
            if let Some(reference) = &reference {
                let error: usize = reference
                    .iter()
                    .zip(&frame.rgba)
                    .map(|(a, b)| usize::from(a.abs_diff(*b)))
                    .sum();
                assert!(
                    error <= frame.rgba.len(),
                    "mean byte error <= 1, offset {offset}, error {error}"
                );
                assert_eq!(
                    reference.chunks_exact(4).filter(|p| p[0] > 0).count(),
                    frame.rgba.chunks_exact(4).filter(|p| p[0] > 0).count()
                );
            } else {
                reference = Some(frame.rgba);
            }
        }
    }
}
