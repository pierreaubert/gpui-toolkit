//! Retained mesh rendering state and platform backend hooks.
//!
//! Geometry preparation remains in [`crate::mesh::upload`] so it is usable in
//! headless builds. Platform backends are deliberately thin adapters around
//! the same retained revisions and may be unavailable on a given target.

mod retained;
pub use retained::*;

#[cfg(all(feature = "gpu-2d", not(test)))]
mod shaders;

#[cfg(feature = "gpu-compute")]
pub mod compute;
#[cfg(feature = "gpu-compute")]
pub mod compute_shaders;

#[cfg(all(feature = "gpu-3d", not(test)))]
pub mod renderer3d;
#[cfg(feature = "gpu-3d")]
pub mod shaders3d;
#[cfg(all(feature = "gpu-3d", not(test)))]
pub use renderer3d::Mesh3DRenderer;
#[cfg(all(feature = "gpu-3d", not(test)))]
pub use renderer3d::WgpuMesh3DRenderer;

#[cfg(all(feature = "gpu-2d", not(test)))]
mod wgpu_backend;
#[cfg(all(feature = "gpu-2d", not(test)))]
pub use wgpu_backend::WgpuMeshRenderer;

#[cfg(all(target_os = "macos", feature = "gpu-metal", not(test)))]
mod metal_backend;
#[cfg(all(target_os = "macos", feature = "gpu-metal", not(test)))]
mod shaders_metal;
#[cfg(all(target_os = "macos", feature = "gpu-metal", not(test)))]
pub use metal_backend::MetalMeshRenderer;

#[cfg(feature = "gpui")]
mod element;
#[cfg(feature = "gpui")]
pub use element::{DEFAULT_LOD_THRESHOLD, MeshLodController, MeshSceneElement};

#[cfg(feature = "gpui")]
mod offscreen;
#[cfg(feature = "gpui")]
pub use offscreen::render_offscreen;

/// Stable GPU-facing revision for retained geometry buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GeometryRevision(pub u64);

/// Stable GPU-facing revision for retained scalar buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FieldRevision(pub u64);
