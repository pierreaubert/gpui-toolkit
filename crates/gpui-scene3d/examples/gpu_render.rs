//! GPU-resident rendering demo: upload once, orbit by uniform updates.
//!
//! Renders a pressure isosurface at 1080p through `GpuRenderer` (Metal on
//! Apple Silicon, Vulkan/CUDA-class elsewhere), writes the frame as PNG
//! through the QA readback, and reports upload size plus per-frame draw
//! cost next to the software rasterizer for the same scene.
//!
//! Run: `cargo run -p gpui-scene3d --example gpu_render --features gpu,png`

// Rust guideline compliant 2026-02-21

use gpui_dataset_transformation::isosurface;
use gpui_mesh::{DataSet, Field};
use gpui_scene3d::{Actor, Camera, GpuRenderer, Scene, render};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("GPU_OUT")
        .unwrap_or_else(|_| "/Volumes/home_tmp/tmp/flow-showcase".to_string());
    std::fs::create_dir_all(&out_dir)?;

    let n = 28usize;
    let step = 6.0 / (n - 1) as f64;
    let mut dataset = DataSet::structured_grid("tg", [n, n, n], [0.0; 3], [step; 3])?;
    let mut pressure = Vec::new();
    for p in dataset.positions.iter() {
        let (x, y, z) = (p[0], p[1], p[2]);
        pressure.push(0.25 * ((2.0 * x).cos() + (2.0 * y).cos()) * (2.0 + (2.0 * z).cos()));
    }
    dataset.point_data.push(Field::scalar("pressure", pressure));
    let surface = isosurface(&dataset, "pressure", 0.4)?;
    // An isosurface has constant pressure, so colour it flat.
    let scene = Scene::new().with_actor(Actor::new(surface).color([0.85, 0.45, 0.35, 1.0]));
    let camera = Camera::fitted(scene.bounds().unwrap());

    let renderer = GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm)?;
    let started = std::time::Instant::now();
    let mut gpu_scene = renderer.upload(&scene)?;
    let upload_ms = started.elapsed().as_secs_f64() * 1e3;
    let target = renderer.create_target(1920, 1080)?;
    let started = std::time::Instant::now();
    renderer.render(&mut gpu_scene, &camera, &target);
    let frame = renderer.read_rgba(&target)?;
    let gpu_ms = started.elapsed().as_secs_f64() * 1e3;
    std::fs::write(format!("{out_dir}/gpu_hero.png"), frame.to_png()?)?;

    let started = std::time::Instant::now();
    let _ = render(&scene, &camera, 1920, 1080)?;
    let cpu_ms = started.elapsed().as_secs_f64() * 1e3;

    println!(
        "gpu_render: {:.1} MB resident, upload {upload_ms:.1} ms, gpu frame {gpu_ms:.1} ms, cpu frame {cpu_ms:.1} ms",
        gpu_scene.geometry_bytes as f64 / 1e6
    );
    println!("  wrote {out_dir}/gpu_hero.png");
    Ok(())
}
