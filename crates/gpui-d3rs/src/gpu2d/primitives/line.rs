//! Line primitive for GPU rendering

use super::Color4;
use bytemuck::{Pod, Zeroable};

/// Vertex data for line rendering
///
/// Lines are rendered as screen-space capsule SDF quads. Each segment becomes
/// 4 vertices (2 triangles = 6 indices), expanded by one AA pixel along both
/// the perpendicular normal and the tangent so caps are antialiased too.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LineVertex {
    /// Position in pixel coordinates
    pub position: [f32; 2],
    /// Local SDF coordinates in segment space: x along tangent, y along normal
    pub local: [f32; 2],
    /// RGBA color
    pub color: [f32; 4],
    /// Half line width in pixels
    pub half_width: f32,
    /// Half segment length in pixels
    pub half_length: f32,
    /// Padding for 16-byte vertex alignment
    pub _padding: [f32; 2],
}

impl LineVertex {
    pub fn new(
        position: [f32; 2],
        local: [f32; 2],
        color: Color4,
        half_width: f32,
        half_length: f32,
    ) -> Self {
        Self {
            position,
            local,
            color,
            half_width,
            half_length,
            _padding: [0.0; 2],
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
        if width <= 0.0 || !width.is_finite() || color[3] <= 0.0 {
            return;
        }

        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();

        if len < 0.001 {
            return; // Skip degenerate lines
        }

        // Perpendicular normal
        let nx = -dy / len;
        let ny = dx / len;

        let hw = width / 2.0;
        let half_length = len / 2.0;
        let coverage_hw = hw + 1.0;
        let coverage_half_length = half_length + 1.0;
        let tx = dx / len;
        let ty = dy / len;
        let cx = (x0 + x1) * 0.5;
        let cy = (y0 + y1) * 0.5;

        let corner = |local_x: f32, local_y: f32| {
            [
                cx + tx * local_x + nx * local_y,
                cy + ty * local_x + ny * local_y,
            ]
        };

        let base_idx = self.vertices.len() as u32;

        // Four vertices forming a quad
        // v0 --- v1
        // |  \    |
        // |   \   |
        // v2 --- v3
        let local_start_top = [-coverage_half_length, coverage_hw];
        let local_end_top = [coverage_half_length, coverage_hw];
        let local_start_bottom = [-coverage_half_length, -coverage_hw];
        let local_end_bottom = [coverage_half_length, -coverage_hw];

        self.vertices.push(LineVertex::new(
            corner(local_start_top[0], local_start_top[1]),
            local_start_top,
            color,
            hw,
            half_length,
        ));
        self.vertices.push(LineVertex::new(
            corner(local_end_top[0], local_end_top[1]),
            local_end_top,
            color,
            hw,
            half_length,
        ));
        self.vertices.push(LineVertex::new(
            corner(local_start_bottom[0], local_start_bottom[1]),
            local_start_bottom,
            color,
            hw,
            half_length,
        ));
        self.vertices.push(LineVertex::new(
            corner(local_end_bottom[0], local_end_bottom[1]),
            local_end_bottom,
            color,
            hw,
            half_length,
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

impl Default for LineBatch {
    fn default() -> Self {
        Self::new()
    }
}
