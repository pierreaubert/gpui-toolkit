use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use d3rs::prelude::*;
use d3rs::shape::{ScatterConfig, ScatterPoint, scatter_chart_scene};
use d3rs::vello2d::CpuRasterizer;

fn scatter_data(n: usize) -> Vec<ScatterPoint> {
    (0..n)
        .map(|i| ScatterPoint::new(i as f64 * 0.01, (i as f64 * 0.017).sin() * 50.0 + 50.0))
        .collect()
}

fn bench_vello2d_scatter(c: &mut Criterion) {
    let mut group = c.benchmark_group("vello2d_scatter");
    for n in [100_000usize, 1_000_000] {
        let data = scatter_data(n);
        let x_scale = LinearScale::new()
            .domain(0.0, n as f64 * 0.01)
            .range(0.0, 800.0);
        let y_scale = LinearScale::new().domain(0.0, 100.0).range(600.0, 0.0);
        let config = ScatterConfig::new()
            .fill_color(D3Color::from_hex(0x1f77b4))
            .point_radius(2.0);

        group.bench_with_input(BenchmarkId::new("scene_build", n), &data, |b, data| {
            b.iter(|| scatter_chart_scene(&x_scale, &y_scale, data, &config, 800.0, 600.0))
        });

        let scene = scatter_chart_scene(&x_scale, &y_scale, &data, &config, 800.0, 600.0);
        let mut rast = CpuRasterizer::new(800, 600);
        group.bench_with_input(BenchmarkId::new("cpu_raster", n), &scene, |b, scene| {
            b.iter(|| rast.rasterize(scene, 800, 600))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_vello2d_scatter);
criterion_main!(benches);
