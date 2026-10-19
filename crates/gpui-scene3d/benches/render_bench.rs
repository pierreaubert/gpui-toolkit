//! Hot-path benchmarks: chunk preparation, surface render, volume raycast.

// Rust guideline compliant 2026-02-21

use criterion::{Criterion, criterion_group, criterion_main};
use gpui_mesh::{DataSet, Field};
use gpui_scene3d::{
    Actor, Camera, Scene, TransferFunction, VolumeOptions, prepare_chunks, raycast_volume, render,
};

fn volume(cells: usize) -> DataSet {
    let mut ds = DataSet::structured_grid(
        "bench",
        [cells, cells, cells],
        [-1.0; 3],
        [2.0 / (cells - 1) as f64; 3],
    )
    .unwrap();
    let values: Vec<f64> = ds
        .positions
        .iter()
        .map(|p| (-2.0 * (p[0] * p[0] + p[1] * p[1] + p[2] * p[2])).exp())
        .collect();
    ds.point_data.push(Field::scalar("pressure", values));
    ds
}

fn bench_render(c: &mut Criterion) {
    let ds = volume(32);
    let actor =
        Actor::new(ds.clone()).coloured_by("pressure", TransferFunction::viridis([0.0, 1.0]));
    c.bench_function("prepare_chunks_32cubed", |b| {
        b.iter(|| prepare_chunks(&actor, 100_000).unwrap());
    });
    let scene = Scene::new().with_actor(actor);
    let camera = Camera::fitted(scene.bounds().unwrap());
    c.bench_function("render_surface_128", |b| {
        b.iter(|| render(&scene, &camera, 128, 128).unwrap());
    });
    let transfer = TransferFunction::viridis([0.0, 1.0]);
    c.bench_function("raycast_volume_64", |b| {
        b.iter(|| {
            raycast_volume(
                &ds,
                "pressure",
                &transfer,
                &camera,
                64,
                64,
                &VolumeOptions::default(),
            )
            .unwrap()
        });
    });
}

criterion_group!(benches, bench_render);
criterion_main!(benches);
