//! Mesh generation for 3D surfaces

use bytemuck::{Pod, Zeroable};

mod generate;
mod gpu_vertex;
mod surface_mesh;
#[cfg(test)]
mod tests;

pub use generate::*;
pub use surface_mesh::*;

/// GPU vertex representation (must match shader layout)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub value: f32,
    pub _padding: f32, // Align to 32 bytes
}
