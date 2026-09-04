//! Regression benches for spec validation and mesh frame ingest.
//!
//! Budgets (MacBook-class dev machine, criterion stable timing):
//! - `mesh_spec_validate`: p50 < 100 us for a 100-vertex inline spec.
//! - `frame_ingest`: p50 < 50 us per 100-vertex f64 frame.
//! - `positions_decode_cold`: p50 < 500 us first decode of a 10k-vertex frame.
//! - `positions_decode_hot`: p50 < 1 us cache-hit share (Arc clone only).
//!
//! Compare against the stored baseline before merging perf-sensitive changes:
//! `cargo bench -p gpui-python-runtime --bench spec_validate -- --save-baseline main`.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use gpui_python_runtime::mesh_frames::{MeshDtype, MeshFrame, MeshFrameKind, MeshFrameStore};
use gpui_python_runtime::meshplot::MeshPlotSpec;
use std::hint::black_box;

fn valid_spec(vertex_count: usize) -> serde_json::Value {
    let positions = (0..vertex_count)
        .map(|index| [index as f64, index as f64, 0.0])
        .collect::<Vec<_>>();
    let triangles = (0..vertex_count.saturating_sub(2))
        .map(|index| [index as u32, index as u32 + 1, index as u32 + 2])
        .collect::<Vec<_>>();
    serde_json::json!({
        "schema_version": 1,
        "id": "bench",
        "geometry": {"id": "mesh", "positions": positions, "triangles": triangles},
        "field": {"values": vec![0.0; vertex_count], "association": "vertex"},
        "mode": "scalar_fill"
    })
}

fn positions_frame(resource_id: &str, vertex_count: usize) -> MeshFrame {
    let payload = (0..vertex_count * 3)
        .flat_map(|value| (value as f64).to_le_bytes())
        .collect::<Vec<_>>();
    let checksum = MeshFrame::checksum(&payload);
    MeshFrame {
        resource_id: resource_id.into(),
        generation: 1,
        sequence: 0,
        chunk_count: 1,
        kind: MeshFrameKind::Geometry,
        dtype: MeshDtype::F64LE,
        shape: vec![vertex_count as u32, 3],
        payload,
        checksum,
    }
}

fn bench_validate(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("mesh_spec_validate");
    for vertex_count in [100, 1_000] {
        let spec = valid_spec(vertex_count);
        group.bench_with_input(
            BenchmarkId::from_parameter(vertex_count),
            &spec,
            |bencher, spec| {
                bencher.iter(|| {
                    MeshPlotSpec::validate_value(black_box(spec)).expect("bench spec is valid")
                });
            },
        );
    }
    group.finish();
}

fn bench_ingest(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("frame_ingest");
    for vertex_count in [100, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(vertex_count),
            &vertex_count,
            |bencher, &vertex_count| {
                bencher.iter_with_setup(
                    || {
                        (
                            MeshFrameStore::new(),
                            positions_frame("bench", vertex_count),
                        )
                    },
                    |(mut store, frame)| {
                        black_box(store.ingest(black_box(frame)).expect("bench frame ingests"));
                    },
                );
            },
        );
    }
    group.finish();
}

fn bench_positions_decode(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("positions_decode");
    group.bench_function("cold", |bencher| {
        let mut counter = 0u64;
        bencher.iter_with_setup(
            || {
                counter += 1;
                let mut store = MeshFrameStore::new();
                let id = format!("cold-{counter}");
                store
                    .ingest(positions_frame(&id, 10_000))
                    .expect("bench frame ingests");
                (store, id)
            },
            |(store, id)| {
                black_box(
                    store
                        .decoded_positions(black_box(&id), 1)
                        .expect("bench positions decode"),
                );
            },
        );
    });
    let mut store = MeshFrameStore::new();
    store
        .ingest(positions_frame("hot", 10_000))
        .expect("bench frame ingests");
    store
        .decoded_positions("hot", 1)
        .expect("bench positions decode");
    group.bench_function("hot", |bencher| {
        bencher.iter(|| {
            black_box(
                store
                    .decoded_positions(black_box("hot"), 1)
                    .expect("bench positions decode"),
            );
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_validate,
    bench_ingest,
    bench_positions_decode
);
criterion_main!(benches);
