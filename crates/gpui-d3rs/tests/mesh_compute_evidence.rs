#![cfg(feature = "gpu-compute")]

use d3rs::mesh::gpu::compute::MeshCompute;
use d3rs::mesh::{ContourBand, CoordinateAxis, MarchingTriangles, MeshTopology, ScalarAssociation};
use d3rs::mesh::{ScalarField, TriangleMesh};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;

const EVIDENCE_DIR_ENV: &str = "SOTF_MESH_COMPUTE_EVIDENCE_DIR";
const SOURCE_REVISION_ENV: &str = "SOTF_MESH_COMPUTE_SOURCE_REVISION";
const SOURCE_DIRTY_ENV: &str = "SOTF_MESH_COMPUTE_SOURCE_DIRTY";

fn source_fields(manifest: &mut Value) {
    let Some(revision) = env::var(SOURCE_REVISION_ENV).ok() else {
        return;
    };
    manifest["source_revision"] = Value::String(revision);
    manifest["source_dirty"] = Value::Bool(
        env::var(SOURCE_DIRTY_ENV)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE"))
            .unwrap_or(true),
    );
}

fn write_manifest(mut manifest: Value) -> Result<(), String> {
    let Some(directory) = env::var_os(EVIDENCE_DIR_ENV) else {
        return Ok(());
    };
    let directory = Path::new(&directory);
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    source_fields(&mut manifest);
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(directory.join("mesh-compute-gpu.json"), bytes).map_err(|error| error.to_string())
}

fn write_skip(reason: &str) {
    write_manifest(json!({
        "schema_version": 1,
        "report_type": "gpui-mesh-compute-gpu-evidence",
        "status": "skipped",
        "reason": reason,
    }))
    .expect("write compute evidence skip manifest");
}

fn compute_required() -> bool {
    env::var_os("QA_COMPUTE_REQUIRED").is_some_and(|value| value == "1")
}

fn square_fixture() -> (TriangleMesh, ScalarField, MeshTopology) {
    let mesh = TriangleMesh {
        id: "compute-evidence".into(),
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
        id: "compute-evidence-field".into(),
        label: "compute-evidence-field".into(),
        unit: None,
        values: Arc::from([0.0, 1.0, 0.0, 1.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let topology = MeshTopology::build(&mesh.triangles);
    (mesh, field, topology)
}

fn segments_match(
    expected: &[d3rs::mesh::IsolineSegment],
    actual: &[d3rs::mesh::IsolineSegment],
) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(expected, actual)| {
            (expected.level - actual.level).abs() <= 1e-4
                && expected
                    .start
                    .iter()
                    .zip(actual.start)
                    .all(|(expected, actual)| (expected - actual).abs() <= 1e-4)
                && expected
                    .end
                    .iter()
                    .zip(actual.end)
                    .all(|(expected, actual)| (expected - actual).abs() <= 1e-4)
        })
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

fn bands_match(expected: &[ContourBand], actual: &[ContourBand]) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(expected, actual)| {
            expected.lower == actual.lower
                && expected.upper == actual.upper
                && (band_area(expected) - band_area(actual)).abs() < 2e-4
        })
}

#[test]
fn metal_compute_release_evidence_covers_cpu_parity_and_gpu_timing() {
    let Some(compute) = MeshCompute::try_new() else {
        if compute_required() {
            panic!("required adapter-backed compute device is unavailable");
        }
        write_skip("compute service unavailable");
        return;
    };
    if !compute.adapter_backed() {
        if compute_required() {
            panic!("required adapter-backed compute device is unavailable");
        }
        write_skip("no usable adapter-backed compute device");
        return;
    }

    let backend = compute
        .adapter_backend()
        .map(|backend| format!("{backend:?}").to_ascii_lowercase())
        .unwrap_or_else(|| "unknown".into());
    let (mesh, field, topology) = square_fixture();
    let levels = [0.25_f32, 0.75];
    let bands_levels = [0.0_f32, 0.5, 1.0];
    let cpu = MeshCompute::cpu_reference();
    let values = [0.0_f32, 0.5, 1.0, 0.25];
    let adapter_range = compute.field_min_max(&values);
    let cpu_range = cpu.field_min_max(&values);
    let adapter_segments = compute
        .marching_segments(&mesh, &field, &topology, &levels)
        .expect("adapter isoline dispatch should succeed");
    let cpu_segments = cpu
        .marching_segments(&mesh, &field, &topology, &levels)
        .expect("CPU isoline reference should succeed");
    let adapter_bands = compute
        .band_triangles(&mesh, &field, &topology, &bands_levels)
        .expect("adapter filled-band dispatch should succeed");
    let cpu_bands = cpu
        .band_triangles(&mesh, &field, &topology, &bands_levels)
        .expect("CPU filled-band reference should succeed");

    let range_parity = adapter_range == cpu_range;
    let isoline_parity = segments_match(&cpu_segments, &adapter_segments);
    let band_parity = bands_match(&cpu_bands, &adapter_bands);
    assert!(
        range_parity,
        "adapter field range differs from CPU reference"
    );
    assert!(isoline_parity, "adapter isolines differ from CPU reference");
    assert!(
        band_parity,
        "adapter filled bands differ from CPU reference"
    );

    let timing_requested =
        env::var_os("SOTF_GPU_TIMESTAMPS").as_deref() == Some(std::ffi::OsStr::new("1"));
    write_manifest(json!({
        "schema_version": 1,
        "report_type": "gpui-mesh-compute-gpu-evidence",
        "status": "captured",
        "backend": backend,
        "adapter_backed": true,
        "parity": {
            "field_min_max": range_parity,
            "isolines": isoline_parity,
            "filled_bands": band_parity,
        },
        "timing": {
            "requested": timing_requested,
            "enabled": compute.adapter_gpu_timing_enabled(),
            "sample_count": compute.adapter_gpu_time_count(),
            "last_gpu_time_ns": compute.adapter_gpu_time_ns(),
        },
    }))
    .expect("write compute evidence manifest");
}

#[allow(dead_code)]
fn _projected_axis_isolines_keep_cpu_contract() {
    let (mesh, field, topology) = square_fixture();
    let _ = MarchingTriangles::new(
        &mesh,
        &field,
        &topology,
        CoordinateAxis::X,
        CoordinateAxis::Y,
    );
}
