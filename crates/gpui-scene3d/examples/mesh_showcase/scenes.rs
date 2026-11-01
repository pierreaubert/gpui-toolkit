//! Analytic datasets and scene presets for the mesh showcase.
//!
//! Everything here is procedural: the Taylor-Green vortex field is built in
//! memory, so the demo runs identically on native targets and wasm with no
//! file or network loading.

use super::mesh_section::MeshSection;
use gpui_dataset_transformation::{
    Plane, contour_lines, dashed_lines, isosurface, lambda2, slice, solid_arrows, stream_ribbons,
    streamlines, streamtubes, swirling_strength,
};
use gpui_mesh::{DataSet, Field};
use gpui_scene3d::{Actor, Camera, Scene, TransferFunction};
use std::f64::consts::TAU;

/// Grid points per axis for the showcase field.
///
/// 20 keeps streamline tracing (the slowest pipeline stage) fast enough
/// for one-shot page loads on wasm.
const GRID_N: usize = 20;

/// Grid points per axis for surface geometry (isosurfaces, slices).
///
/// 32 makes shells and cuts look smooth. Only cheap per-point filters run
/// on this grid; tracing stays on `GRID_N`.
const FINE_GRID_N: usize = 32;

/// Edge length of the cubic showcase domain.
const SIZE: f64 = 6.0;

/// Taylor-Green vortex on an `n`^3 grid over `[0, SIZE]^3`, with
/// velocity, pressure, and strain-rate tensor point fields.
fn taylor_green(n: usize) -> DataSet {
    let step = SIZE / (n - 1) as f64;
    let mut ds = DataSet::structured_grid("taylor-green", [n, n, n], [0.0; 3], [step; 3])
        .expect("analytic showcase grid is valid");
    let mut velocity = Vec::new();
    let mut pressure = Vec::new();
    let mut strain = Vec::new();
    for p in ds.positions.iter() {
        let (x, y, z) = (p[0], p[1], p[2]);
        let (sx, cx) = (x.sin(), x.cos());
        let (sy, cy) = (y.sin(), y.cos());
        let (sz, cz) = (z.sin(), z.cos());
        velocity.push([sx * cy * cz, -cx * sy * cz, 0.0]);
        pressure.push(0.25 * ((2.0 * x).cos() + (2.0 * y).cos()) * (2.0 + (2.0 * z).cos()));
        // Analytic strain-rate tensor [xx, yy, zz, xy, yz, xz].
        strain.push([
            cx * cy * cz,
            -cx * cy * cz,
            0.0,
            0.5 * (-sx * sy * cz + sx * sy * cz),
            0.5 * (cx * sy * sz),
            0.5 * (-sx * cy * sz),
        ]);
    }
    ds.point_data.push(Field::vector("velocity", velocity));
    ds.point_data.push(Field::scalar("pressure", pressure));
    ds.point_data.push(Field::tensor("strain", strain));
    ds
}

/// Pressure contour levels shared by the slice and contour scenes.
fn pressure_levels() -> Vec<f64> {
    (-5..=5).map(|i| f64::from(i) * 0.3).collect()
}

/// Vortex cores, streamtubes, ribbons, and solid arrows (hero scene).
fn vortices_scene() -> Result<(Scene, Camera), String> {
    // Cores stay on the coarse grid: finer isosurfaces facet-sparkle under
    // flat shading, while the coarse shells shade smoothly. Tracing, tubes,
    // and arrows share the same grid, so everything stays consistent.
    let dataset = taylor_green(GRID_N);
    let with_cores = lambda2(&dataset, "velocity", "lambda2").map_err(|e| e.to_string())?;
    let with_swirling =
        swirling_strength(&with_cores, "velocity", "swirling").map_err(|e| e.to_string())?;
    let min_lambda = with_swirling
        .field(gpui_mesh::Association::Point, "lambda2")
        .and_then(|field| field.as_scalar())
        .map(|values| values.iter().copied().fold(f64::MAX, f64::min))
        .unwrap_or(0.0);
    let cores =
        isosurface(&with_swirling, "lambda2", min_lambda * 0.3).map_err(|e| e.to_string())?;

    let mut seeds = Vec::new();
    for i in 0..10 {
        let angle = TAU * i as f64 / 10.0;
        seeds.push([3.0 + 1.8 * angle.cos(), 3.0 + 1.8 * angle.sin(), 3.0]);
    }
    let lines = streamlines(&dataset, "velocity", &seeds, 0.12, 160).map_err(|e| e.to_string())?;
    let tubes = streamtubes(&lines, 0.035, 5).map_err(|e| e.to_string())?;
    let ribbons = stream_ribbons(&lines, 0.22).map_err(|e| e.to_string())?;
    let cut = slice(&dataset, &Plane::axis(2, 3.0).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let arrows = solid_arrows(&cut, "velocity", 0.95, 2, 0.055).map_err(|e| e.to_string())?;
    let dashes = dashed_lines(&lines, 0.0, 0.35, 0.3).map_err(|e| e.to_string())?;

    let pressure = TransferFunction::coolwarm([-1.6, 1.6]);
    let scene = Scene::new()
        .with_actor(
            Actor::new(cores)
                .coloured_by("pressure", pressure.clone())
                .opacity(0.9),
        )
        .with_actor(Actor::new(tubes).color([0.95, 0.8, 0.3, 1.0]))
        .with_actor(Actor::new(ribbons).color([0.4, 0.75, 0.95, 0.55]))
        .with_actor(
            Actor::new(arrows).coloured_by("magnitude", TransferFunction::viridis([0.0, 1.2])),
        )
        .with_actor(Actor::new(dashes).color([1.0, 1.0, 1.0, 0.9]));
    let camera = scene.bounds().map(Camera::fitted).unwrap_or_default();
    Ok((scene, camera))
}

/// Top-down camera over slice bounds.
fn top_camera(dataset: &DataSet) -> Camera {
    let mut camera = dataset.bounds().map(Camera::fitted).unwrap_or_default();
    camera.azimuth = 0.0;
    camera.elevation = 0.0;
    camera
}

/// Mid-plane slice: pressure surface with solid velocity arrows and
/// pressure contour lines.
fn slice_scene() -> Result<(Scene, Camera), String> {
    let fine = taylor_green(FINE_GRID_N);
    let cut = slice(&fine, &Plane::axis(2, 3.0).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    // Stride 5 keeps ~200 arrows on the 32^2 slice, the density that reads
    // best against the contour lines.
    let arrows = solid_arrows(&cut, "velocity", 0.95, 5, 0.055).map_err(|e| e.to_string())?;
    let lines = contour_lines(&cut, "pressure", &pressure_levels()).map_err(|e| e.to_string())?;
    let scene = Scene::new()
        .with_actor(
            Actor::new(cut.clone())
                .coloured_by("pressure", TransferFunction::coolwarm([-1.6, 1.6])),
        )
        .with_actor(
            Actor::new(arrows).coloured_by("magnitude", TransferFunction::viridis([0.0, 1.2])),
        )
        .with_actor(Actor::new(lines).color([0.05, 0.05, 0.08, 1.0]));
    Ok((scene, top_camera(&cut)))
}

/// Banded filled pressure contours with contour lines on the slice.
fn contours_scene() -> Result<(Scene, Camera), String> {
    let fine = taylor_green(FINE_GRID_N);
    let cut = slice(&fine, &Plane::axis(2, 3.0).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let pressure = TransferFunction::coolwarm([-1.6, 1.6]);
    let lines = contour_lines(&cut, "pressure", &pressure_levels()).map_err(|e| e.to_string())?;
    let scene = Scene::new()
        .with_actor(Actor::new(cut.clone()).coloured_by("pressure", pressure.banded(8)))
        .with_actor(Actor::new(lines).color([1.0, 1.0, 1.0, 0.8]));
    Ok((scene, top_camera(&cut)))
}

/// Symmetric pressure isosurface shells, coloured by pressure.
fn shells_scene() -> Result<(Scene, Camera), String> {
    let fine = taylor_green(FINE_GRID_N);
    let hi = isosurface(&fine, "pressure", 0.9).map_err(|e| e.to_string())?;
    let lo = isosurface(&fine, "pressure", -0.9).map_err(|e| e.to_string())?;
    let pressure = TransferFunction::coolwarm([-1.6, 1.6]);
    let scene = Scene::new()
        .with_actor(Actor::new(hi).coloured_by("pressure", pressure.clone()))
        .with_actor(Actor::new(lo).coloured_by("pressure", pressure));
    let camera = fine.bounds().map(Camera::fitted).unwrap_or_default();
    Ok((scene, camera))
}

/// Builds the scene and home camera for a showcase section.
///
/// The field is rebuilt per section so captures (`?section=...`) only pay
/// for the scene they show; each build is deterministic.
pub(super) fn build_scene(section: MeshSection) -> Result<(Scene, Camera), String> {
    match section {
        MeshSection::Vortices => vortices_scene(),
        MeshSection::Slice => slice_scene(),
        MeshSection::Contours => contours_scene(),
        MeshSection::Shells => shells_scene(),
    }
}

#[cfg(test)]
mod scene_tests {
    use super::*;
    use std::collections::HashSet;
    use std::time::Instant;

    /// Builds every section, renders a headless frame with the software
    /// rasterizer, and stores preview PPMs for visual review. Prints per
    /// section build/render timings for wasm-load budgeting.
    #[test]
    fn renders_every_section_headlessly() {
        let out_dir = "/Volumes/home_tmp/tmp/mesh-showcase";
        std::fs::create_dir_all(out_dir).expect("preview output dir");
        for section in MeshSection::all() {
            let started = Instant::now();
            let (scene, camera) = build_scene(section).expect("showcase scene builds");
            let build_ms = started.elapsed().as_secs_f64() * 1e3;
            let started = Instant::now();
            let frame = gpui_scene3d::render(&scene, &camera, 960, 600).expect("frame renders");
            let render_ms = started.elapsed().as_secs_f64() * 1e3;
            assert_eq!((frame.width, frame.height), (960, 600));
            // PPM needs no image dependency, so this test also builds for
            // wasm targets: skip the text header, count distinct triplets.
            let ppm = frame.to_ppm();
            let mut newlines = ppm
                .iter()
                .enumerate()
                .filter(|(_, byte)| **byte == b'\n')
                .map(|(index, _)| index);
            let body = &ppm[newlines.nth(2).expect("ppm header") + 1..];
            let distinct: HashSet<[u8; 3]> = body
                .chunks_exact(3)
                .map(|pixel| [pixel[0], pixel[1], pixel[2]])
                .collect();
            // Banded fills quantize to a handful of flat colors (8 bands +
            // background + lines), so the bar sits just below that instead
            // of at a smooth-shaded hundred.
            assert!(
                distinct.len() > 8,
                "{section:?} frame looks blank ({} colors)",
                distinct.len()
            );
            std::fs::write(
                format!("{out_dir}/{}.ppm", section.label().to_ascii_lowercase()),
                &ppm,
            )
            .expect("preview ppm writes");
            eprintln!(
                "{section:?}: build {build_ms:.0} ms, render {render_ms:.0} ms, {} colors",
                distinct.len()
            );
        }
    }
}
