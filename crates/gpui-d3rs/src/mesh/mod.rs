//! Unstructured triangle mesh data model, validation, topology, contours,
//! picking, and revolve. Feature-independent: no GPUI, no wgpu.

mod model;
mod topology;
pub use model::{MeshValidationError, ScalarAssociation, ScalarField, TriangleMesh};
pub use topology::MeshTopology;
