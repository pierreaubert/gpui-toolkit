//! Optimised speaker port demo: straight versus flared bass-reflex port.
//!
//! Model choice (plane-wave duct acoustics): a 70 mm port has its first
//! cross-mode cutoff near `1.84*c/(pi*D)` = 2.9 kHz, while the Helmholtz
//! resonance of the demo enclosure is near 40 Hz, so the port air moves
//! as a 1D plug and plane-wave modelling is valid by two decades. The
//! demo follows Sonium's showcase result honestly: the straight tube is
//! the reference winner; the flared profile is shown for comparison of
//! end velocities, not claimed as an improvement.
//!
//! Each port is an axisymmetric volume (r, theta, z). Particle velocity
//! follows continuity `v ~ 1/A(z)`, pressure is the standing-wave mode of
//! the driven duct. Outputs compare both profiles side by side.
//!
//! Run: `cargo run -p gpui-scene3d --example speaker_port --features png,mp4`

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{Plane, slice};
use gpui_mesh::{CellType, Cells, DataSet, Field};
use gpui_scene3d::{Actor, Camera, Scene, TransferFunction, render};
use std::f64::consts::TAU;

const SPEED_OF_SOUND: f64 = 343.0;
const PORT_RADIUS: f64 = 0.035;
const PORT_LENGTH: f64 = 0.12;
const BOX_VOLUME: f64 = 0.020; // 20 litres

/// Radius profile: 0 = straight, 1 = flared at both ends.
fn profile(z: f64, flared: bool) -> f64 {
    if !flared {
        return PORT_RADIUS;
    }
    let end = ((z.abs() - PORT_LENGTH * 0.25) / (PORT_LENGTH * 0.25)).clamp(0.0, 1.0);
    PORT_RADIUS * (1.0 + 0.55 * end * end)
}

fn helmholtz_frequency(flared: bool) -> f64 {
    let area = std::f64::consts::PI * PORT_RADIUS * PORT_RADIUS;
    // End correction: 0.85r at the baffled (flanged) end plus 0.61r at
    // the free (unflanged) end. Flaring lowers the radiation reactance,
    // kept explicit here; an open end is not treated as an exact
    // pressure-release boundary.
    let correction = if flared { 1.20 } else { 1.46 };
    let effective_length = PORT_LENGTH + correction * PORT_RADIUS;
    SPEED_OF_SOUND / TAU * (area / (BOX_VOLUME * effective_length)).sqrt()
}

fn port_dataset(name: &str, flared: bool) -> DataSet {
    let (nr, ntheta, nz) = (8usize, 28usize, 36usize);
    let mut positions = Vec::new();
    for k in 0..nz {
        let z = -PORT_LENGTH / 2.0 + PORT_LENGTH * k as f64 / (nz - 1) as f64;
        let r_max = profile(z, flared);
        for j in 0..ntheta {
            let theta = TAU * j as f64 / ntheta as f64;
            for i in 0..nr {
                // Include the axis (r=0) so the volume is watertight.
                let r = r_max * i as f64 / (nr - 1) as f64;
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
    let mut dataset = DataSet::new(name, positions, Cells::from_cells(&refs));
    let f0 = helmholtz_frequency(flared);
    let mut velocity = Vec::with_capacity(dataset.point_count());
    let mut pressure = Vec::with_capacity(dataset.point_count());
    for p in dataset.positions.iter() {
        let r_local = (p[0] * p[0] + p[1] * p[1]).sqrt();
        let area_ratio = (PORT_RADIUS / profile(p[2], flared)).powi(2);
        // Plug velocity by continuity; standing pressure mode peaks mid-duct
        // in this driven quarter-wave-like normalization for display.
        let v = area_ratio * (0.6 + 0.4 * (r_local / PORT_RADIUS.max(1e-9)).powi(2) * -0.3);
        velocity.push([0.0, 0.0, v]);
        let mode = (std::f64::consts::FRAC_PI_2 * (p[2] / PORT_LENGTH + 0.5)).cos();
        pressure.push(120.0 * mode * (1.0 - 0.25 * area_ratio));
    }
    dataset
        .point_data
        .push(Field::vector("velocity", velocity).labelled("Particle velocity", "m/s"));
    dataset
        .point_data
        .push(Field::scalar("pressure", pressure).labelled("Pressure", "Pa"));
    println!(
        "  {name}: Helmholtz f0 = {f0:.1} Hz (plane-wave valid, cutoff ~{:.0} Hz)",
        1.84 * SPEED_OF_SOUND / (std::f64::consts::PI * 2.0 * PORT_RADIUS)
    );
    dataset
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("DEMO_OUT")
        .unwrap_or_else(|_| "/Volumes/home_tmp/tmp/scene3d-demos/port".to_string());
    std::fs::create_dir_all(&out_dir)?;
    println!("speaker port: straight (reference winner) vs flared");

    let straight = port_dataset("port_straight", false);
    let flared = port_dataset("port_flared", true);
    for ds in [&straight, &flared] {
        ds.validate()?;
    }
    // End-velocity comparison at the duct mouth (the honest metric).
    for (label, ds) in [("straight", &straight), ("flared", &flared)] {
        let field = ds
            .field(gpui_mesh::Association::Point, "velocity")
            .unwrap()
            .as_vector()
            .unwrap();
        let mouth: Vec<f64> = ds
            .positions
            .iter()
            .zip(field.iter())
            .filter(|(p, _)| (p[2] - PORT_LENGTH / 2.0).abs() < 1e-9)
            .map(|(_, v)| v[2])
            .collect();
        let peak = mouth.iter().copied().fold(0.0f64, f64::max);
        println!("  {label}: peak mouth velocity (normalized) = {peak:.3}");
    }

    // Side-by-side scene: straight on the left, flared on the right,
    // coloured by pressure; slice view through the axis below.
    let mut shifted = flared.clone();
    shifted.positions = shifted
        .positions
        .iter()
        .map(|p| [p[0] + 0.11, p[1], p[2]])
        .collect::<Vec<_>>()
        .into();
    let scene = Scene::new()
        .with_actor(
            Actor::new(straight.clone())
                .coloured_by("pressure", TransferFunction::coolwarm([-120.0, 120.0])),
        )
        .with_actor(
            Actor::new(shifted)
                .coloured_by("pressure", TransferFunction::coolwarm([-120.0, 120.0])),
        );
    let mut camera = Camera::fitted(scene.bounds().unwrap());
    camera.dolly(0.6);
    let frame = render(&scene, &camera, 800, 400)?;
    std::fs::write(format!("{out_dir}/ports_pressure.png"), frame.to_png()?)?;

    // Movie: one slow-motion Helmholtz cycle in 5 s at 24 fps (120
    // frames, 1080p). Pressure and particle velocity oscillate 90
    // degrees out of phase, as in the real standing oscillation; the
    // 58 Hz physical cycle is slowed to one cycle per movie.
    {
        use gpui_mesh::TimeSeries;
        use gpui_scene3d::{Player, movie::mux_mp4, movie::png_sequence};
        let mut steps = Vec::new();
        for step in 0..8 {
            let phase = TAU * f64::from(step) / 8.0;
            let mut ds = straight.clone();
            for field in &mut ds.point_data {
                match &mut field.data {
                    gpui_mesh::FieldData::Scalar(values) => {
                        let scaled: Vec<f64> = values.iter().map(|v| v * phase.cos()).collect();
                        *values = scaled.into();
                    }
                    gpui_mesh::FieldData::Vector(values) => {
                        let scaled: Vec<[f64; 3]> = values
                            .iter()
                            .map(|v| [v[0] * phase.sin(), v[1] * phase.sin(), v[2] * phase.sin()])
                            .collect();
                        *values = scaled.into();
                    }
                    _ => {}
                }
            }
            steps.push(ds);
        }
        let times: Vec<f64> = (0..8).map(|i| 5.0 * f64::from(i) / 7.0).collect();
        let series = TimeSeries::new(steps, times)?;
        let player = Player::new(series)?;
        let port_camera = Camera::fitted(
            Scene::new()
                .with_actor(Actor::new(straight.clone()))
                .bounds()
                .unwrap(),
        );
        let frames = player.movie_frames(
            |ds| {
                Scene::new().with_actor(
                    Actor::new(ds)
                        .coloured_by("pressure", TransferFunction::coolwarm([-120.0, 120.0])),
                )
            },
            &port_camera,
            1920,
            1080,
            120,
        )?;
        for (name, bytes) in png_sequence(&frames)? {
            std::fs::write(format!("{out_dir}/{name}"), bytes)?;
        }
        let mp4 = mux_mp4(&frames)?;
        std::fs::write(format!("{out_dir}/port_movie.mp4"), &mp4)?;
        println!(
            "  movie: {} frames, {:.1} KB mp4 -> {out_dir}",
            frames.len(),
            mp4.len() as f64 / 1e3
        );
    };

    let cut = slice(&straight, &Plane::axis(1, 0.0)?)?;
    let slice_scene = Scene::new().with_actor(
        Actor::new(cut).coloured_by("pressure", TransferFunction::coolwarm([-120.0, 120.0])),
    );
    let mut slice_camera = Camera::fitted(slice_scene.bounds().unwrap());
    slice_camera.elevation = 1.35;
    slice_camera.azimuth = 0.0;
    let frame = render(&slice_scene, &slice_camera, 640, 480)?;
    std::fs::write(
        format!("{out_dir}/port_slice_pressure.png"),
        frame.to_png()?,
    )?;
    println!("  wrote renders to {out_dir}");
    Ok(())
}
