//! Circle primitive for GPU rendering

use super::Color4;
use bytemuck::{Pod, Zeroable};

/// Vertex data for circle/point rendering
///
/// Circles are rendered as quads with SDF-based anti-aliasing in the fragment shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CircleVertex {
    /// Position in pixel coordinates
    pub position: [f32; 2],
    /// Circle center in pixel coordinates
    pub center: [f32; 2],
    /// Circle radius in pixels
    pub radius: f32,
    /// RGBA color
    pub color: [f32; 4],
}

impl CircleVertex {
    pub fn new(position: [f32; 2], center: [f32; 2], radius: f32, color: Color4) -> Self {
        Self {
            position,
            center,
            radius,
            color,
        }
    }
}

/// Batch of circle vertices and indices
pub struct CircleBatch {
    pub vertices: Vec<CircleVertex>,
    pub indices: Vec<u32>,
}

impl CircleBatch {
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

    /// Add a circle to the batch
    ///
    /// # Arguments
    /// * `cx`, `cy` - Center in pixel coordinates
    /// * `radius` - Radius in pixels
    /// * `color` - RGBA color
    pub fn add_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color4) {
        // Expand bounds for anti-aliasing
        let padding = 2.0;
        let r = radius + padding;

        let base_idx = self.vertices.len() as u32;

        // Four vertices forming a quad around the circle
        // v0 --- v1
        // |   O   |
        // v2 --- v3
        self.vertices
            .push(CircleVertex::new([cx - r, cy - r], [cx, cy], radius, color));
        self.vertices
            .push(CircleVertex::new([cx + r, cy - r], [cx, cy], radius, color));
        self.vertices
            .push(CircleVertex::new([cx - r, cy + r], [cx, cy], radius, color));
        self.vertices
            .push(CircleVertex::new([cx + r, cy + r], [cx, cy], radius, color));

        // Two triangles: (0, 2, 1) and (1, 2, 3)
        self.indices.extend_from_slice(&[
            base_idx,
            base_idx + 2,
            base_idx + 1,
            base_idx + 1,
            base_idx + 2,
            base_idx + 3,
        ]);
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

impl Default for CircleBatch {
    fn default() -> Self {
        Self::new()
    }
}
