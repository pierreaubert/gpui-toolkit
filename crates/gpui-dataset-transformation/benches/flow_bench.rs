//! Hot-path benchmarks: flow diagnostics and isosurface on a volume grid.

// Rust guideline compliant 2026-02-21

use criterion::{Criterion, criterion_group, criterion_main};
use gpui_dataset_transformation::{isosurface, q_criterion, vorticity};
use gpui_mesh::{DataSet, Field};

fn rotating_grid(cells: usize) -> DataSet {
    let mut ds = DataSet::structured_grid(
        "bench",
        [cells, cells, cells],
        [-1.0; 3],
        [2.0 / (cells - 1) as f64; 3],
    )
    .unwrap();
    let velocity: Vec<[f64; 3]> = ds
        .positions
        .iter()
        .map(|p| [-2.0 * p[1], 2.0 * p[0], 0.0])
        .collect();
    ds.point_data.push(Field::vector("velocity", velocity));
    let radius: Vec<f64> = ds
        .positions
        .iter()
        .map(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
        .collect();
    ds.point_data.push(Field::scalar("radius", radius));
    ds
}

fn bench_flow(c: &mut Criterion) {
    let ds = rotating_grid(24);
    c.bench_function("vorticity_24cubed", |b| {
        b.iter(|| vorticity(&ds, "velocity", "vorticity"));
    });
    c.bench_function("q_criterion_24cubed", |b| {
        b.iter(|| q_criterion(&ds, "velocity", "q"));
    });
    c.bench_function("isosurface_24cubed", |b| {
        b.iter(|| isosurface(&ds, "radius", 0.8));
    });
}

criterion_group!(benches, bench_flow);
#[cfg(not(feature = "gpu"))]
criterion_main!(benches);
#[cfg(feature = "gpu")]
criterion_main!(benches, gpu_benches);

#[cfg(feature = "gpu")]
fn bench_gpu(c: &mut Criterion) {
    let ds = rotating_grid(24);
    let Ok(gpu) = gpui_dataset_transformation::gpu::GpuFlow::new() else {
        return;
    };
    eprintln!("gpu backend: {}", gpu.backend);
    c.bench_function("gpu_jacobians_24cubed", |b| {
        b.iter(|| gpu.point_jacobians(&ds, "velocity"));
    });
    let large = rotating_grid(48);
    c.bench_function("gpu_jacobians_48cubed", |b| {
        b.iter(|| gpu.point_jacobians(&large, "velocity"));
    });
    c.bench_function("cpu_vorticity_48cubed", |b| {
        b.iter(|| vorticity(&large, "velocity", "vorticity"));
    });
    let mut prepared =
        gpui_dataset_transformation::gpu::PreparedFlow::new(&gpu, &large, "velocity").unwrap();
    c.bench_function("gpu_prepared_48cubed", |b| {
        b.iter(|| prepared.diagnostics(None));
    });
}

#[cfg(feature = "gpu")]
criterion_group!(gpu_benches, bench_gpu);
