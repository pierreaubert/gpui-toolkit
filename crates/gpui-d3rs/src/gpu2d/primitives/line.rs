//! Line primitive for GPU rendering

use super::Color4;
use bytemuck::{Pod, Zeroable};

/// Vertex data for line rendering
///
/// Lines are rendered as quads expanded along the perpendicular normal.
/// Each line segment becomes 4 vertices (2 triangles = 6 indices).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LineVertex {
    /// Position in pixel coordinates
    pub position: [f32; 2],
    /// Normal vector (perpendicular to line direction) scaled by half line width
    pub normal: [f32; 2],
    /// RGBA color
    pub color: [f32; 4],
}

impl LineVertex {
    pub fn new(position: [f32; 2], normal: [f32; 2], color: Color4) -> Self {
        Self {
            position,
            normal,
            color,
        }
    }
}

/// Batch of line vertices and indices
pub struct LineBatch {
    pub vertices: Vec<LineVertex>,
    pub indices: Vec<u32>,
}

impl LineBatch {
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

    /// Add a line segment to the batch
    ///
    /// # Arguments
    /// * `x0`, `y0` - Start point in pixel coordinates
    /// * `x1`, `y1` - End point in pixel coordinates
    /// * `width` - Line width in pixels
    /// * `color` - RGBA color
    pub fn add_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, color: Color4) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();

        if len < 0.001 {
            return; // Skip degenerate lines
        }

        // Perpendicular normal
        let nx = -dy / len;
        let ny = dx / len;

        // Half width for expansion
        let hw = width / 2.0;
        let normal_pos = [nx * hw, ny * hw];
        let normal_neg = [-nx * hw, -ny * hw];

        let base_idx = self.vertices.len() as u32;

        // Four vertices forming a quad
        // v0 --- v1
        // |  \    |
        // |   \   |
        // v2 --- v3
        self.vertices
            .push(LineVertex::new([x0, y0], normal_pos, color)); // v0: start + normal
        self.vertices
            .push(LineVertex::new([x1, y1], normal_pos, color)); // v1: end + normal
        self.vertices
            .push(LineVertex::new([x0, y0], normal_neg, color)); // v2: start - normal
        self.vertices
            .push(LineVertex::new([x1, y1], normal_neg, color)); // v3: end - normal

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

impl Default for LineBatch {
    fn default() -> Self {
        Self::new()
    }
}
