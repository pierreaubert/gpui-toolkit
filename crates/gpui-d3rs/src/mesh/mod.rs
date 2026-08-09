//! Unstructured triangle mesh data model, validation, topology, contours,
//! picking, and revolve. Feature-independent: no GPUI, no wgpu.

mod levels;
mod marching_triangles;
mod model;
mod revolve;
mod spatial;
mod topology;
pub use levels::ContourLevels;
pub use marching_triangles::{ContourBand, IsolineSegment, MarchingTriangles};
pub use model::{MeshValidationError, ScalarAssociation, ScalarField, TriangleMesh};
pub use revolve::{revolve, revolve_field, RevolveSpec, RevolvedMesh};
pub use spatial::{barycentric_2d, project_2d, CoordinateAxis, MeshBounds, TriGridIndex};
pub use topology::MeshTopology;
