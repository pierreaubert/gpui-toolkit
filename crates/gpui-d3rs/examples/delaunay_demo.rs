//! Delaunay triangulation and Voronoi diagram demo
//!
//! Run with: cargo run --example delaunay_demo

use d3rs::delaunay::Delaunay;

fn main() {
    println!("=== d3-delaunay Demo ===\n");

    // Create some sample points
    let points = vec![
        (0.0, 0.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (0.0, 1.0),
        (0.5, 0.5),
        (0.25, 0.75),
        (0.75, 0.25),
    ];

    println!("Input points:");
    for (i, &(x, y)) in points.iter().enumerate() {
        println!("  Point {}: ({:.2}, {:.2})", i, x, y);
    }

    // Create Delaunay triangulation
    let delaunay = Delaunay::new(&points);
    println!("\n--- Delaunay Triangulation ---");
    println!("Number of points: {}", delaunay.len());
    println!("Number of triangles: {}", delaunay.triangle_count());

    // Print triangles
    println!("\nTriangles (vertex indices):");
    for (i, (a, b, c)) in delaunay.triangles().enumerate() {
        let pa = delaunay.point(a).unwrap();
        let pb = delaunay.point(b).unwrap();
        let pc = delaunay.point(c).unwrap();
        println!(
            "  Triangle {}: [{}, {}, {}] = [({:.2},{:.2}), ({:.2},{:.2}), ({:.2},{:.2})]",
            i, a, b, c, pa.0, pa.1, pb.0, pb.1, pc.0, pc.1
        );
    }

    // Print edges
    println!("\nEdges:");
    for (a, b) in delaunay.edges() {
        let pa = delaunay.point(a).unwrap();
        let pb = delaunay.point(b).unwrap();
        println!(
            "  {} -- {}: ({:.2},{:.2}) -- ({:.2},{:.2})",
            a, b, pa.0, pa.1, pb.0, pb.1
        );
    }

    // Convex hull
    println!("\nConvex Hull:");
    println!("  Indices: {:?}", delaunay.hull());
    let hull_polygon = delaunay.hull_polygon();
    println!("  Polygon: {:?}", hull_polygon);

    // Find nearest neighbor
    println!("\n--- Nearest Neighbor Queries ---");
    let queries = vec![(0.3, 0.3), (0.9, 0.1), (0.5, 0.5), (0.1, 0.9)];
    for (x, y) in queries {
        if let Some(nearest) = delaunay.find(x, y, None) {
            let (px, py) = delaunay.point(nearest).unwrap();
            println!(
                "  Nearest to ({:.1}, {:.1}): point {} at ({:.2}, {:.2})",
                x, y, nearest, px, py
            );
        }
    }

    // Find with radius
    println!("\n--- Radius Search ---");
    let nearest = delaunay.find_within_radius(0.5, 0.5, 0.1);
    println!("  Find within 0.1 of (0.5, 0.5): {:?}", nearest);
    let nearest = delaunay.find_within_radius(0.5, 0.5, 0.01);
    println!("  Find within 0.01 of (0.5, 0.5): {:?}", nearest);

    // Neighbors
    println!("\n--- Point Neighbors ---");
    let center_point = 4; // (0.5, 0.5)
    let neighbors: Vec<_> = delaunay.neighbors(center_point).collect();
    println!("  Point {} neighbors: {:?}", center_point, neighbors);

    // SVG path rendering
    println!("\n--- SVG Path Data ---");
    let triangulation_path = delaunay.render_to_path();
    println!(
        "  Triangulation path (first 100 chars): {}...",
        &triangulation_path[..triangulation_path.len().min(100)]
    );
    let hull_path = delaunay.render_hull_to_path();
    println!("  Hull path: {}", hull_path);

    // Voronoi diagram
    println!("\n--- Voronoi Diagram ---");
    let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));
    println!("Bounds: {:?}", voronoi.bounds());
    println!("Number of cells: {}", voronoi.cell_count());

    println!("\nVoronoi cells:");
    for (i, cell) in voronoi.cell_polygons().enumerate() {
        if !cell.is_empty() {
            println!("  Cell {} (point {:?}):", i, delaunay.point(i));
            println!("    Vertices: {} points", cell.len());
            if cell.len() <= 6 {
                for (j, &(x, y)) in cell.iter().enumerate() {
                    println!("      {}: ({:.3}, {:.3})", j, x, y);
                }
            }
        }
    }

    // Voronoi SVG path
    println!("\n--- Voronoi SVG Path Data ---");
    let voronoi_path = voronoi.render_to_path();
    if !voronoi_path.is_empty() {
        println!(
            "  Voronoi path (first 100 chars): {}...",
            &voronoi_path[..voronoi_path.len().min(100)]
        );
    }

    // Check point containment
    println!("\n--- Point Containment ---");
    let test_point = (0.3, 0.3);
    for i in 0..voronoi.cell_count() {
        if voronoi.contains(i, test_point.0, test_point.1) {
            println!(
                "  Point ({:.1}, {:.1}) is in cell {} (seed at {:?})",
                test_point.0,
                test_point.1,
                i,
                delaunay.point(i)
            );
        }
    }

    // Voronoi neighbors
    println!("\n--- Voronoi Cell Neighbors ---");
    let voronoi_neighbors: Vec<_> = voronoi.neighbors(center_point).collect();
    println!(
        "  Cell {} (at {:?}) neighbors: {:?}",
        center_point,
        delaunay.point(center_point),
        voronoi_neighbors
    );

    // Random points demo
    println!("\n--- Large Dataset Demo ---");
    let n = 100;
    let large_points: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
            let r = 0.3 + 0.2 * ((i * 7) % 10) as f64 / 10.0;
            (0.5 + r * angle.cos(), 0.5 + r * angle.sin())
        })
        .collect();

    let large_delaunay = Delaunay::new(&large_points);
    println!(
        "  Created triangulation with {} points",
        large_delaunay.len()
    );
    println!("  Number of triangles: {}", large_delaunay.triangle_count());
    println!("  Hull size: {}", large_delaunay.hull().len());

    let large_voronoi = large_delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));
    println!("  Voronoi cells: {}", large_voronoi.cell_count());

    println!("\n=== Demo Complete ===");
}
