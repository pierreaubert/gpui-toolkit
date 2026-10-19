//! PC-fan airflow demo: swirling through-flow through a 120 mm fan.
//!
//! Data is generated at demo scale from the same model family as
//! Sonium's `sonium-sim-pcfan`: a 120 mm frame, 25 mm depth, 18 mm hub
//! radius, 54 mm blade-tip radius (`PcFanConfig::default_120mm`), at
//! 1500 rpm. The field is an actuator-style mean flow: uniform axial
//! through-flow, solid-body swirl added across the rotor disc and
//! decaying downstream, pressure from radial equilibrium. It is not the
//! production CFD solution; it exercises the visualization pipeline
//! (volume colouring, plane slice, streamlines, vorticity, movie) on
//! fan-shaped data with the production geometry.
//!
//! Run: `cargo run -p gpui-scene3d --example pcfan_airflow --features png,mp4`

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{Plane, q_criterion, slice, streamlines, vorticity};
use gpui_mesh::{CellType, Cells, DataSet, Field, TimeSeries};
use gpui_scene3d::{
    Actor, Camera, Player, Scene, TransferFunction, VolumeOptions, movie::mux_mp4,
    movie::png_sequence, raycast_volume, render,
};
use std::f64::consts::TAU;

const HUB_R: f64 = 0.018;
const TIP_R: f64 = 0.054;
const ROTOR_Z: [f64; 2] = [0.0, 0.025];
const RPM: f64 = 1500.0;
const AXIAL_SPEED: f64 = 3.2;
const RHO: f64 = 1.2;

/// Cylindrical grid (r from hub to shroud) with a wrapped theta seam.
fn fan_dataset(phase: f64) -> DataSet {
    let (nr, ntheta, nz) = (10usize, 36usize, 28usize);
    let (z_min, z_max) = (-0.05, 0.10);
    let mut positions = Vec::new();
    for k in 0..nz {
        let z = z_min + (z_max - z_min) * k as f64 / (nz - 1) as f64;
        for j in 0..ntheta {
            let theta = TAU * j as f64 / ntheta as f64;
            for i in 0..nr {
                let r = HUB_R + (TIP_R - HUB_R) * i as f64 / (nr - 1) as f64;
                positions.push([r * theta.cos(), r * theta.sin(), z]);
            }
        }
    }
    let idx = |i: usize, j: usize, k: usize| (k * ntheta * nr + (j % ntheta) * nr + i) as u32;
    let mut cells: Vec<(CellType, [u32; 8])> = Vec::new();
    for k in 0..nz - 1 {
        for j in 0..ntheta {
            for i in 0..nr - 1 {
                cells.push((
                    CellType::Hexahedron,
                    [
                        idx(i, j, k),
                        idx(i + 1, j, k),
                        idx(i + 1, j + 1, k),
                        idx(i, j + 1, k),
                        idx(i, j, k + 1),
                        idx(i + 1, j, k + 1),
                        idx(i + 1, j + 1, k + 1),
                        idx(i, j + 1, k + 1),
                    ],
                ));
            }
        }
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    let mut dataset = DataSet::new("pcfan", positions, Cells::from_cells(&refs));

    let omega = RPM * TAU / 60.0;
    let mut velocity = Vec::with_capacity(dataset.point_count());
    let mut pressure = Vec::with_capacity(dataset.point_count());
    for p in dataset.positions.iter() {
        let r = (p[0] * p[0] + p[1] * p[1]).sqrt();
        let theta = p[1].atan2(p[0]);
        // Swirl ramps across the rotor and decays exponentially downstream.
        let swirl_fraction = if p[2] < ROTOR_Z[0] {
            0.0
        } else if p[2] <= ROTOR_Z[1] {
            (p[2] - ROTOR_Z[0]) / (ROTOR_Z[1] - ROTOR_Z[0])
        } else {
            (-(p[2] - ROTOR_Z[1]) / 0.05).exp()
        };
        // Blade-passage ripple rotates with the phase (for the movie).
        let ripple = 1.0 + 0.18 * (9.0 * (theta - phase) - p[2] * 40.0).sin() * swirl_fraction;
        let v_theta = 0.38 * omega * r * swirl_fraction * ripple;
        let v_z = AXIAL_SPEED * (1.0 - 0.25 * ((r - HUB_R) / (TIP_R - HUB_R)));
        velocity.push([-v_theta * theta.sin(), v_theta * theta.cos(), v_z]);
        // Radial equilibrium for solid-body-like swirl: p grows with r.
        pressure.push(-40.0 + 0.5 * RHO * v_theta * v_theta + 900.0 * (p[2] + 0.05));
    }
    dataset
        .point_data
        .push(Field::vector("velocity", velocity).labelled("Velocity", "m/s"));
    dataset
        .point_data
        .push(Field::scalar("pressure", pressure).labelled("Pressure", "Pa"));
    dataset
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("DEMO_OUT")
        .unwrap_or_else(|_| "/Volumes/home_tmp/tmp/scene3d-demos/pcfan".to_string());
    std::fs::create_dir_all(&out_dir)?;

    let dataset = fan_dataset(0.0);
    dataset.validate()?;
    println!(
        "pcfan: {} points, {} cells, {:.1} MB",
        dataset.point_count(),
        dataset.cell_count(),
        dataset.byte_len() as f64 / 1e6
    );
    // Mass conservation check: mean axial velocity at inlet and outlet.
    for (label, z) in [("inlet", -0.05), ("outlet", 0.10)] {
        let values: Vec<f64> = dataset
            .positions
            .iter()
            .zip(
                dataset
                    .field(gpui_mesh::Association::Point, "velocity")
                    .unwrap()
                    .as_vector()
                    .unwrap(),
            )
            .filter(|(p, _)| (p[2] - z).abs() < 1e-9)
            .map(|(_, v)| v[2])
            .collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        println!("  mean axial velocity at {label}: {mean:.3} m/s");
    }

    // Diagnostics from the toolkit calculators.
    let with_vorticity = vorticity(&dataset, "velocity", "vorticity")?;
    let with_q = q_criterion(&with_vorticity, "velocity", "q_criterion")?;
    let max_q = with_q
        .field(gpui_mesh::Association::Point, "q_criterion")
        .unwrap()
        .as_scalar()
        .unwrap()
        .iter()
        .copied()
        .fold(f64::MIN, f64::max);
    println!("  max Q-criterion downstream of rotor: {max_q:.1} 1/s^2");

    // 1) Surface coloured by pressure.
    let scene = Scene::new().with_actor(
        Actor::new(dataset.clone())
            .coloured_by("pressure", TransferFunction::coolwarm([-40.0, 160.0])),
    );
    let camera = Camera::fitted(scene.bounds().unwrap());
    let frame = render(&scene, &camera, 640, 480)?;
    std::fs::write(format!("{out_dir}/fan_pressure.png"), frame.to_png()?)?;

    // 2) Axial slice coloured by pressure (the showcase's classic view).
    let cut = slice(&dataset, &Plane::axis(1, 0.0)?)?;
    let slice_scene = Scene::new().with_actor(
        Actor::new(cut).coloured_by("pressure", TransferFunction::coolwarm([-40.0, 160.0])),
    );
    // Looking down on the x-z slice plane (normal +y).
    let mut slice_camera = Camera::fitted(slice_scene.bounds().unwrap());
    slice_camera.elevation = 1.35;
    slice_camera.azimuth = 0.0;
    let frame = render(&slice_scene, &slice_camera, 640, 480)?;
    std::fs::write(format!("{out_dir}/fan_slice_pressure.png"), frame.to_png()?)?;

    // 3) Volume raycast coloured by Q (vortex cores downstream).
    let frame = raycast_volume(
        &with_q,
        "q_criterion",
        &TransferFunction::viridis([0.0, max_q.max(1.0)]),
        &camera,
        480,
        360,
        &VolumeOptions::default(),
    )?;
    std::fs::write(format!("{out_dir}/fan_volume_q.png"), frame.to_png()?)?;

    // 4) Streamlines from an inlet ring, saved as a VTU-style summary render.
    let mut seeds = Vec::new();
    for i in 0..12 {
        let theta = TAU * f64::from(i) / 12.0;
        seeds.push([0.035 * theta.cos(), 0.035 * theta.sin(), -0.045]);
    }
    let lines = streamlines(&dataset, "velocity", &seeds, 0.004, 400)?;
    println!("  streamlines: {} segments", lines.cell_count());
    let lines_scene = Scene::new()
        .with_actor(
            Actor::new(dataset.clone())
                .coloured_by("pressure", TransferFunction::coolwarm([-40.0, 160.0]))
                .opacity(0.35),
        )
        .with_actor(Actor::new(lines).color([1.0, 1.0, 1.0, 1.0]));
    let frame = render(&lines_scene, &camera, 640, 480)?;
    std::fs::write(format!("{out_dir}/fan_streamlines.png"), frame.to_png()?)?;

    // 5) Movie: 5 seconds at 24 fps (120 frames, 1080p). The series
    // holds 12 ripple phases over 5 s; the player interpolates between
    // them, so the exported rotation is smooth.
    let steps: Vec<DataSet> = (0..12)
        .map(|i| fan_dataset(TAU * f64::from(i) / 12.0))
        .collect();
    let times: Vec<f64> = (0..12).map(|i| 5.0 * f64::from(i) / 11.0).collect();
    let series = TimeSeries::new(steps, times)?;
    let player = Player::new(series)?;
    let frames = player.movie_frames(
        |ds| {
            Scene::new().with_actor(
                Actor::new(ds).coloured_by("pressure", TransferFunction::coolwarm([-40.0, 160.0])),
            )
        },
        &camera,
        1920,
        1080,
        120,
    )?;
    for (name, bytes) in png_sequence(&frames)? {
        std::fs::write(format!("{out_dir}/{name}"), bytes)?;
    }
    let mp4 = mux_mp4(&frames)?;
    std::fs::write(format!("{out_dir}/fan_movie.mp4"), &mp4)?;
    println!(
        "  movie: {} frames, {:.1} KB mp4 -> {out_dir}",
        frames.len(),
        mp4.len() as f64 / 1e3
    );
    Ok(())
}
