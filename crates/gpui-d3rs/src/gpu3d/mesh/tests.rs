use super::super::config::SurfacePlotType;
use super::super::data::SurfaceData;
use super::GpuVertex;
use super::generate::generate_wireframe_indices;
use super::surface_mesh::SurfaceMesh;

use super::*;
use crate::gpu3d::config::SurfacePlotType;

#[test]
fn test_mesh_generation() {
    let data = SurfaceData::from_function((-1.0, 1.0), (-1.0, 1.0), 10, 10, |x, y| x * x + y * y);
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
    let data = SurfaceData::from_function((20.0, 20000.0), (-180.0, 180.0), 100, 37, |_, _| 0.0);
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
