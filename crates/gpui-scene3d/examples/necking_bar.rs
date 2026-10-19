//! Necking bar demo: a steel bar pulled past the Considere strain.
//!
//! Finite-strain tension with plastic incompressibility: the bar first
//! stretches uniformly (axial log strain `eps`), and once `eps` passes
//! the Considere strain (0.12 for the demo steel, reaching ultimate
//! strength) deformation localizes into a Gaussian neck. Cross-section
//! positions are remapped at every step so total volume is conserved
//! numerically; the summary prints the residual volume error per step,
//! which is the demo's honesty check. Colour is axial log strain, the
//! same quantity as Sonium's showcase film.
//!
//! Run: `cargo run -p gpui-scene3d --example necking_bar --features png,mp4`

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::{Plane, slice};
use gpui_mesh::{CellType, Cells, DataSet, Field, TimeSeries};
use gpui_scene3d::{
    Actor, Camera, Player, Scene, TransferFunction, movie::mux_mp4, movie::png_sequence, render,
};
use std::f64::consts::TAU;

const LENGTH: f64 = 0.10;
const RADIUS: f64 = 0.006;
const CONSIDERE_STRAIN: f64 = 0.12;
const STEPS: usize = 16;

/// Builds one deformed state. The axial remap integrates the local
/// stretch `(R/r(x))^2` implied by incompressibility, then renormalizes
/// so the total elongation matches the imposed end displacement.
fn bar_step(step: usize) -> (DataSet, f64, f64) {
    let t = step as f64 / (STEPS - 1) as f64;
    // Uniform strain grows to 0.20; localization starts at Considere.
    let uniform_strain = 0.20 * t;
    let (depth, width) = if uniform_strain <= CONSIDERE_STRAIN {
        (0.0, LENGTH)
    } else {
        let local = (uniform_strain - CONSIDERE_STRAIN) / (0.20 - CONSIDERE_STRAIN);
        (0.55 * local, LENGTH * (0.16 - 0.10 * local))
    };
    let (nx, nr, ntheta) = (72usize, 6usize, 24usize);
    // Material stations along the reference bar. Local log strain is the
    // uniform part plus, past Considere, a Gaussian localization. Plastic
    // incompressibility gives the local axial stretch lambda = exp(eps)
    // and radius r = R * exp(-eps/2); integrating lambda yields the
    // deformed axial map with volume conserved by construction.
    let stations: Vec<f64> = (0..nx)
        .map(|i| LENGTH * i as f64 / (nx - 1) as f64)
        .collect();
    let extra_max = if depth == 0.0 {
        0.0
    } else {
        1.15 * depth / 0.55
    };
    let radius_at: Vec<f64> = stations
        .iter()
        .map(|&x| {
            let extra = extra_max * (-((x - LENGTH / 2.0) / width).powi(2)).exp();
            RADIUS * (-(uniform_strain + extra) / 2.0).exp()
        })
        .collect();
    let mut mapped = vec![0.0f64; nx];
    for i in 1..nx {
        let r_mid = f64::midpoint(radius_at[i], radius_at[i - 1]);
        let lambda = (RADIUS / r_mid).powi(2);
        mapped[i] = mapped[i - 1] + lambda * (stations[i] - stations[i - 1]);
    }

    let mut positions = Vec::new();
    let mut strain = Vec::new();
    for k in 0..nx {
        for j in 0..ntheta {
            let theta = TAU * j as f64 / ntheta as f64;
            for i in 0..nr {
                let r = radius_at[k] * i as f64 / (nr - 1) as f64;
                positions.push([mapped[k], r * theta.cos(), r * theta.sin()]);
                // Axial log strain from area change: eps = 2 ln(R_ref / r).
                let reference = RADIUS;
                strain.push(2.0 * (reference / radius_at[k].max(1e-12)).ln());
            }
        }
    }
    let idx = |i: usize, j: usize, k: usize| (k * ntheta * nr + (j % ntheta) * nr + i) as u32;
    let mut cells: Vec<(CellType, [u32; 8])> = Vec::new();
    for k in 0..nx - 1 {
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
    let mut dataset = DataSet::new(
        format!("neck_step_{step}"),
        positions,
        Cells::from_cells(&refs),
    );
    dataset
        .point_data
        .push(Field::scalar("strain", strain).labelled("Axial log strain", "-"));

    // Volume check: sum of frustum volumes vs the reference cylinder
    // stretched by the imposed displacement.
    let mut volume = 0.0;
    for k in 1..nx {
        let dx = mapped[k] - mapped[k - 1];
        volume += std::f64::consts::PI
            * dx
            * (radius_at[k].powi(2) + radius_at[k] * radius_at[k - 1] + radius_at[k - 1].powi(2))
            / 3.0;
    }
    let expected = std::f64::consts::PI * RADIUS * RADIUS * LENGTH;
    let min_radius = radius_at.iter().copied().fold(f64::MAX, f64::min);
    (dataset, volume / expected - 1.0, min_radius)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("DEMO_OUT")
        .unwrap_or_else(|_| "/Volumes/home_tmp/tmp/scene3d-demos/necking".to_string());
    std::fs::create_dir_all(&out_dir)?;
    println!(
        "necking bar: L={} mm, R={} mm, Considere strain {CONSIDERE_STRAIN}",
        LENGTH * 1e3,
        RADIUS * 1e3
    );

    let mut steps = Vec::new();
    for step in 0..STEPS {
        let (dataset, volume_error, min_radius) = bar_step(step);
        dataset.validate()?;
        if step % 5 == 0 || step == STEPS - 1 {
            println!(
                "  step {step:2}: volume error {:+.3}%, min radius {:.2} mm",
                volume_error * 100.0,
                min_radius * 1e3
            );
        }
        steps.push(dataset);
    }
    // 5 seconds of movie time; the player interpolates the 16 solver
    // steps into 120 frames at 24 fps.
    let times: Vec<f64> = (0..STEPS)
        .map(|i| 5.0 * i as f64 / (STEPS - 1) as f64)
        .collect();
    let series = TimeSeries::new(steps.clone(), times)?;

    let scene_for = |ds: DataSet| {
        Scene::new()
            .with_actor(Actor::new(ds).coloured_by("strain", TransferFunction::viridis([0.0, 1.6])))
    };
    let initial_scene = scene_for(steps[0].clone());
    let mut camera = Camera::fitted(initial_scene.bounds().unwrap());
    // The bar runs along +x: view it from the side, slightly above.
    camera.azimuth = 0.15;
    camera.elevation = 0.30;
    camera.dolly(0.45);

    for (label, index) in [("initial", 0), ("considere", 9), ("necked", STEPS - 1)] {
        let frame = render(&scene_for(steps[index].clone()), &camera, 800, 300)?;
        std::fs::write(format!("{out_dir}/bar_{label}.png"), frame.to_png()?)?;
    }
    // Axial slice through the final neck, coloured by strain.
    let cut = slice(&steps[STEPS - 1], &Plane::axis(1, 0.0)?)?;
    let frame = render(&scene_for(cut), &camera, 800, 300)?;
    std::fs::write(format!("{out_dir}/bar_neck_slice.png"), frame.to_png()?)?;

    let player = Player::new(series)?;
    let frames = player.movie_frames(scene_for, &camera, 1920, 1080, 120)?;
    for (name, bytes) in png_sequence(&frames)? {
        std::fs::write(format!("{out_dir}/{name}"), bytes)?;
    }
    let mp4 = mux_mp4(&frames)?;
    std::fs::write(format!("{out_dir}/bar_movie.mp4"), &mp4)?;
    println!(
        "  movie: {} frames, {:.1} KB mp4 -> {out_dir}",
        frames.len(),
        mp4.len() as f64 / 1e3
    );
    Ok(())
}
