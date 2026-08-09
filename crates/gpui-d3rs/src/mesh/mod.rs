//! Unstructured triangle mesh data model, validation, topology, contours,
//! picking, and revolve. Feature-independent: no GPUI, no wgpu.

mod model;
mod spatial;
mod topology;
pub use model::{MeshValidationError, ScalarAssociation, ScalarField, TriangleMesh};
pub use spatial::{barycentric_2d, project_2d, CoordinateAxis, MeshBounds, TriGridIndex};
pub use topology::MeshTopology;
