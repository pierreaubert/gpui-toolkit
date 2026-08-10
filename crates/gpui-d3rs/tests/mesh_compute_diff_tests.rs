use d3rs::mesh::gpu::compute::MeshCompute;
use d3rs::mesh::{CoordinateAxis, MarchingTriangles, MeshTopology, ScalarAssociation};
use d3rs::mesh::{ScalarField, TriangleMesh};
use std::sync::Arc;

fn square_fixture() -> (TriangleMesh, ScalarField, MeshTopology) {
    let mesh = TriangleMesh {
        id: "compute-diff".into(),
        positions: Arc::from([
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [1, 3, 2]]),
        vertex_ids: None,
        cell_ids: None,
    };
    let field = ScalarField {
        id: "compute-field".into(),
        label: "compute-field".into(),
        unit: None,
        values: Arc::from([0.0, 1.0, 0.0, 1.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let topology = MeshTopology::build(&mesh.triangles);
    (mesh, field, topology)
}

#[test]
fn adapter_isolines_match_cpu_golden_order_and_geometry() {
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
    if !compute.adapter_backed() {
        return;
    }
    let (mesh, field, topology) = square_fixture();
    let levels = [0.5_f32, 0.75];
    let cpu_levels = levels.iter().map(|&level| level as f64).collect::<Vec<_>>();
    let cpu = MarchingTriangles::new(
        &mesh,
        &field,
        &topology,
        CoordinateAxis::X,
        CoordinateAxis::Y,
    )
    .unwrap()
    .isolines(&cpu_levels);
    let gpu = compute
        .marching_segments(&mesh, &field, &topology, &levels)
        .unwrap();

    assert_eq!(gpu.len(), cpu.len());
    for (expected, actual) in cpu.iter().zip(&gpu) {
        assert_eq!(actual.level, expected.level);
        for (&actual, &expected) in actual.start.iter().zip(expected.start.iter()) {
            assert!((actual - expected).abs() < 1e-4, "{actual} != {expected}");
        }
        for (&actual, &expected) in actual.end.iter().zip(expected.end.iter()) {
            assert!((actual - expected).abs() < 1e-4, "{actual} != {expected}");
        }
    }
}

#[test]
fn adapter_on_level_tie_break_matches_cpu_zero_length_segment() {
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
    if !compute.adapter_backed() {
        return;
    }
    let mesh = TriangleMesh {
        id: "on-level".into(),
        positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
        triangles: Arc::from([[0, 1, 2]]),
        vertex_ids: None,
        cell_ids: None,
    };
    let field = ScalarField {
        id: "on-level-field".into(),
        label: "on-level-field".into(),
        unit: None,
        values: Arc::from([0.5, 0.0, 0.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let topology = MeshTopology::build(&mesh.triangles);
    let segments = compute
        .marching_segments(&mesh, &field, &topology, &[0.5])
        .unwrap();
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].start, segments[0].end);
}

#[test]
fn adapter_field_min_max_matches_cpu_scan() {
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
    if !compute.adapter_backed() {
        return;
    }
    let values = [f32::NAN, -4.5, 1.25, f32::INFINITY, 9.0];
    assert_eq!(compute.field_min_max(&values), Some([-4.5, 9.0]));
}
