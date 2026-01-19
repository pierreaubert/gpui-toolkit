//! Triangle primitive for filled polygon rendering

use super::Color4;
use bytemuck::{Pod, Zeroable};

/// Vertex for a triangle (simple position + color)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TriangleVertex {
    /// Position in pixel coordinates
    pub position: [f32; 2],
    /// RGBA color
    pub color: [f32; 4],
}

impl TriangleVertex {
    pub fn new(position: [f32; 2], color: Color4) -> Self {
        Self { position, color }
    }
}

/// Batch of triangles for efficient rendering
pub struct TriangleBatch {
    pub vertices: Vec<TriangleVertex>,
    pub indices: Vec<u32>,
}

impl TriangleBatch {
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
        self.indices.is_empty()
    }

    /// Add a single triangle
    pub fn add_triangle(&mut self, p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], color: Color4) {
        let base = self.vertices.len() as u32;

        self.vertices.push(TriangleVertex::new(p0, color));
        self.vertices.push(TriangleVertex::new(p1, color));
        self.vertices.push(TriangleVertex::new(p2, color));

        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// Add a polygon by triangulating it using ear clipping
    /// Points should be in counter-clockwise order for front-facing
    pub fn add_polygon(&mut self, points: &[[f32; 2]], color: Color4) {
        if points.len() < 3 {
            return;
        }

        // Simple ear clipping triangulation
        let triangles = triangulate_polygon(points);

        for tri in triangles {
            self.add_triangle(tri[0], tri[1], tri[2], color);
        }
    }

    pub fn vertex_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.vertices)
    }

    pub fn index_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.indices)
    }
}

impl Default for TriangleBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple ear clipping triangulation for convex/simple polygons
fn triangulate_polygon(points: &[[f32; 2]]) -> Vec<[[f32; 2]; 3]> {
    if points.len() < 3 {
        return Vec::new();
    }

    if points.len() == 3 {
        return vec![[points[0], points[1], points[2]]];
    }

    // For simple/convex polygons, fan triangulation works
    // For complex polygons, we use ear clipping

    let mut result = Vec::with_capacity(points.len() - 2);
    let mut remaining: Vec<usize> = (0..points.len()).collect();

    // Check if polygon is clockwise and reverse if needed
    let area = signed_polygon_area(points);
    if area > 0.0 {
        remaining.reverse();
    }

    let mut safety = points.len() * 2;
    while remaining.len() > 3 && safety > 0 {
        safety -= 1;
        let mut found_ear = false;

        for i in 0..remaining.len() {
            let prev_idx = if i == 0 { remaining.len() - 1 } else { i - 1 };
            let next_idx = (i + 1) % remaining.len();

            let prev = points[remaining[prev_idx]];
            let curr = points[remaining[i]];
            let next = points[remaining[next_idx]];

            // Check if this is a convex vertex (ear candidate)
            if !is_convex(prev, curr, next) {
                continue;
            }

            // Check if any other point is inside this triangle
            let mut is_ear = true;
            for j in 0..remaining.len() {
                if j == prev_idx || j == i || j == next_idx {
                    continue;
                }
                let p = points[remaining[j]];
                if point_in_triangle(p, prev, curr, next) {
                    is_ear = false;
                    break;
                }
            }

            if is_ear {
                result.push([prev, curr, next]);
                remaining.remove(i);
                found_ear = true;
                break;
            }
        }

        if !found_ear {
            // Fallback to fan triangulation if ear clipping fails
            break;
        }
    }

    // Handle remaining triangle
    if remaining.len() == 3 {
        result.push([
            points[remaining[0]],
            points[remaining[1]],
            points[remaining[2]],
        ]);
    } else if remaining.len() > 3 {
        // Fallback: fan triangulation from first vertex
        let first = points[remaining[0]];
        for i in 1..remaining.len() - 1 {
            result.push([first, points[remaining[i]], points[remaining[i + 1]]]);
        }
    }

    result
}

fn signed_polygon_area(points: &[[f32; 2]]) -> f32 {
    let mut area = 0.0;
    let n = points.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += points[i][0] * points[j][1];
        area -= points[j][0] * points[i][1];
    }
    area / 2.0
}

fn is_convex(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    cross_product_2d(a, b, c) < 0.0
}

fn cross_product_2d(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn point_in_triangle(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);

    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;

    !(has_neg && has_pos)
}

fn sign(p1: [f32; 2], p2: [f32; 2], p3: [f32; 2]) -> f32 {
    (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])
}
