//! Surface plot demonstration example
//!
//! This example demonstrates the 3D surface plotting capabilities.
//! Note: This is a non-GUI demo that shows how to create surface data
//! and mesh structures. For visual output, use the showcase binary.

use d3rs::surface::{Camera2D, IsometricProjection, Projection, SurfaceData, SurfaceMesh};

fn main() {
    println!("=== d3rs Surface Plot Demo ===\n");

    // Create surface data from a mathematical function
    println!("Creating surface from z = sin(sqrt(x^2 + y^2))...");
    let data = SurfaceData::from_z_function((-3.0, 3.0), (-3.0, 3.0), 20, |x, y| {
        let r = (x * x + y * y).sqrt();
        r.sin()
    });

    println!("Surface data created:");
    println!("  Grid size: {}x{}", data.rows(), data.cols());
    println!("  X range: {:?}", data.x_range);
    println!("  Y range: {:?}", data.y_range);
    println!("  Z range: ({:.3}, {:.3})", data.z_range.0, data.z_range.1);
    println!("  T range: ({:.3}, {:.3})", data.t_range.0, data.t_range.1);

    // Create a mesh from the surface data
    println!("\nTriangulating surface...");
    let mesh = SurfaceMesh::from_surface_data(&data);
    println!("  Triangles created: {}", mesh.len());

    // Demonstrate normalized data
    println!("\nNormalizing to unit cube...");
    let normalized = data.normalize();
    println!("  Normalized X range: {:?}", normalized.x_range);
    println!("  Normalized Y range: {:?}", normalized.y_range);
    println!("  Normalized Z range: {:?}", normalized.z_range);

    // Demonstrate projection
    println!("\n=== Projection Demo ===\n");

    let projection = IsometricProjection::new()
        .scale(100.0)
        .origin(300.0, 200.0)
        .camera(Camera2D::new().rotation(30.0, 45.0).zoom(1.0));

    // Sample point projection
    let test_points = [
        (0.0, 0.0, 0.0),
        (1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0),
        (0.0, 0.0, 1.0),
        (1.0, 1.0, 1.0),
    ];

    println!("Isometric projection (scale=100, rotation=(30, 45)):");
    for (x, y, z) in test_points {
        let p = projection.project(x, y, z);
        let depth = projection.depth(x, y, z);
        println!(
            "  ({:.1}, {:.1}, {:.1}) -> screen({:.1}, {:.1}), depth={:.2}",
            x, y, z, p.x, p.y, depth
        );
    }

    // Different surface functions
    println!("\n=== Surface Function Examples ===\n");

    // Saddle surface
    println!("Saddle surface: z = x^2 - y^2");
    let saddle = SurfaceData::from_z_function((-2.0, 2.0), (-2.0, 2.0), 10, |x, y| x * x - y * y);
    println!(
        "  Z range: ({:.3}, {:.3})",
        saddle.z_range.0, saddle.z_range.1
    );

    // Gaussian
    println!("\nGaussian: z = exp(-(x^2 + y^2))");
    let gaussian = SurfaceData::from_z_function((-2.0, 2.0), (-2.0, 2.0), 10, |x, y| {
        (-(x * x + y * y)).exp()
    });
    println!(
        "  Z range: ({:.3}, {:.3})",
        gaussian.z_range.0, gaussian.z_range.1
    );

    // Plane
    println!("\nPlane: z = 0.5*x + 0.3*y");
    let plane =
        SurfaceData::from_z_function((-2.0, 2.0), (-2.0, 2.0), 10, |x, y| 0.5 * x + 0.3 * y);
    println!(
        "  Z range: ({:.3}, {:.3})",
        plane.z_range.0, plane.z_range.1
    );

    // Surface with separate color mapping value
    println!("\n=== Surface with Custom Color Mapping ===\n");

    println!("Surface: z = sin(x)*cos(y), color by x+y");
    let custom = SurfaceData::from_function((-3.14, 3.14), (-3.14, 3.14), 15, |x, y| {
        let z = x.sin() * y.cos();
        let t = x + y; // Color by position, not height
        (z, t)
    });
    println!(
        "  Z range: ({:.3}, {:.3})",
        custom.z_range.0, custom.z_range.1
    );
    println!(
        "  T range (for color): ({:.3}, {:.3})",
        custom.t_range.0, custom.t_range.1
    );

    // Grid access demonstration
    println!("\n=== Grid Access ===\n");

    let sample = SurfaceData::from_z_function((0.0, 1.0), (0.0, 1.0), 5, |x, y| x * y);
    println!("5x5 grid of z = x * y:");
    for row in 0..sample.rows() {
        print!("  ");
        for col in 0..sample.cols() {
            if let Some(p) = sample.get(row, col) {
                print!("{:.2} ", p.z);
            }
        }
        println!();
    }

    println!("\n=== Demo Complete ===");
}
