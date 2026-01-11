//! GPU text rendering via font atlas
//!
//! This module provides text rendering using a glyph atlas texture.
//! Glyphs are rasterized on-demand using fontdue and cached in a texture atlas.

mod atlas;
mod rasterizer;

pub use atlas::TextAtlas;
pub use rasterizer::GlyphRasterizer;

use super::primitives::Color4;
use bytemuck::{Pod, Zeroable};

/// Vertex data for text rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TextVertex {
    /// Position in pixel coordinates
    pub position: [f32; 2],
    /// Texture coordinates in atlas
    pub tex_coord: [f32; 2],
    /// RGBA color
    pub color: [f32; 4],
}

impl TextVertex {
    pub fn new(position: [f32; 2], tex_coord: [f32; 2], color: Color4) -> Self {
        Self {
            position,
            tex_coord,
            color,
        }
    }
}

/// Batch of text vertices and indices
pub struct TextBatch {
    pub vertices: Vec<TextVertex>,
    pub indices: Vec<u32>,
}

impl TextBatch {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn vertex_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.vertices)
    }

    pub fn index_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.indices)
    }
}

impl Default for TextBatch {
    fn default() -> Self {
        Self::new()
    }
}
