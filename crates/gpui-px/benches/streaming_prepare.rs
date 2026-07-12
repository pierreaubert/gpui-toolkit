use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use gpui_px::{line, scatter};
use std::hint::black_box;
use std::sync::Arc;

type SharedFrame = (Arc<[f64]>, Arc<[f64]>);

fn shared_frames(points: usize) -> [SharedFrame; 2] {
    let x_a: Arc<[f64]> = (0..points).map(|value| value as f64).collect();
    let y_a: Arc<[f64]> = (0..points).map(|value| (value as f64).sin()).collect();
    let x_b: Arc<[f64]> = (0..points).map(|value| value as f64 + 0.5).collect();
    let y_b: Arc<[f64]> = (0..points).map(|value| (value as f64).cos()).collect();
    [(x_a, y_a), (x_b, y_b)]
}

fn benchmark_streaming_prepare(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_prepare");
    for points in [10_000usize, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(points as u64));
        let frames = shared_frames(points);

        let mut line_chart = line(&frames[0].0, &frames[0].1);
        line_chart.prepare_primary_data();
        let mut line_frame = 0usize;
        group.bench_with_input(BenchmarkId::new("line", points), &points, |b, _| {
            b.iter(|| {
                line_frame ^= 1;
                let (x, y) = &frames[line_frame];
                line_chart
                    .replace_primary_data_shared(Arc::clone(x), Arc::clone(y))
                    .unwrap();
                black_box(line_chart.prepare_primary_data());
            });
        });

        let mut scatter_chart = scatter(&frames[0].0, &frames[0].1);
        scatter_chart.prepare_primary_data();
        let mut scatter_frame = 0usize;
        group.bench_with_input(BenchmarkId::new("scatter", points), &points, |b, _| {
            b.iter(|| {
                scatter_frame ^= 1;
                let (x, y) = &frames[scatter_frame];
                scatter_chart
                    .replace_primary_data_shared(Arc::clone(x), Arc::clone(y))
                    .unwrap();
                black_box(scatter_chart.prepare_primary_data());
            });
        });
    }
    group.finish();
}

criterion_group!(benches, benchmark_streaming_prepare);
criterion_main!(benches);
