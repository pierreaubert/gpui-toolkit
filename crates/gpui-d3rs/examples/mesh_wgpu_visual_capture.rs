//! Capture retained MeshPlot scenes through the real WGPU adapter-backed
//! renderer. The output is consumed by the WGPU visual QA lane.

use d3rs::gpu3d::Camera3D;
use d3rs::mesh::gpu::{
    GeometryRevision, MeshColorConfig, MeshSceneState, render_offscreen_wgpu_with_camera,
};
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
    comparison_id: &'static str,
}

const CASES: &[CaptureCase] = &[
    CaptureCase {
        id: "mesh",
        description: "mesh-only",
        comparison_id: "px.mesh_plot.mesh_only",
    },
    CaptureCase {
        id: "smooth",
        description: "smooth vertex scalar fill",
        comparison_id: "px.mesh_plot.smooth_fill",
    },
    CaptureCase {
        id: "cell",
        description: "flat cell scalar fill",
        comparison_id: "px.mesh_plot.flat_fill",
    },
    CaptureCase {
        id: "wireframe",
        description: "smooth fill with wireframe",
        comparison_id: "px.mesh_plot.wireframe",
    },
    CaptureCase {
        id: "isoline",
        description: "smooth fill with isoline",
        comparison_id: "px.mesh_plot.isolines",
    },
    CaptureCase {
        id: "revolve",
        description: "axisymmetric revolved surface",
        comparison_id: "px.mesh_plot.revolve",
    },
];

const EXPANDED_CASES: &[CaptureCase] = &[
    CaptureCase {
        id: "camera",
        description: "alternate camera projection",
        comparison_id: "px.mesh_plot.state.camera",
    },
    CaptureCase {
        id: "range",
        description: "displayed scalar range",
        comparison_id: "px.mesh_plot.state.range",
    },
    CaptureCase {
        id: "masked",
        description: "NaN-masked scalar surface",
        comparison_id: "px.mesh_plot.state.masked",
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

fn square_cell_field() -> ScalarField {
    ScalarField {
        id: "wgpu-visual-cell-field".into(),
        label: "cell scalar".into(),
        unit: Some("arb".into()),
        values: Arc::from([0.15, 0.85]),
        association: ScalarAssociation::Cell,
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

fn scene_camera(id: &str) -> Camera3D {
    if id == "camera" {
        Camera3D::default()
            .with_position(glam::Vec3::new(1.25, 2.75, 3.5))
            .with_target(glam::Vec3::new(0.15, 0.0, 0.0))
            .with_aspect(WIDTH as f32 / HEIGHT as f32)
    } else {
        Camera3D::default()
    }
}

fn scene_for(id: &str) -> Result<(MeshSceneState, Camera3D), String> {
    let camera = scene_camera(id);
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
        return Ok((state, camera));
    }

    let (field, wireframe, isoline_step, range) = match id {
        "mesh" => (None, false, 0.0, [0.0, 1.0]),
        "smooth" | "camera" => (Some(square_field()), false, 0.0, [0.0, 1.0]),
        "range" => (Some(square_field()), false, 0.0, [0.25, 0.75]),
        "masked" => {
            let mut field = square_field();
            field.values = Arc::from([0.0, f64::NAN, 1.0, 0.25]);
            (Some(field), false, 0.0, [0.0, 1.0])
        }
        "cell" => (Some(square_cell_field()), false, 0.0, [0.0, 1.0]),
        "wireframe" => (Some(square_field()), true, 0.0, [0.0, 1.0]),
        "isoline" => (Some(square_field()), false, 0.5, [0.0, 1.0]),
        other => return Err(format!("unknown WGPU visual capture case: {other}")),
    };
    let mesh = square_mesh();
    let upload = prepare_upload(&mesh, &MeshTopology::build(&mesh.triangles));
    let mut state = MeshSceneState {
        geometry_rev: GeometryRevision(1),
        upload: Some(upload),
        color: MeshColorConfig {
            range,
            wireframe,
            isoline_step,
            isoline_width_px: if isoline_step == 0.0 { 0.0 } else { 1.5 },
            unlit: true,
            ..MeshColorConfig::default()
        },
        ..MeshSceneState::default()
    };
    if let Some(field) = field {
        let values = field.values.iter().map(|value| *value as f32).collect();
        if field.association == ScalarAssociation::Cell {
            state.upload.as_mut().unwrap().cell_values_f32 = Some(values);
        } else {
            state.upload.as_mut().unwrap().values_f32 = Some(values);
        }
    }
    Ok((state, camera))
}

fn expanded_case_set() -> bool {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--case-set" {
            return args.next().as_deref() == Some("expanded");
        }
    }
    false
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
    let cases = if expanded_case_set() {
        EXPANDED_CASES
    } else {
        CASES
    };
    let mut rows = Vec::with_capacity(cases.len());
    for case in cases {
        let (state, camera) = scene_for(case.id)?;
        let image = match render_offscreen_wgpu_with_camera(&state, WIDTH, HEIGHT, &camera) {
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
            "    {{\"id\":\"{}\",\"comparison_id\":\"{}\",\"description\":\"{}\",\"path\":\"{}\",\"opaque_pixels\":{},\"rgba_checksum\":\"fnv1a64:{checksum:016x}\"}}",
            case.id,
            case.comparison_id,
            case.description,
            path.file_name()
                .expect("WGPU capture path always has a file name")
                .display(),
            opaque,
        ));
    }
    write_manifest(&dir, &rows)
}
