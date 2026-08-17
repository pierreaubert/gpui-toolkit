use d3rs::mesh::gpu::compute::MeshCompute;
use d3rs::mesh::{ContourBand, CoordinateAxis, MarchingTriangles, MeshTopology, ScalarAssociation};
use d3rs::mesh::{ScalarField, TriangleMesh};
use std::env;
use std::sync::Arc;

#[cfg(target_os = "macos")]
#[test]
fn adapter_compute_differential_path_reports_metal_on_macos() {
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
    let Some(backend) = compute.adapter_backend() else {
        return;
    };
    assert_eq!(backend, wgpu::Backend::Metal);
}

#[test]
fn adapter_compute_timestamp_queries_recover_gpu_duration_when_opted_in() {
    if env::var_os("SOTF_GPU_TIMESTAMPS").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
    if !compute.adapter_backed() || !compute.adapter_gpu_timing_enabled() {
        eprintln!("SKIP compute timestamp test: adapter has no timestamp-query support");
        return;
    }

    let (mesh, field, topology) = square_fixture();
    assert_eq!(
        compute.field_min_max(&[0.0, 0.5, 1.0, 0.25]),
        Some([0.0, 1.0])
    );
    assert!(compute.adapter_gpu_time_count() > 0);
    assert!(compute.adapter_gpu_time_ns() > 0);

    compute
        .marching_segments(&mesh, &field, &topology, &[0.5])
        .expect("timestamped adapter isoline dispatch should succeed");
    assert!(compute.adapter_gpu_time_count() > 1);
}

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

fn band_area(band: &ContourBand) -> f64 {
    band.triangles
        .iter()
        .map(|&[a, b, c]| {
            let a = band.positions[a as usize];
            let b = band.positions[b as usize];
            let c = band.positions[c as usize];
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
        })
        .sum()
}

fn assert_bands_match_cpu(expected: &[ContourBand], actual: &[ContourBand]) {
    assert_eq!(actual.len(), expected.len());
    for (expected, actual) in expected.iter().zip(actual) {
        assert_eq!(actual.lower, expected.lower);
        assert_eq!(actual.upper, expected.upper);
        assert!(
            (band_area(actual) - band_area(expected)).abs() < 2e-4,
            "GPU area {} != CPU area {} for {:?}..{:?}",
            band_area(actual),
            band_area(expected),
            expected.lower,
            expected.upper,
        );
    }
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
fn on_level_tie_break_matches_cpu_zero_length_segment() {
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
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
        .marching_segments_projected(
            &mesh,
            &field,
            &topology,
            CoordinateAxis::X,
            CoordinateAxis::Y,
            &[0.5],
        )
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

#[test]
fn projected_axis_isolines_match_cpu_with_or_without_an_adapter() {
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
    let mesh = TriangleMesh {
        id: "projected-compute-diff".into(),
        positions: Arc::from([
            [10.0, 0.0, 100.0],
            [20.0, 2.0, 100.0],
            [30.0, 0.0, 104.0],
            [40.0, 2.0, 104.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [1, 3, 2]]),
        vertex_ids: None,
        cell_ids: None,
    };
    let field = ScalarField {
        id: "projected-field".into(),
        label: "projected-field".into(),
        unit: None,
        values: Arc::from([0.0, 1.0, 0.0, 1.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let topology = MeshTopology::build(&mesh.triangles);
    let levels = [0.25_f64, 0.75];
    let cpu = MarchingTriangles::new(
        &mesh,
        &field,
        &topology,
        CoordinateAxis::Z,
        CoordinateAxis::Y,
    )
    .unwrap()
    .isolines(&levels);
    let actual = compute
        .marching_segments_projected(
            &mesh,
            &field,
            &topology,
            CoordinateAxis::Z,
            CoordinateAxis::Y,
            &levels,
        )
        .unwrap();

    assert_eq!(actual.len(), cpu.len());
    for (expected, actual) in cpu.iter().zip(&actual) {
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
fn masked_vertex_isolines_match_cpu_with_or_without_an_adapter() {
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
    let (mesh, mut field, topology) = square_fixture();
    field.values = Arc::from([0.0, 1.0, 0.0, f64::NAN]);
    field.valid = Some(Arc::from([true, true, true, false]));
    let levels = [0.5_f64];
    let cpu = MarchingTriangles::new(
        &mesh,
        &field,
        &topology,
        CoordinateAxis::X,
        CoordinateAxis::Y,
    )
    .unwrap()
    .isolines(&levels);
    let actual = compute
        .marching_segments_projected(
            &mesh,
            &field,
            &topology,
            CoordinateAxis::X,
            CoordinateAxis::Y,
            &levels,
        )
        .unwrap();

    assert_eq!(actual.len(), cpu.len());
    for (expected, actual) in cpu.iter().zip(&actual) {
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
fn adapter_filled_bands_match_cpu_golden_area_and_boundaries() {
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
    if !compute.adapter_backed() {
        return;
    }
    let (mesh, field, topology) = square_fixture();
    let levels = [0.0_f32, 0.25, 0.75, 1.0];
    let expected = MarchingTriangles::new(
        &mesh,
        &field,
        &topology,
        CoordinateAxis::X,
        CoordinateAxis::Y,
    )
    .unwrap()
    .filled_bands(&levels.iter().map(|&level| level as f64).collect::<Vec<_>>());
    let actual = compute
        .band_triangles(&mesh, &field, &topology, &levels)
        .unwrap();
    assert_bands_match_cpu(&expected, &actual);
}

#[test]
fn projected_and_masked_filled_bands_match_cpu_with_or_without_an_adapter() {
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
    let mut mesh = square_fixture().0;
    mesh.positions = Arc::from([
        [10.0, 0.0, 100.0],
        [20.0, 2.0, 100.0],
        [30.0, 0.0, 104.0],
        [40.0, 2.0, 104.0],
    ]);
    let mut field = square_fixture().1;
    field.values = Arc::from([0.0, 1.0, 0.0, f64::NAN]);
    field.valid = Some(Arc::from([true, true, true, false]));
    let topology = MeshTopology::build(&mesh.triangles);
    let levels = [0.0_f64, 0.5, 1.0];
    let expected = MarchingTriangles::new(
        &mesh,
        &field,
        &topology,
        CoordinateAxis::Z,
        CoordinateAxis::Y,
    )
    .unwrap()
    .filled_bands(&levels);
    let actual = compute
        .band_triangles_projected(
            &mesh,
            &field,
            &topology,
            CoordinateAxis::Z,
            CoordinateAxis::Y,
            &levels,
        )
        .unwrap();
    assert_bands_match_cpu(&expected, &actual);
}

#[test]
fn multilevel_on_vertex_and_masked_neighbor_bands_match_cpu_with_finite_output() {
    let Some(compute) = MeshCompute::try_new() else {
        return;
    };
    let (mesh, mut field, topology) = square_fixture();
    // The first triangle contains a sample exactly on the middle level; the
    // second triangle is fully excluded by its masked vertex. This combines
    // the two most error-prone clipping paths in one multi-band dispatch.
    field.values = Arc::from([0.0, 0.5, 1.0, f64::NAN]);
    field.valid = Some(Arc::from([true, true, true, false]));
    let levels = [0.0_f64, 0.25, 0.5, 0.75, 1.0];
    let expected = MarchingTriangles::new(
        &mesh,
        &field,
        &topology,
        CoordinateAxis::X,
        CoordinateAxis::Y,
    )
    .unwrap()
    .filled_bands(&levels);
    let actual = compute
        .band_triangles_projected(
            &mesh,
            &field,
            &topology,
            CoordinateAxis::X,
            CoordinateAxis::Y,
            &levels,
        )
        .unwrap();
    assert_bands_match_cpu(&expected, &actual);
    for band in &actual {
        for position in &band.positions {
            assert!(
                position.iter().all(|coordinate| coordinate.is_finite()),
                "band {:?}..{:?} contains a non-finite clipped position: {position:?}",
                band.lower,
                band.upper,
            );
        }
        assert!(
            band_area(band).is_finite(),
            "band {:?}..{:?} has non-finite total area",
            band.lower,
            band.upper,
        );
    }
}
