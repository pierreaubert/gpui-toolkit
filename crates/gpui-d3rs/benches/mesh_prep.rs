use criterion::{Criterion, criterion_group, criterion_main};
use d3rs::mesh::gpu::MeshLodController;
use d3rs::mesh::{
    CoordinateAxis, MarchingTriangles, MeshBvh, MeshTopology, RevolveSpec, ScalarAssociation,
    ScalarField, TriangleMesh, prepare_upload, revolve, revolve_field,
};
use std::hint::black_box;
use std::sync::Arc;

/// A connected triangulated grid. Unlike independent triangles, this drives
/// shared-edge topology, contour stitching, normal accumulation, and BVH
/// locality in the same way as a typical finite-element surface mesh.
fn fixture(triangles: usize) -> TriangleMesh {
    let side = ((triangles as f64 / 2.0).sqrt().ceil() as usize).saturating_add(1);
    let positions = (0..side)
        .flat_map(|y| (0..side).map(move |x| [x as f64, y as f64, 0.0]))
        .collect::<Vec<_>>();
    let mut indices = Vec::with_capacity(triangles);
    'rows: for y in 0..side - 1 {
        for x in 0..side - 1 {
            let a = (y * side + x) as u32;
            let b = a + 1;
            let c = a + side as u32;
            let d = c + 1;
            indices.push([a, b, c]);
            if indices.len() == triangles {
                break 'rows;
            }
            indices.push([b, d, c]);
            if indices.len() == triangles {
                break 'rows;
            }
        }
    }
    TriangleMesh {
        id: "bench".into(),
        positions: Arc::from(positions),
        triangles: Arc::from(indices),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn vertex_field(mesh: &TriangleMesh) -> ScalarField {
    ScalarField {
        id: "bench-field".into(),
        label: "bench-field".into(),
        unit: None,
        values: mesh
            .positions
            .iter()
            .map(|position| position[0] * 0.001 + position[1])
            .collect::<Vec<_>>()
            .into(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

fn revolve_fixture() -> TriangleMesh {
    TriangleMesh {
        id: "revolve-bench".into(),
        positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]),
        triangles: Arc::from([[0, 1, 2]]),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn mesh_prep(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_prep");
    // The first two sizes are the release-evidence workloads from the MeshPlot
    // specification: 100k vertices / 200k triangles is representative of a
    // production BEM/FEM surface without turning routine CI into a 10M-tri
    // memory stress test.
    for triangle_count in [100_000, 200_000] {
        let mesh = fixture(triangle_count);
        let topology = MeshTopology::build(&mesh.triangles);
        group.bench_function(format!("topology_{triangle_count}_triangles"), |b| {
            b.iter(|| black_box(MeshTopology::build(&mesh.triangles)))
        });
        group.bench_function(format!("prepare_{triangle_count}_triangles"), |b| {
            b.iter(|| black_box(prepare_upload(&mesh, &topology)))
        });

        let field = vertex_field(&mesh);
        let marching = MarchingTriangles::new(
            &mesh,
            &field,
            &topology,
            CoordinateAxis::X,
            CoordinateAxis::Y,
        )
        .expect("benchmark fixture uses a vertex field");
        // Twelve levels exercise the release contour workload rather than the
        // three-level smoke fixture used by the original benchmark.
        let contour_levels = [
            20.0, 40.0, 60.0, 80.0, 100.0, 120.0, 140.0, 160.0, 180.0, 200.0, 220.0, 240.0,
        ];
        group.bench_function(
            format!("marching_isolines_{triangle_count}_triangles"),
            |b| b.iter(|| black_box(marching.isolines(&contour_levels))),
        );
        group.bench_function(format!("marching_bands_{triangle_count}_triangles"), |b| {
            b.iter(|| black_box(marching.filled_bands(&contour_levels)))
        });

        group.bench_function(format!("bvh_{triangle_count}_triangles"), |b| {
            b.iter(|| black_box(MeshBvh::build(&mesh)))
        });
        let bvh = MeshBvh::build(&mesh);
        group.bench_function(format!("bvh_query_{triangle_count}_triangles"), |b| {
            b.iter(|| black_box(bvh.ray_cast([0.25, 0.25, 1.0], [0.0, 0.0, -1.0])))
        });
    }

    let revolve_mesh = revolve_fixture();
    let revolve_spec = RevolveSpec {
        radial: CoordinateAxis::X,
        axial: CoordinateAxis::Z,
        start_angle: 0.0,
        sweep_angle: std::f64::consts::TAU,
        segments: 64,
        end_caps: false,
    };
    group.bench_function("revolve_full_64_segments", |b| {
        b.iter(|| black_box(revolve(&revolve_mesh, &revolve_spec)))
    });
    let revolve_field_values = vertex_field(&revolve_mesh);
    group.bench_function("revolve_full_64_segments_with_vertex_field", |b| {
        b.iter(|| {
            let revolved = revolve(&revolve_mesh, &revolve_spec)
                .expect("benchmark fixture has a valid axisymmetric profile");
            black_box(revolve_field(&revolve_field_values, &revolved))
        })
    });
    let partial_revolve_spec = RevolveSpec {
        sweep_angle: std::f64::consts::PI,
        end_caps: true,
        ..revolve_spec
    };
    group.bench_function("revolve_partial_capped_64_segments", |b| {
        b.iter(|| black_box(revolve(&revolve_mesh, &partial_revolve_spec)))
    });
    group.bench_function(
        "revolve_partial_capped_64_segments_with_vertex_field",
        |b| {
            b.iter(|| {
                let revolved = revolve(&revolve_mesh, &partial_revolve_spec)
                    .expect("benchmark fixture has a valid axisymmetric profile");
                black_box(revolve_field(&revolve_field_values, &revolved))
            })
        },
    );

    // Drag-time LOD evidence uses the same connected topology as the release
    // mesh-preparation workloads. Proxy and full uploads are measured
    // separately, while the controller transition and association-safe field
    // mapping are measured without involving a platform GPU.
    let lod_mesh = fixture(100_000);
    let lod_field = vertex_field(&lod_mesh);
    let mut lod_controller = MeshLodController::with_lod_threshold(lod_mesh.clone(), 10_000);
    let proxy_mesh = lod_controller
        .proxy_mesh()
        .expect("large connected mesh must have a drag proxy")
        .clone();
    let proxy_topology = MeshTopology::build(&proxy_mesh.triangles);
    let full_topology = MeshTopology::build(&lod_mesh.triangles);
    group.bench_function("lod_proxy_upload_100000_triangles", |b| {
        b.iter(|| black_box(prepare_upload(&proxy_mesh, &proxy_topology)))
    });
    group.bench_function("lod_full_restore_upload_100000_triangles", |b| {
        b.iter(|| black_box(prepare_upload(&lod_mesh, &full_topology)))
    });
    group.bench_function("lod_proxy_field_mapping_100000_triangles", |b| {
        b.iter(|| black_box(lod_controller.active_field(&lod_field)))
    });
    group.bench_function("lod_drag_transition_100000_triangles", |b| {
        b.iter(|| {
            lod_controller.begin_camera_drag();
            let proxy_count = lod_controller.active_mesh().triangles.len();
            lod_controller.end_camera_drag();
            black_box((proxy_count, lod_controller.active_mesh().triangles.len()))
        })
    });
    group.finish();
}
criterion_group!(benches, mesh_prep);
criterion_main!(benches);
