use criterion::{Criterion, criterion_group, criterion_main};
use d3rs::mesh::{
    CoordinateAxis, MarchingTriangles, MeshBvh, MeshTopology, RevolveSpec, ScalarAssociation,
    ScalarField, TriangleMesh, prepare_upload, revolve,
};
use std::hint::black_box;
use std::sync::Arc;

fn fixture(triangles: usize) -> TriangleMesh {
    let mut positions = Vec::with_capacity(triangles * 3);
    let mut indices = Vec::with_capacity(triangles);
    for index in 0..triangles {
        let base = positions.len() as u32;
        let x = index as f64;
        positions.extend([[x, 0.0, 0.0], [x + 1.0, 0.0, 0.0], [x, 1.0, 0.0]]);
        indices.push([base, base + 1, base + 2]);
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
    for triangle_count in [100_000, 1_000_000, 10_000_000] {
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
        let isoline_levels = [0.25, 0.5, 0.75];
        group.bench_function(
            format!("marching_isolines_{triangle_count}_triangles"),
            |b| b.iter(|| black_box(marching.isolines(&isoline_levels))),
        );
        group.bench_function(format!("marching_bands_{triangle_count}_triangles"), |b| {
            b.iter(|| black_box(marching.filled_bands(&[0.25, 0.5, 0.75])))
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
    group.finish();
}
criterion_group!(benches, mesh_prep);
criterion_main!(benches);
