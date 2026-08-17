#![cfg(feature = "gpu-3d")]

// The task is intentionally limited to the new renderer/shader files. Import
// those files directly so these cache tests do not require edits to the
// in-progress mesh/gpu/mod.rs module wiring.
pub use d3rs::mesh::gpu::{FieldRevision, GeometryRevision, MeshGpuRenderer};
pub use d3rs::{gpu3d, mesh};

#[path = "../src/mesh/gpu/renderer3d.rs"]
mod renderer3d;
#[path = "../src/mesh/gpu/shaders3d.rs"]
mod shaders3d;

use d3rs::mesh::{MeshTopology, TriangleMesh, expand_cell_shading, prepare_upload};
use renderer3d::Mesh3DRenderer;
use std::sync::Arc;

fn square_upload() -> mesh::MeshUpload {
    let mesh = TriangleMesh {
        id: "square".into(),
        positions: Arc::from([
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: None,
        cell_ids: None,
    };
    prepare_upload(&mesh, &MeshTopology::build(&mesh.triangles))
}

fn square_upload_with_cell_field() -> mesh::MeshUpload {
    let mut upload = square_upload();
    upload.cell_values_f32 = Some(vec![0.2, 0.8]);
    upload
}

#[test]
fn camera_only_change_does_not_reupload() {
    let mut renderer = Mesh3DRenderer::new();
    renderer.upload_geometry(GeometryRevision(1), &square_upload());
    let mut camera = gpu3d::Camera3D::default();
    camera.position.x += 1.0;
    renderer.set_camera(&camera);

    assert_eq!(renderer.geometry_revision(), Some(GeometryRevision(1)));
    assert_eq!(renderer.upload_count(), 1);
    assert_eq!(renderer.upload_bytes(), square_upload().geometry_byte_len());
}

#[test]
fn stale_geometry_upload_cannot_replace_newer_retained_geometry() {
    let mut renderer = Mesh3DRenderer::new();
    renderer.upload_geometry(GeometryRevision(4), &square_upload());
    renderer.upload_geometry(GeometryRevision(3), &square_upload());

    assert_eq!(renderer.geometry_revision(), Some(GeometryRevision(4)));
    assert_eq!(renderer.upload_count(), 1);
    assert_eq!(renderer.upload_bytes(), square_upload().geometry_byte_len());
}

#[test]
fn field_write_keeps_geometry() {
    let mut renderer = Mesh3DRenderer::new();
    renderer.upload_geometry(GeometryRevision(1), &square_upload());
    renderer.write_field(FieldRevision(2), &[0.1, 0.2, 0.3, 0.4]);

    assert_eq!(renderer.geometry_revision(), Some(GeometryRevision(1)));
    assert_eq!(renderer.field_revision(), Some(FieldRevision(2)));
    assert_eq!(renderer.upload_count(), 1);
    assert_eq!(renderer.upload_bytes(), square_upload().geometry_byte_len());
}

#[test]
fn stale_field_write_cannot_overwrite_newer_retained_values() {
    let mut renderer = Mesh3DRenderer::new();
    renderer.upload_geometry(GeometryRevision(1), &square_upload());
    renderer.write_field(FieldRevision(4), &[0.4, 0.3, 0.2, 0.1]);
    renderer.write_field(FieldRevision(3), &[0.1, 0.2, 0.3, 0.4]);

    assert_eq!(renderer.field_revision(), Some(FieldRevision(4)));
    assert_eq!(renderer.backend().field_values(), &[0.4, 0.3, 0.2, 0.1]);
    assert_eq!(renderer.backend().state().field_write_count, 1);
    assert_eq!(renderer.upload_count(), 1);
}

#[test]
fn cell_field_duplicates_vertices_for_flat_shading() {
    let upload = square_upload_with_cell_field();
    let gpu = expand_cell_shading(&upload);

    assert_eq!(gpu.positions_f32.len(), upload.indices.len());
    let values = gpu
        .values_f32
        .expect("cell field must become vertex values");
    assert_eq!(values.len(), upload.indices.len());
    assert_eq!(values[0], values[1]);
    assert_eq!(values[1], values[2]);
    assert_eq!(values[3], values[4]);
    assert_eq!(values[4], values[5]);
}

#[test]
fn shader_sources_include_surface_and_wireframe_entries() {
    assert!(shaders3d::wgsl().contains("fn fs_main"));
    assert!(shaders3d::wgsl().contains("fn fs_wireframe"));
    assert!(shaders3d::wgsl().contains("var<storage, read> values"));
    assert!(shaders3d::wgsl().contains("@builtin(vertex_index) vertex_index"));
    assert!(shaders3d::wgsl().contains("uniforms.value_range.z > 0.5"));
    assert!(shaders3d::msl().contains("fragment float4 fs_main"));
    assert!(shaders3d::msl().contains("fragment float4 fs_wireframe"));
    assert!(shaders3d::msl().contains("const device float* values [[buffer(2)]]"));
    assert!(shaders3d::msl().contains("uniforms.value_range.z > 0.5 ? values[vertex_id] : 0.0"));
    // The Metal vertex ABI carries the scalar slot and trailing padding so
    // vertex_id advances over the same 48-byte stride as Rust's upload.
    assert!(shaders3d::msl().contains("value [[attribute(2)]]"));
}
