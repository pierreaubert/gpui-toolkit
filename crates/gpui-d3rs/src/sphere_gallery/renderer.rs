//! wgpu-based renderer for the sphere gallery

use bytemuck::{Pod, Zeroable};

mod sphere_gallery_config;
mod sphere_gallery_renderer;

pub use sphere_gallery_config::*;
pub use sphere_gallery_renderer::*;

/// Uniform buffer data (must match shader layout)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    atlas_cols: f32,
    atlas_rows: f32,
    cell_count: f32,
    selected_index: f32,
    hovered_index: f32,
    ambient: f32,
    diffuse: f32,
    _pad: f32,
}
