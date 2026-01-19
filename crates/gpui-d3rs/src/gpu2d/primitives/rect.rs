//! Rectangle primitive for GPU rendering

use super::{Color4, Rect};
use bytemuck::{Pod, Zeroable};

/// Vertex data for rectangle rendering with rounded corners
///
/// Rectangles are rendered as quads with SDF-based corner rounding in the fragment shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RectVertex {
    /// Position in pixel coordinates
    pub position: [f32; 2],
    /// Rectangle min corner (for SDF calculation)
    pub rect_min: [f32; 2],
    /// Rectangle max corner (for SDF calculation)
    pub rect_max: [f32; 2],
    /// Corner radius in pixels
    pub corner_radius: f32,
    /// RGBA color
    pub color: [f32; 4],
}

impl RectVertex {
    pub fn new(
        position: [f32; 2],
        rect_min: [f32; 2],
        rect_max: [f32; 2],
        corner_radius: f32,
        color: Color4,
    ) -> Self {
        Self {
            position,
            rect_min,
            rect_max,
            corner_radius,
            color,
        }
    }
}

/// Batch of rectangle vertices and indices
pub struct RectBatch {
    pub vertices: Vec<RectVertex>,
    pub indices: Vec<u32>,
}

impl RectBatch {
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

    /// Add a rectangle to the batch
    ///
    /// # Arguments
    /// * `rect` - Rectangle bounds in pixel coordinates
    /// * `color` - RGBA color
    /// * `corner_radius` - Corner radius in pixels (0 for sharp corners)
    pub fn add_rect(&mut self, rect: Rect, color: Color4, corner_radius: f32) {
        let rect_min = rect.min();
        let rect_max = rect.max();

        // Expand bounds slightly for anti-aliasing
        let padding = 1.0;
        let expanded_min = [rect_min[0] - padding, rect_min[1] - padding];
        let expanded_max = [rect_max[0] + padding, rect_max[1] + padding];

        let base_idx = self.vertices.len() as u32;

        // Four vertices forming a quad
        // v0 --- v1
        // |       |
        // v2 --- v3
        self.vertices.push(RectVertex::new(
            expanded_min,
            rect_min,
            rect_max,
            corner_radius,
            color,
        ));
        self.vertices.push(RectVertex::new(
            [expanded_max[0], expanded_min[1]],
            rect_min,
            rect_max,
            corner_radius,
            color,
        ));
        self.vertices.push(RectVertex::new(
            [expanded_min[0], expanded_max[1]],
            rect_min,
            rect_max,
            corner_radius,
            color,
        ));
        self.vertices.push(RectVertex::new(
            expanded_max,
            rect_min,
            rect_max,
            corner_radius,
            color,
        ));

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

impl Default for RectBatch {
    fn default() -> Self {
        Self::new()
    }
}
