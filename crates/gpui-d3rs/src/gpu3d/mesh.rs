//! Mesh generation for 3D surfaces

use super::config::SurfacePlotType;
use super::data::SurfaceData;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// GPU vertex representation (must match shader layout)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub value: f32,
    pub _padding: f32, // Align to 32 bytes
}

impl GpuVertex {
    pub fn new(position: Vec3, normal: Vec3, value: f32) -> Self {
        Self {
            position: position.to_array(),
            normal: normal.to_array(),
            value,
            _padding: 0.0,
        }
    }
}

/// Surface mesh containing GPU-ready vertex and index data
#[derive(Debug)]
pub struct SurfaceMesh {
    /// Vertex data
    pub vertices: Vec<GpuVertex>,
    /// Triangle indices
    pub indices: Vec<u32>,
    /// Number of vertices
    pub vertex_count: usize,
    /// Number of indices
    pub index_count: usize,
    /// Number of grid columns (X dimension)
    pub x_count: usize,
    /// Number of grid rows (Y dimension)
    pub y_count: usize,
}

impl SurfaceMesh {
    /// Generate a surface mesh from surface data
    pub fn from_data(data: &SurfaceData, plot_type: SurfacePlotType) -> Self {
        let x_count = data.x_count();
        let y_count = data.y_count();

        if x_count < 2 || y_count < 2 {
            return Self::empty();
        }

        // Generate vertices
        let mut vertices = Vec::with_capacity(x_count * y_count);

        for yi in 0..y_count {
            for xi in 0..x_count {
                let x = data.x_values[xi];
                let y = data.y_values[yi];
                let z = data.z_values[yi][xi];

                // Normalize to [-1, 1] range for x and y, and scale z appropriately
                let nx = data.normalize_x(x);
                let ny = data.normalize_y(y);
                let nz = data.normalize_z(z);

                let position = match plot_type {
                    SurfacePlotType::Cartesian => {
                        // Map normalized z [0,1] to height [-0.5, 0.5]
                        let height = nz - 0.5;
                        Vec3::new(nx, height, ny)
                    }
                    SurfacePlotType::Spherical => {
                        // Map X (Freq) to Latitude (Phi): [-1, 1] -> [-PI/2, PI/2]
                        // Map Y (Angle) to Longitude (Theta): [-1, 1] -> [-PI, PI]
                        // Map Z (SPL) to Radius? Or just color.
                        // Let's use Radius = 1.0 + nz * 0.2 (slight extrusion)

                        let phi = nx * std::f32::consts::FRAC_PI_2; // -90 to 90 deg
                        let theta = ny * std::f32::consts::PI; // -180 to 180 deg
                        let radius = 1.0; // Unit sphere

                        // Spherical to Cartesian
                        // y is up (sin phi)
                        // x, z are horizontal plane
                        let y_pos = radius * phi.sin();
                        let r_xz = radius * phi.cos();
                        let x_pos = r_xz * theta.sin();
                        let z_pos = r_xz * theta.cos();

                        Vec3::new(x_pos, y_pos, z_pos)
                    }
                };

                let value = nz;

                // Placeholder normal - will be computed after
                vertices.push(GpuVertex::new(position, Vec3::Y, value));
            }
        }

        // Compute normals using central differences
        Self::compute_normals(&mut vertices, x_count, y_count);

        // Generate triangle indices (two triangles per grid cell)
        let mut indices = Vec::with_capacity((x_count - 1) * (y_count - 1) * 6);

        for yi in 0..(y_count - 1) {
            for xi in 0..(x_count - 1) {
                let i00 = (yi * x_count + xi) as u32;
                let i10 = (yi * x_count + xi + 1) as u32;
                let i01 = ((yi + 1) * x_count + xi) as u32;
                let i11 = ((yi + 1) * x_count + xi + 1) as u32;

                // First triangle
                indices.push(i00);
                indices.push(i10);
                indices.push(i01);

                // Second triangle
                indices.push(i10);
                indices.push(i11);
                indices.push(i01);
            }
        }

        Self {
            vertex_count: vertices.len(),
            index_count: indices.len(),
            vertices,
            indices,
            x_count,
            y_count,
        }
    }

    /// Create an empty mesh
    pub fn empty() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_count: 0,
            index_count: 0,
            x_count: 0,
            y_count: 0,
        }
    }

    /// Compute vertex normals using central differences
    fn compute_normals(vertices: &mut [GpuVertex], x_count: usize, y_count: usize) {
        for yi in 0..y_count {
            for xi in 0..x_count {
                let idx = yi * x_count + xi;

                // Get neighboring heights
                let get_pos = |xi: usize, yi: usize| -> Vec3 {
                    let i = yi * x_count + xi;
                    Vec3::from_array(vertices[i].position)
                };

                // Use central differences where possible, forward/backward at edges
                let pos = get_pos(xi, yi);

                let dx = if xi == 0 {
                    get_pos(xi + 1, yi) - pos
                } else if xi == x_count - 1 {
                    pos - get_pos(xi - 1, yi)
                } else {
                    (get_pos(xi + 1, yi) - get_pos(xi - 1, yi)) * 0.5
                };

                let dy = if yi == 0 {
                    get_pos(xi, yi + 1) - pos
                } else if yi == y_count - 1 {
                    pos - get_pos(xi, yi - 1)
                } else {
                    (get_pos(xi, yi + 1) - get_pos(xi, yi - 1)) * 0.5
                };

                // Normal is cross product of tangent vectors
                let normal = dy.cross(dx).normalize_or_zero();

                // Ensure normal points upward (positive Y component)
                let normal = if normal.y < 0.0 { -normal } else { normal };

                vertices[idx].normal = normal.to_array();
            }
        }
    }

    /// Get vertex buffer data as bytes
    pub fn vertex_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.vertices)
    }

    /// Get index buffer data as bytes
    pub fn index_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.indices)
    }

    /// Check if mesh is empty
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}

/// Generate wireframe indices for the surface mesh
pub fn generate_wireframe_indices(x_count: usize, y_count: usize) -> Vec<u32> {
    let mut indices = Vec::new();

    // Horizontal lines
    for yi in 0..y_count {
        for xi in 0..(x_count - 1) {
            let i0 = (yi * x_count + xi) as u32;
            let i1 = (yi * x_count + xi + 1) as u32;
            indices.push(i0);
            indices.push(i1);
        }
    }

    // Vertical lines
    for yi in 0..(y_count - 1) {
        for xi in 0..x_count {
            let i0 = (yi * x_count + xi) as u32;
            let i1 = ((yi + 1) * x_count + xi) as u32;
            indices.push(i0);
            indices.push(i1);
        }
    }

    indices
}

/// Generate a bounding box mesh for the surface (for grid rendering)
pub fn generate_bounding_box_mesh() -> SurfaceMesh {
    let mut vertices = Vec::with_capacity(8);
    let mut indices = Vec::with_capacity(36);

    // Box corners: [-1, 1] x [-0.5, 0.5] x [-1, 1]
    // Matches the normalized surface coordinates
    let min = Vec3::new(-1.0, -0.5, -1.0);
    let max = Vec3::new(1.0, 0.5, 1.0);

    // 8 corners
    let corners = [
        Vec3::new(min.x, min.y, min.z), // 0: 000
        Vec3::new(max.x, min.y, min.z), // 1: 100
        Vec3::new(min.x, max.y, min.z), // 2: 010
        Vec3::new(max.x, max.y, min.z), // 3: 110
        Vec3::new(min.x, min.y, max.z), // 4: 001
        Vec3::new(max.x, min.y, max.z), // 5: 101
        Vec3::new(min.x, max.y, max.z), // 6: 011
        Vec3::new(max.x, max.y, max.z), // 7: 111
    ];

    for pos in corners {
        vertices.push(GpuVertex::new(pos, Vec3::ZERO, 0.0));
    }

    // Indices for 12 triangles (6 faces)
    // We want to see INSIDE faces, so winding order matters.
    // Standard CCW winding for outside faces.
    // If we use FrontFace::Ccw and CullMode::Front, we render back faces.
    // So we generate standard box indices.

    // Front (Z=0)
    indices.extend_from_slice(&[0, 2, 1, 1, 2, 3]);
    // Back (Z=1)
    indices.extend_from_slice(&[5, 7, 4, 4, 7, 6]);
    // Left (X=0)
    indices.extend_from_slice(&[4, 6, 0, 0, 6, 2]);
    // Right (X=1)
    indices.extend_from_slice(&[1, 3, 5, 5, 3, 7]);
    // Bottom (Y=-0.5)
    indices.extend_from_slice(&[4, 0, 5, 5, 0, 1]);
    // Top (Y=0.5)
    indices.extend_from_slice(&[2, 6, 3, 3, 6, 7]);

    SurfaceMesh {
        vertices,
        indices,
        vertex_count: 8,
        index_count: 36,
        x_count: 0, // Not applicable for bounding box
        y_count: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu3d::config::SurfacePlotType;

    #[test]
    fn test_mesh_generation() {
        let data =
            SurfaceData::from_function((-1.0, 1.0), (-1.0, 1.0), 10, 10, |x, y| x * x + y * y);
        let mesh = SurfaceMesh::from_data(&data, SurfacePlotType::Cartesian);

        assert_eq!(mesh.vertex_count, 100); // 10 * 10
        assert_eq!(mesh.index_count, 9 * 9 * 6); // (10-1) * (10-1) * 6
    }

    #[test]
    fn test_mesh_normals() {
        // Flat surface should have normals pointing up
        let data = SurfaceData::from_function((-1.0, 1.0), (-1.0, 1.0), 5, 5, |_, _| 0.0);
        let mesh = SurfaceMesh::from_data(&data, SurfacePlotType::Cartesian);

        for vertex in &mesh.vertices {
            // Normal should be approximately (0, 1, 0) for flat surface
            assert!(
                vertex.normal[1] > 0.9,
                "Normal Y component should be high for flat surface"
            );
        }
    }

    #[test]
    fn test_wireframe_indices_square_3x3() {
        // 3x3 grid: 3 columns (x), 3 rows (y)
        let indices = generate_wireframe_indices(3, 3);

        // Horizontal lines: y_count rows * (x_count - 1) segments = 3 * 2 = 6 segments = 12 indices
        // Vertical lines: (y_count - 1) rows * x_count columns = 2 * 3 = 6 segments = 12 indices
        // Total: 24 indices
        assert_eq!(
            indices.len(),
            24,
            "3x3 grid should have 24 wireframe indices"
        );

        // Verify some specific line segments
        // First horizontal line (y=0): 0-1, 1-2
        assert!(
            indices.contains(&0) && indices.contains(&1),
            "Should have edge 0-1"
        );
        assert!(
            indices.contains(&1) && indices.contains(&2),
            "Should have edge 1-2"
        );
    }

    #[test]
    fn test_wireframe_indices_minimum_2x2() {
        // Minimum valid grid: 2x2
        let indices = generate_wireframe_indices(2, 2);

        // Horizontal: 2 rows * 1 segment = 2 segments = 4 indices
        // Vertical: 1 row * 2 columns = 2 segments = 4 indices
        // Total: 8 indices
        assert_eq!(indices.len(), 8, "2x2 grid should have 8 wireframe indices");

        // Verify all 4 edges of the quad are present
        // Vertices: 0(0,0) 1(1,0) 2(0,1) 3(1,1)
        // Horizontal: 0-1, 2-3
        // Vertical: 0-2, 1-3
        let expected_pairs = vec![(0, 1), (2, 3), (0, 2), (1, 3)];
        for (a, b) in expected_pairs {
            let has_edge = indices
                .windows(2)
                .step_by(2)
                .any(|w| (w[0] == a && w[1] == b) || (w[0] == b && w[1] == a));
            assert!(has_edge, "Should have edge {}-{}", a, b);
        }
    }

    #[test]
    fn test_wireframe_indices_wide_grid() {
        // Wide grid: 10 columns (x), 3 rows (y) = 30 vertices
        let x_count = 10;
        let y_count = 3;
        let indices = generate_wireframe_indices(x_count, y_count);

        // Horizontal: y_count * (x_count - 1) = 3 * 9 = 27 segments = 54 indices
        // Vertical: (y_count - 1) * x_count = 2 * 10 = 20 segments = 40 indices
        // Total: 94 indices
        let expected = 2 * (y_count * (x_count - 1) + (y_count - 1) * x_count);
        assert_eq!(
            indices.len(),
            expected,
            "Wide 10x3 grid should have {} wireframe indices",
            expected
        );

        // Verify all indices are within bounds
        let vertex_count = (x_count * y_count) as u32;
        for &idx in &indices {
            assert!(
                idx < vertex_count,
                "Index {} out of bounds (max: {})",
                idx,
                vertex_count - 1
            );
        }
    }

    #[test]
    fn test_wireframe_indices_tall_grid() {
        // Tall grid: 3 columns (x), 10 rows (y) = 30 vertices
        let x_count = 3;
        let y_count = 10;
        let indices = generate_wireframe_indices(x_count, y_count);

        // Horizontal: y_count * (x_count - 1) = 10 * 2 = 20 segments = 40 indices
        // Vertical: (y_count - 1) * x_count = 9 * 3 = 27 segments = 54 indices
        // Total: 94 indices
        let expected = 2 * (y_count * (x_count - 1) + (y_count - 1) * x_count);
        assert_eq!(
            indices.len(),
            expected,
            "Tall 3x10 grid should have {} wireframe indices",
            expected
        );

        // Verify all indices are within bounds
        let vertex_count = (x_count * y_count) as u32;
        for &idx in &indices {
            assert!(
                idx < vertex_count,
                "Index {} out of bounds (max: {})",
                idx,
                vertex_count - 1
            );
        }
    }

    #[test]
    fn test_wireframe_indices_line_connectivity() {
        // Test that wireframe indices form proper line segments
        // For a 4x3 grid, verify connectivity pattern
        let x_count = 4;
        let y_count = 3;
        let indices = generate_wireframe_indices(x_count, y_count);

        // Each pair of indices should form a line between adjacent vertices
        for chunk in indices.chunks_exact(2) {
            let i0 = chunk[0] as usize;
            let i1 = chunk[1] as usize;

            let x0 = i0 % x_count;
            let y0 = i0 / x_count;
            let x1 = i1 % x_count;
            let y1 = i1 / x_count;

            // Lines should connect adjacent vertices (horizontally or vertically)
            let is_horizontal = y0 == y1 && (x1 as i32 - x0 as i32).abs() == 1;
            let is_vertical = x0 == x1 && (y1 as i32 - y0 as i32).abs() == 1;

            assert!(
                is_horizontal || is_vertical,
                "Invalid wireframe edge: ({},{}) to ({},{}) - vertices {} and {}",
                x0,
                y0,
                x1,
                y1,
                i0,
                i1
            );
        }
    }

    #[test]
    fn test_wireframe_indices_no_duplicates() {
        // Verify no duplicate edges in the wireframe
        let x_count = 5;
        let y_count = 4;
        let indices = generate_wireframe_indices(x_count, y_count);

        let mut edges: Vec<(u32, u32)> = indices
            .chunks_exact(2)
            .map(|chunk| {
                let a = chunk[0];
                let b = chunk[1];
                if a < b { (a, b) } else { (b, a) }
            })
            .collect();

        let original_len = edges.len();
        edges.sort();
        edges.dedup();

        assert_eq!(
            edges.len(),
            original_len,
            "Wireframe should not have duplicate edges"
        );
    }

    #[test]
    fn test_empty_mesh() {
        let data = SurfaceData::from_grid(vec![0.0], vec![0.0], vec![vec![0.0]]);
        let mesh = SurfaceMesh::from_data(&data, SurfacePlotType::Cartesian);

        assert!(mesh.is_empty());
    }

    #[test]
    fn test_gpu_vertex_size() {
        // Ensure GpuVertex is properly aligned for GPU
        assert_eq!(std::mem::size_of::<GpuVertex>(), 32);
    }

    /// Test demonstrating what the bug was: inferring grid dimensions from vertex count
    /// For a non-square grid, sqrt-based inference gives wrong results
    #[test]
    fn test_wireframe_dimension_inference_bug() {
        // Create a non-square mesh: 100 x 50 = 5000 vertices
        let x_count = 100;
        let y_count = 50;
        let vertex_count = x_count * y_count;

        // This is what the renderer used to do (BUG!)
        let inferred_x = ((vertex_count as f64).sqrt() as usize).max(2);
        let inferred_y = vertex_count / inferred_x;

        // sqrt(5000) ≈ 70.7 -> 70
        // 5000 / 70 = 71 (with remainder!)
        assert_ne!(
            inferred_x, x_count,
            "Sqrt inference should be wrong for non-square"
        );
        assert_ne!(
            inferred_y, y_count,
            "Sqrt inference should be wrong for non-square"
        );

        // The inferred values don't even multiply back to the original count!
        assert_ne!(
            inferred_x * inferred_y,
            vertex_count,
            "Inferred dimensions don't match vertex count"
        );

        // Correct wireframe indices for actual dimensions
        let correct_indices = generate_wireframe_indices(x_count, y_count);

        // Incorrect wireframe indices using inferred dimensions
        let incorrect_indices = generate_wireframe_indices(inferred_x, inferred_y);

        // The incorrect indices will have wrong count
        let correct_count = 2 * (y_count * (x_count - 1) + (y_count - 1) * x_count);
        assert_eq!(correct_indices.len(), correct_count);

        // And the incorrect indices will reference wrong vertex positions
        // This would cause rendering artifacts
        let incorrect_count = 2 * (inferred_y * (inferred_x - 1) + (inferred_y - 1) * inferred_x);
        assert_eq!(incorrect_indices.len(), incorrect_count);

        // Show the magnitude of the error
        let diff = (correct_count as i64 - incorrect_count as i64).abs();
        println!(
            "Correct indices: {}, Incorrect indices: {}, Difference: {}",
            correct_count, incorrect_count, diff
        );
    }

    /// Verify that SurfaceMesh correctly stores grid dimensions for wireframe rendering
    #[test]
    fn test_mesh_stores_grid_dimensions() {
        // Test with a non-square grid
        let data = SurfaceData::from_function((-1.0, 1.0), (-1.0, 1.0), 20, 10, |x, y| x + y);
        let mesh = SurfaceMesh::from_data(&data, SurfacePlotType::Cartesian);

        // Verify mesh stores correct dimensions
        assert_eq!(mesh.x_count, 20, "Mesh should store x_count");
        assert_eq!(mesh.y_count, 10, "Mesh should store y_count");
        assert_eq!(mesh.vertex_count, 200, "Vertex count should be x*y");

        // Verify wireframe can be generated with correct dimensions
        let wireframe_indices = generate_wireframe_indices(mesh.x_count, mesh.y_count);

        // Expected: 10 rows * 19 h-segments + 9 rows * 20 v-segments = 190 + 180 = 370 segments = 740 indices
        let expected = 2 * (mesh.y_count * (mesh.x_count - 1) + (mesh.y_count - 1) * mesh.x_count);
        assert_eq!(wireframe_indices.len(), expected);

        // Verify all indices are valid
        for &idx in &wireframe_indices {
            assert!(
                (idx as usize) < mesh.vertex_count,
                "Wireframe index {} out of bounds",
                idx
            );
        }
    }

    /// Test wireframe with a very wide grid (like audio frequency data)
    #[test]
    fn test_mesh_dimensions_audio_typical() {
        // Typical audio data: many frequency points (x), fewer angle points (y)
        // e.g., 100 frequency bins x 37 angles (-180 to 180 in 10° steps)
        let data =
            SurfaceData::from_function((20.0, 20000.0), (-180.0, 180.0), 100, 37, |_, _| 0.0);
        let mesh = SurfaceMesh::from_data(&data, SurfacePlotType::Cartesian);

        assert_eq!(mesh.x_count, 100);
        assert_eq!(mesh.y_count, 37);
        assert_eq!(mesh.vertex_count, 3700);

        // Generate wireframe - should not panic and have correct count
        let wireframe_indices = generate_wireframe_indices(mesh.x_count, mesh.y_count);

        // Verify count
        let expected = 2 * (37 * 99 + 36 * 100); // 3663 + 3600 = 7263 segments = 14526 indices
        assert_eq!(wireframe_indices.len(), expected);
    }
}
