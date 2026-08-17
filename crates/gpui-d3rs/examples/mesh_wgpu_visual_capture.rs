//! Capture retained MeshPlot scenes through the real WGPU adapter-backed
//! renderer. The output is consumed by the WGPU visual QA lane.

use d3rs::mesh::gpu::{GeometryRevision, MeshColorConfig, MeshSceneState, render_offscreen_wgpu};
use d3rs::mesh::{
    MeshTopology, RevolveSpec, ScalarAssociation, ScalarField, TriangleMesh, prepare_upload,
    revolve, revolve_field,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 192;

#[derive(Clone, Copy)]
struct CaptureCase {
    id: &'static str,
    description: &'static str,
}

const CASES: &[CaptureCase] = &[
    CaptureCase {
        id: "mesh",
        description: "mesh-only",
    },
    CaptureCase {
        id: "smooth",
        description: "smooth vertex scalar fill",
    },
    CaptureCase {
        id: "wireframe",
        description: "smooth fill with wireframe",
    },
    CaptureCase {
        id: "isoline",
        description: "smooth fill with isoline",
    },
    CaptureCase {
        id: "revolve",
        description: "axisymmetric revolved surface",
    },
];

fn square_mesh() -> TriangleMesh {
    TriangleMesh {
        id: "wgpu-visual-square".into(),
        positions: Arc::from([
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn square_field() -> ScalarField {
    ScalarField {
        id: "wgpu-visual-field".into(),
        label: "scalar".into(),
        unit: Some("arb".into()),
        values: Arc::from([0.0, 0.5, 1.0, 0.25]),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

fn profile_mesh() -> TriangleMesh {
    TriangleMesh {
        id: "wgpu-visual-profile".into(),
        positions: Arc::from([
            [0.15, 0.0, -1.0],
            [0.75, 0.0, -1.0],
            [0.15, 0.0, 1.0],
            [0.75, 0.0, 1.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [1, 3, 2]]),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn profile_field() -> ScalarField {
    ScalarField {
        id: "wgpu-visual-profile-field".into(),
        label: "pressure".into(),
        unit: Some("Pa".into()),
        values: Arc::from([0.0, 0.35, 0.75, 1.0]),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

fn scene_for(id: &str) -> Result<MeshSceneState, String> {
    if id == "revolve" {
        let profile = profile_mesh();
        let field = profile_field();
        let spec = RevolveSpec {
            radial: d3rs::mesh::CoordinateAxis::X,
            axial: d3rs::mesh::CoordinateAxis::Z,
            segments: 32,
            end_caps: true,
            ..RevolveSpec::default()
        };
        let result = revolve(&profile, &spec).map_err(|error| error.to_string())?;
        let values = revolve_field(&field, &result);
        let upload = prepare_upload(&result.mesh, &MeshTopology::build(&result.mesh.triangles));
        let mut state = MeshSceneState {
            geometry_rev: GeometryRevision(1),
            upload: Some(upload),
            color: MeshColorConfig {
                range: [0.0, 1.0],
                unlit: false,
                ..MeshColorConfig::default()
            },
            ..MeshSceneState::default()
        };
        state.upload.as_mut().unwrap().values_f32 =
            Some(values.iter().map(|value| *value as f32).collect::<Vec<_>>());
        return Ok(state);
    }

    let (field, wireframe, isoline_step) = match id {
        "mesh" => (None, false, 0.0),
        "smooth" => (Some(square_field()), false, 0.0),
        "wireframe" => (Some(square_field()), true, 0.0),
        "isoline" => (Some(square_field()), false, 0.5),
        other => return Err(format!("unknown WGPU visual capture case: {other}")),
    };
    let mesh = square_mesh();
    let upload = prepare_upload(&mesh, &MeshTopology::build(&mesh.triangles));
    let mut state = MeshSceneState {
        geometry_rev: GeometryRevision(1),
        upload: Some(upload),
        color: MeshColorConfig {
            range: [0.0, 1.0],
            wireframe,
            isoline_step,
            isoline_width_px: if isoline_step == 0.0 { 0.0 } else { 1.5 },
            unlit: true,
            ..MeshColorConfig::default()
        },
        ..MeshSceneState::default()
    };
    if let Some(field) = field {
        state.upload.as_mut().unwrap().values_f32 =
            Some(field.values.iter().map(|value| *value as f32).collect());
    }
    Ok(state)
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        hash.wrapping_mul(0x100000001b3) ^ u64::from(*byte)
    })
}

fn output_dir() -> PathBuf {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--output-dir" {
            return PathBuf::from(args.next().expect("--output-dir requires a path"));
        }
    }
    PathBuf::from("target/qa/visual/mesh-plot-wgpu/actual")
}

fn write_manifest(dir: &Path, rows: &[String]) -> Result<(), String> {
    let manifest = format!(
        "{{\n  \"schema_version\": 1,\n  \"renderer\": \"wgpu-headless\",\n  \"status\": \"captured\",\n  \"width\": {WIDTH},\n  \"height\": {HEIGHT},\n  \"cases\": [\n{}\n  ]\n}}\n",
        rows.join(",\n")
    );
    fs::write(dir.join("manifest.json"), manifest).map_err(|error| error.to_string())
}

fn main() -> Result<(), String> {
    let dir = output_dir();
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let mut rows = Vec::with_capacity(CASES.len());
    for case in CASES {
        let state = scene_for(case.id)?;
        let image = match render_offscreen_wgpu(&state, WIDTH, HEIGHT) {
            Ok(image) => image,
            Err(error) if error.starts_with("Failed to request a headless GPU adapter") => {
                return Err(format!("WGPU adapter unavailable: {error}"));
            }
            Err(error) => return Err(error),
        };
        let path = dir.join(format!("{}.png", case.id));
        image.save(&path).map_err(|error| error.to_string())?;
        let opaque = image.pixels().filter(|pixel| pixel.0[3] != 0).count();
        let checksum = fnv1a64(image.as_raw());
        rows.push(format!(
            "    {{\"id\":\"{}\",\"description\":\"{}\",\"path\":\"{}\",\"opaque_pixels\":{},\"rgba_checksum\":\"fnv1a64:{checksum:016x}\"}}",
            case.id,
            case.description,
            path.file_name()
                .expect("WGPU capture path always has a file name")
                .display(),
            opaque,
        ));
    }
    write_manifest(&dir, &rows)
}
