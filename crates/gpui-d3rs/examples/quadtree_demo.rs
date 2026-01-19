//! QuadTree demonstration
//!
//! This example demonstrates the d3rs quadtree implementation, showing:
//! - Building a quadtree from data points
//! - Nearest neighbor search
//! - Range queries
//! - Tree traversal

use d3rs::quadtree::{QuadNode, QuadTree};

fn main() {
    println!("=== d3rs QuadTree Demo ===\n");

    // Create a quadtree and add points with their names
    println!("1. Creating QuadTree from 10 points:");
    let mut tree = QuadTree::new();
    tree.add(0.0, 0.0, "origin");
    tree.add(1.0, 0.0, "right");
    tree.add(0.0, 1.0, "up");
    tree.add(1.0, 1.0, "corner");
    tree.add(0.5, 0.5, "center");
    tree.add(0.25, 0.25, "quarter");
    tree.add(0.75, 0.75, "three-quarter");
    tree.add(2.0, 2.0, "far");
    tree.add(3.5, 1.5, "distant");
    tree.add(-0.5, 0.5, "negative");

    println!("   Tree size: {}", tree.size());
    if let Some(ext) = tree.extent() {
        println!(
            "   Extent: ({:.2}, {:.2}) to ({:.2}, {:.2})",
            ext.x0, ext.y0, ext.x1, ext.y1
        );
    }

    // Nearest neighbor search
    println!("\n2. Nearest Neighbor Search:");

    let queries = [(0.3, 0.3), (1.5, 1.5), (0.0, 0.0), (10.0, 10.0)];

    for (qx, qy) in queries {
        if let Some(nearest) = tree.find(qx, qy, None) {
            println!("   Nearest to ({:.1}, {:.1}): '{}'", qx, qy, nearest);
        }
    }

    // Radius-limited search
    println!("\n3. Radius-Limited Search:");

    let (qx, qy) = (0.5, 0.5);
    for radius in [0.3, 0.5, 1.0, 2.0] {
        if let Some(nearest) = tree.find(qx, qy, Some(radius)) {
            println!(
                "   Nearest to ({:.1}, {:.1}) within {:.1}: '{}'",
                qx, qy, radius, nearest
            );
        } else {
            println!(
                "   Nearest to ({:.1}, {:.1}) within {:.1}: none found",
                qx, qy, radius
            );
        }
    }

    // Find all within radius
    println!("\n4. Find All Within Radius:");

    let (cx, cy, radius) = (0.5, 0.5, 0.6);
    let within = tree.find_all(cx, cy, radius);
    println!(
        "   Points within {:.1} of ({:.1}, {:.1}): {} found",
        radius,
        cx,
        cy,
        within.len()
    );
    for point in within {
        println!("   - '{}'", point);
    }

    // Tree traversal
    println!("\n5. Tree Traversal (pre-order visit):");
    let mut node_count = 0;
    let mut leaf_count = 0;
    let mut internal_count = 0;

    tree.visit(|x0, y0, x1, y1, node| {
        node_count += 1;
        match node {
            QuadNode::Leaf(point) => {
                leaf_count += 1;
                println!(
                    "   Leaf at ({:.2}, {:.2})-({:.2}, {:.2}): point ({:.2}, {:.2})",
                    x0, y0, x1, y1, point.x, point.y
                );
            }
            QuadNode::Internal(_) => {
                internal_count += 1;
                println!(
                    "   Internal node: ({:.2}, {:.2})-({:.2}, {:.2})",
                    x0, y0, x1, y1
                );
            }
        }
        true // Continue visiting children
    });

    println!(
        "\n   Total nodes: {}, Leaves: {}, Internal: {}",
        node_count, leaf_count, internal_count
    );

    // Data extraction
    println!("\n6. Extracting All Data:");
    let all_data = tree.data();
    println!("   Retrieved {} points:", all_data.len());
    for (x, y, name) in &all_data {
        println!("   ({:.2}, {:.2}): {}", x, y, name);
    }

    // Demonstrate adding and removing
    println!("\n7. Dynamic Updates:");
    let mut mutable_tree: QuadTree<&str> = QuadTree::new();

    mutable_tree.add(0.0, 0.0, "A");
    mutable_tree.add(1.0, 1.0, "B");
    mutable_tree.add(2.0, 2.0, "C");
    println!("   After adding A, B, C: size = {}", mutable_tree.size());

    mutable_tree.remove(1.0, 1.0);
    println!("   After removing B: size = {}", mutable_tree.size());

    // Verify B is removed
    let found = mutable_tree.find(1.0, 1.0, Some(0.01));
    println!(
        "   Can find B at (1,1) with tiny radius? {}",
        found.is_some()
    );

    // Coincident points
    println!("\n8. Handling Coincident Points:");
    let mut coin_tree: QuadTree<&str> = QuadTree::new();
    coin_tree.add(5.0, 5.0, "first");
    coin_tree.add(5.0, 5.0, "second");
    coin_tree.add(5.0, 5.0, "third");
    println!(
        "   Added 3 points at same location, size = {}",
        coin_tree.size()
    );

    // Performance demo
    println!("\n9. Performance with Large Dataset:");
    let mut large_tree: QuadTree<i32> = QuadTree::new();
    for i in 0..10000 {
        // Use golden ratio for nice distribution
        let x = (i as f64 * 0.618033988749895).fract() * 100.0;
        let y = (i as f64 * 0.381966011250105).fract() * 100.0;
        large_tree.add(x, y, i);
    }

    let start = std::time::Instant::now();
    let build_time = start.elapsed();

    println!(
        "   Built tree with {} points in {:?}",
        large_tree.size(),
        build_time
    );

    // Nearest neighbor queries
    let start = std::time::Instant::now();
    let mut found_count = 0;
    for i in 0..1000 {
        let x = (i as f64 * 0.123).fract() * 100.0;
        let y = (i as f64 * 0.456).fract() * 100.0;
        if large_tree.find(x, y, None).is_some() {
            found_count += 1;
        }
    }
    let query_time = start.elapsed();
    println!(
        "   Performed 1000 nearest-neighbor queries in {:?} ({} found)",
        query_time, found_count
    );

    // Range queries
    let start = std::time::Instant::now();
    let mut total_found = 0;
    for i in 0..100 {
        let x = (i as f64 * 0.789).fract() * 100.0;
        let y = (i as f64 * 0.321).fract() * 100.0;
        total_found += large_tree.find_all(x, y, 5.0).len();
    }
    let range_time = start.elapsed();
    println!(
        "   Performed 100 range queries (r=5.0) in {:?} ({} total points found)",
        range_time, total_found
    );

    println!("\n=== Demo Complete ===");
}
