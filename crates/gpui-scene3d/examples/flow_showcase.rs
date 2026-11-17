//! Flow-visualisation showcase: every wave of the VTK-class feature set
//! on one analytic Taylor-Green vortex field.
//!
//! Outputs (all under `FLOW_OUT`, default `/Volumes/home_tmp/tmp/flow-showcase`):
//! - `hero.png` (1080p): lambda-2 vortex cores, streamtubes, solid 3D
//!   arrows on a slice, colour bar and axis triad.
//! - `slice.png` (1080p): LIC streaks on a slice with banded filled
//!   contours and contour lines of pressure.
//! - `tensors.png`: strain-rate tensor ellipsoid glyphs.
//! - `particles/`: 48-frame PNG sequence (+ MP4 with system ffmpeg) of
//!   advected particles with fading trails in a clip box.
//!
//! Run: `cargo run -p gpui-scene3d --example flow_showcase --features png,mp4`

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{
    ParticleOptions, Plane, contour_lines, dashed_lines, isosurface, lambda2, lic, particle_frames,
    slice, solid_arrows, stream_ribbons, streamlines, streamtubes, swirling_strength,
    tensor_glyphs,
};
use gpui_mesh::{DataSet, Field, TimeSeries};
use gpui_scene3d::{
    Actor, Camera, ClipBox, ColorBar, Rect, Scene, TransferFunction, draw_axis_triad,
    draw_colorbar, layout_colorbar, movie::mux_mp4, movie::png_sequence, render,
};
use std::f64::consts::TAU;

const SIZE: f64 = 6.0;

/// Taylor-Green vortex at phase `phase` on an n^3 grid over [0, 6]^3.
fn taylor_green(n: usize, phase: f64) -> DataSet {
    let step = SIZE / (n - 1) as f64;
    let mut ds =
        DataSet::structured_grid("taylor-green", [n, n, n], [0.0; 3], [step; 3]).expect("grid");
    let mut velocity = Vec::new();
    let mut pressure = Vec::new();
    let mut strain = Vec::new();
    for p in ds.positions.iter() {
        let (x, y, z) = (p[0] + phase, p[1], p[2]);
        let (sx, cx) = (x.sin(), x.cos());
        let (sy, cy) = (y.sin(), y.cos());
        let (sz, cz) = (z.sin(), z.cos());
        let u = sx * cy * cz;
        let v = -cx * sy * cz;
        velocity.push([u, v, 0.0]);
        pressure.push(0.25 * ((2.0 * x).cos() + (2.0 * y).cos()) * (2.0 + (2.0 * z).cos()));
        // Analytic strain-rate tensor [xx, yy, zz, xy, yz, xz].
        strain.push([
            cx * cy * cz,
            -cx * cy * cz,
            0.0,
            f64::midpoint(-sx * sy * cz, sx * sy * cz),
            0.5 * (cx * sy * sz),
            0.5 * (-sx * cy * sz),
        ]);
    }
    ds.point_data.push(Field::vector("velocity", velocity));
    ds.point_data.push(Field::scalar("pressure", pressure));
    ds.point_data.push(Field::tensor("strain", strain));
    ds
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("FLOW_OUT")
        .unwrap_or_else(|_| "/Volumes/home_tmp/tmp/flow-showcase".to_string());
    std::fs::create_dir_all(format!("{out_dir}/particles"))?;
    let dataset = taylor_green(20, 0.0);
    let bounds = dataset.bounds().unwrap();
    let camera = Camera::fitted(bounds);
    let with_cores = lambda2(&dataset, "velocity", "lambda2")?;
    let with_swirling = swirling_strength(&with_cores, "velocity", "swirling")?;
    let min_lambda = with_swirling
        .field(gpui_mesh::Association::Point, "lambda2")
        .unwrap()
        .as_scalar()
        .unwrap()
        .iter()
        .copied()
        .fold(f64::MAX, f64::min);
    let cores = isosurface(&with_swirling, "lambda2", min_lambda * 0.3)?;
    println!("flow_showcase: cores {} triangles", cores.cell_count());

    // Hero: cores + tubes + ribbons + solid arrows + dashes, legend, triad.
    let mut seeds = Vec::new();
    for i in 0..10 {
        let a = TAU * f64::from(i) / 10.0;
        seeds.push([3.0 + 1.8 * a.cos(), 3.0 + 1.8 * a.sin(), 3.0]);
    }
    let lines = streamlines(&dataset, "velocity", &seeds, 0.12, 220)?;
    let tubes = streamtubes(&lines, 0.035, 6)?;
    let ribbons = stream_ribbons(&lines, 0.22)?;
    let cut = slice(&dataset, &Plane::axis(2, 3.0)?)?;
    let arrows = solid_arrows(&cut, "velocity", 0.95, 2, 0.055)?;
    let dashes = dashed_lines(&lines, 0.0, 0.35, 0.3)?;
    let pressure_tf = TransferFunction::coolwarm([-1.6, 1.6]);
    let hero = Scene::new()
        .with_actor(
            Actor::new(cores.clone())
                .coloured_by("pressure", pressure_tf.clone())
                .opacity(0.9),
        )
        .with_actor(Actor::new(tubes).color([0.95, 0.8, 0.3, 1.0]))
        .with_actor(Actor::new(ribbons).color([0.4, 0.75, 0.95, 0.55]))
        .with_actor(
            Actor::new(arrows).coloured_by("magnitude", TransferFunction::viridis([0.0, 1.2])),
        )
        .with_actor(Actor::new(dashes).color([1.0, 1.0, 1.0, 0.9]))
        .with_annotation(gpui_scene3d::Annotation {
            text: "Taylor-Green vortex: lambda-2 cores, streamtubes, solid arrows".into(),
            x: 0.02,
            y: 0.03,
        });
    let mut frame = render(&hero, &camera, 1920, 1080)?;
    let bar = ColorBar::new(&pressure_tf, "Pressure", Some("Pa".into()));
    let layout = layout_colorbar(
        &bar,
        &pressure_tf,
        Rect {
            x: 1830,
            y: 340,
            width: 70,
            height: 400,
        },
    );
    draw_colorbar(&mut frame, &pressure_tf, &layout);
    draw_axis_triad(&mut frame, &camera, (90, 990), 55);
    std::fs::write(format!("{out_dir}/hero.png"), frame.to_png()?)?;
    println!(
        "  hero.png written (annotations: {})",
        hero.annotations.len()
    );

    // Slice: LIC streaks + banded pressure contours + contour lines.
    let with_lic = lic(&dataset, "velocity", 18, 0.14, 42)?;
    let lic_cut = slice(&with_lic, &Plane::axis(2, 3.0)?)?;
    let levels: Vec<f64> = (-5..=5).map(|i| f64::from(i) * 0.3).collect();
    let contours = contour_lines(&lic_cut, "pressure", &levels)?;
    let slice_scene = Scene::new()
        .with_actor(
            Actor::new(lic_cut.clone()).coloured_by("lic", TransferFunction::viridis([0.15, 0.85])),
        )
        .with_actor(Actor::new(contours).color([0.05, 0.05, 0.08, 1.0]));
    let mut top = Camera::fitted(lic_cut.bounds().unwrap());
    top.elevation = 0.0;
    top.azimuth = 0.0;
    let frame = render(&slice_scene, &top, 1920, 1080)?;
    std::fs::write(format!("{out_dir}/slice.png"), frame.to_png()?)?;

    // Filled contours: same slice, banded pressure + lines.
    let banded_scene = Scene::new()
        .with_actor(Actor::new(lic_cut).coloured_by("pressure", pressure_tf.clone().banded(8)))
        .with_actor(
            Actor::new(contour_lines(&cut, "pressure", &levels)?).color([1.0, 1.0, 1.0, 0.8]),
        );
    let frame = render(&banded_scene, &top, 1920, 1080)?;
    std::fs::write(format!("{out_dir}/contours.png"), frame.to_png()?)?;

    // Tensor ellipsoids on a coarse slice.
    let glyphs = tensor_glyphs(&cut, "strain", 0.5, 6)?;
    let tensor_scene = Scene::new()
        .with_actor(Actor::new(cut.clone()).coloured_by("pressure", pressure_tf.clone()))
        .with_actor(
            Actor::new(glyphs).coloured_by("magnitude", TransferFunction::viridis([0.0, 1.0])),
        );
    let frame = render(&tensor_scene, &top, 1920, 1080)?;
    std::fs::write(format!("{out_dir}/tensors.png"), frame.to_png()?)?;

    // Particles movie: unsteady phases, clip box, fading trails.
    let steps: Vec<DataSet> = (0..13)
        .map(|i| taylor_green(16, 0.35 * f64::from(i)))
        .collect();
    let times: Vec<f64> = (0..13).map(|i| 0.5 * f64::from(i)).collect();
    let series = TimeSeries::new(steps, times)?;
    let mut particle_seeds = Vec::new();
    for j in 0..5 {
        for i in 0..5 {
            particle_seeds.push([0.4, 0.8 + 1.1 * f64::from(i), 0.8 + 1.1 * f64::from(j)]);
        }
    }
    let frames_ds = particle_frames(
        &series,
        "velocity",
        &particle_seeds,
        ParticleOptions {
            dt: 0.125,
            frames: 48,
            trail: 20,
        },
    )?;
    let clip = ClipBox {
        min: [0.4, 0.4, 0.4],
        max: [5.6, 5.6, 5.6],
    };
    let mut timed = Vec::new();
    for (i, ds) in frames_ds.into_iter().enumerate() {
        let scene = Scene::new()
            .with_actor(
                Actor::new(cores.clone())
                    .coloured_by("pressure", pressure_tf.clone())
                    .opacity(0.35)
                    .clip_box(clip),
            )
            .with_actor(Actor::new(ds).coloured_by("age", TransferFunction::viridis([0.0, 1.0])));
        let frame = render(&scene, &camera, 1280, 720)?;
        timed.push((f64::from(i as u32) / 24.0, frame));
    }
    for (name, bytes) in png_sequence(&timed)? {
        std::fs::write(format!("{out_dir}/particles/{name}"), bytes)?;
    }
    match mux_mp4(&timed) {
        Ok(mp4) => {
            std::fs::write(format!("{out_dir}/particles.mp4"), &mp4)?;
            println!(
                "  particles.mp4: {} frames, {:.0} KB",
                timed.len(),
                mp4.len() as f64 / 1024.0
            );
        }
        Err(e) => println!("  mp4 skipped: {e}"),
    }
    println!("flow_showcase done -> {out_dir}");
    Ok(())
}
