use criterion::{Criterion, criterion_group, criterion_main};
use d3rs::mesh::TriangleMesh;
use gpui_px::mesh_plot::MeshPlotState;
use gpui_px::{MeshRenderMode, mesh_plot};
use std::hint::black_box;
use std::sync::Arc;

#[cfg(feature = "gpu-3d")]
use d3rs::gpu3d::Camera3D;
#[cfg(feature = "gpu-3d")]
use d3rs::mesh::{MeshBvh, RevolveSpec, ScalarAssociation, ScalarField, revolve, revolve_field};
#[cfg(feature = "gpu-3d")]
use gpui_px::mesh_plot::picking3d::{pick_3d_with_bvh, pick_revolved_3d_with_bvh};

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

    #[cfg(feature = "gpu-3d")]
    retained_pick_bench(c);
}

#[cfg(feature = "gpu-3d")]
fn retained_pick_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_plot_retained_picking");
    let surface = TriangleMesh {
        id: "surface".into(),
        positions: Arc::from([
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: None,
        cell_ids: None,
    };
    let field = ScalarField {
        id: "height".into(),
        label: "Height".into(),
        unit: None,
        values: Arc::from([0.0, 1.0, 2.0, 1.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let camera = Camera3D::default().with_position(glam::Vec3::new(0.0, 0.0, 3.0));
    let bvh = MeshBvh::build(&surface);
    let plot_id: Arc<str> = Arc::from("bench-surface");
    group.bench_function("surface_bvh_pick", |b| {
        b.iter(|| {
            black_box(pick_3d_with_bvh(
                &surface,
                Some(&field),
                &bvh,
                &camera,
                [50.0, 50.0],
                [100.0, 100.0],
                plot_id.clone(),
            ))
        })
    });

    let profile = TriangleMesh {
        id: "profile".into(),
        positions: Arc::from([[0.0, 0.0, -1.0], [1.0, 0.0, -1.0], [1.0, 0.0, 1.0]]),
        triangles: Arc::from([[0, 1, 2]]),
        vertex_ids: Some(Arc::from([10, 20, 30])),
        cell_ids: Some(Arc::from([40])),
    };
    let source_field = ScalarField {
        id: "pressure".into(),
        label: "Pressure".into(),
        unit: None,
        values: Arc::from([0.0, 1.0, 2.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let revolved = revolve(&profile, &RevolveSpec::default()).expect("valid benchmark profile");
    let derived_field = ScalarField {
        values: revolve_field(&source_field, &revolved).into(),
        ..source_field.clone()
    };
    let revolved_bvh = MeshBvh::build(&revolved.mesh);
    let revolved_id: Arc<str> = Arc::from("bench-revolve");
    group.bench_function("revolved_bvh_pick", |b| {
        b.iter(|| {
            black_box(pick_revolved_3d_with_bvh(
                &profile,
                &revolved,
                Some(&derived_field),
                &revolved_bvh,
                Some(source_field.id.clone()),
                &camera,
                [50.0, 50.0],
                [100.0, 100.0],
                revolved_id.clone(),
            ))
        })
    });
    group.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
