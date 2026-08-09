//! Unstructured triangle mesh data model, validation, topology, contours,
//! picking, and revolve. Feature-independent: no GPUI, no wgpu.

mod model;
pub use model::{MeshValidationError, ScalarAssociation, ScalarField, TriangleMesh};
