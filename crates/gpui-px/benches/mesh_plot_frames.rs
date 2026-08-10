use criterion::{Criterion, criterion_group, criterion_main};
use d3rs::mesh::TriangleMesh;
use gpui_px::mesh_plot::MeshPlotState;
use gpui_px::{MeshRenderMode, mesh_plot};
use std::hint::black_box;
use std::sync::Arc;

fn bench(c: &mut Criterion) {
    let mesh = TriangleMesh {
        id: "bench".into(),
        positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
        triangles: Arc::from([[0, 1, 2]]),
        vertex_ids: None,
        cell_ids: None,
    };
    c.bench_function("mesh_plot_svg_frame", |b| {
        b.iter(|| black_box(mesh_plot(mesh.clone()).mode(MeshRenderMode::Mesh).to_svg()))
    });

    let mut group = c.benchmark_group("mesh_plot_retained_frames");
    for value_count in [1_000_000, 10_000_000] {
        let values_a = vec![0.25_f32; value_count];
        let values_b = vec![0.75_f32; value_count];

        let mut camera_state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        camera_state.reserve_field_capacity(value_count);
        camera_state.replace_field_values(1, &values_a);
        group.bench_function(format!("camera_{value_count}_values"), |b| {
            b.iter(|| {
                camera_state.set_viewport_without_history(0.05, 0.95, 0.1, 0.9);
                black_box(camera_state.interaction.x_domain());
            })
        });

        let mut field_state = MeshPlotState::new(0.0, 1.0, 0.0, 1.0);
        field_state.reserve_field_capacity(value_count);
        field_state.replace_field_values(1, &values_a);
        group.bench_function(format!("field_replace_{value_count}_values"), |b| {
            let mut revision = 2;
            b.iter(|| {
                let values = if revision % 2 == 0 {
                    &values_b
                } else {
                    &values_a
                };
                field_state.replace_field_values(revision, values);
                revision += 1;
                black_box(field_state.field_revision);
            })
        });
    }
    group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
