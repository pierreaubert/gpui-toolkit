//! Unstructured triangle mesh data model, validation, topology, contours,
//! picking, and revolve. Feature-independent: no GPUI, no wgpu.

mod bvh;
mod decimate;
#[cfg(any(feature = "gpu-2d", feature = "gpu-3d", feature = "gpu-compute"))]
pub mod gpu;
mod levels;
mod marching_triangles;
mod model;
mod revolve;
mod spatial;
mod topology;
mod upload;
pub use bvh::MeshBvh;
pub use decimate::decimate_vertex_clustering;
pub use levels::ContourLevels;
pub use marching_triangles::{ContourBand, IsolineSegment, MarchingTriangles};
pub use model::{MeshValidationError, ScalarAssociation, ScalarField, TriangleMesh};
pub use revolve::{RevolveSpec, RevolvedMesh, revolve, revolve_field};
pub use spatial::{CoordinateAxis, MeshBounds, TriGridIndex, barycentric_2d, project_2d};
pub use topology::MeshTopology;
pub use upload::{
    MAX_GPU_UPLOAD_BYTES, MeshUpload, compute_smooth_normals, expand_cell_shading, prepare_field,
    prepare_upload, upload_chunks,
};
