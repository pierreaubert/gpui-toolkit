//! d3-contour demonstration
//!
//! This example demonstrates the contour utilities inspired by d3-contour:
//! - Marching squares algorithm for contour generation
//! - Density estimation for point clouds
//! - Threshold calculation functions

use d3rs::contour::{
    // Marching squares
    ContourGenerator,
    // Density estimation
    DensityEstimator,
    contours,
    density_2d,
    gaussian_kernel,
    threshold_freedman_diaconis,
    threshold_scott,
    // Thresholds
    threshold_sturges,
};

fn main() {
    println!("=== d3-contour Demonstration ===\n");

    // ========================================
    // Marching Squares - Basic Contours
    // ========================================
    println!("--- Marching Squares Algorithm ---\n");

    // Create a simple 5x5 grid with a peak in the center
    let grid: Vec<f64> = vec![
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.3, 0.5, 0.3, 0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 0.0, 0.3, 0.5,
        0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];

    println!("5x5 grid with center peak:");
    for row in 0..5 {
        print!("  ");
        for col in 0..5 {
            print!("{:.1} ", grid[row * 5 + col]);
        }
        println!();
    }

    // Generate contours at different thresholds
    let thresholds = vec![0.25, 0.5, 0.75];
    let generator = ContourGenerator::new(5, 5);

    println!("\nContours at thresholds {:?}:", thresholds);
    for threshold in &thresholds {
        let c = generator.contour(&grid, *threshold);
        println!(
            "  Threshold {:.2}: {} ring(s)",
            threshold,
            c.coordinates.len()
        );
        for (i, ring) in c.coordinates.iter().enumerate() {
            println!(
                "    Ring {}: {} points, closed={}, area={:.2}",
                i,
                ring.points.len(),
                ring.is_closed(),
                ring.area().abs()
            );
        }
    }

    // ========================================
    // Marching Squares - Larger Grid
    // ========================================
    println!("\n--- Larger Grid Example ---\n");

    // Generate a 20x20 grid with a Gaussian peak
    let size = 20;
    let mut large_grid = vec![0.0; size * size];

    for y in 0..size {
        for x in 0..size {
            let dx = (x as f64 - 10.0) / 5.0;
            let dy = (y as f64 - 10.0) / 5.0;
            large_grid[y * size + x] = (-0.5 * (dx * dx + dy * dy)).exp();
        }
    }

    println!("{}x{} Gaussian peak grid:", size, size);
    println!(
        "  Min: {:.3}",
        large_grid.iter().cloned().fold(f64::INFINITY, f64::min)
    );
    println!(
        "  Max: {:.3}",
        large_grid.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    );

    // Generate multiple contour levels
    let levels = vec![0.1, 0.3, 0.5, 0.7, 0.9];
    let multi_contours = contours(&large_grid, size, size, &levels);

    println!("\nContour levels:");
    for c in &multi_contours {
        let total_points: usize = c.coordinates.iter().map(|r| r.points.len()).sum();
        println!(
            "  Level {:.1}: {} ring(s), {} total points",
            c.value,
            c.coordinates.len(),
            total_points
        );
    }

    // ========================================
    // Density Estimation
    // ========================================
    println!("\n--- Density Estimation ---\n");

    // Generate random point cloud with two clusters
    let points: Vec<(f64, f64)> = {
        let mut pts = Vec::new();
        // Cluster 1 around (0.3, 0.3)
        for i in 0..50 {
            let angle = (i as f64) * 0.1;
            let r = 0.1 * (i as f64 % 5.0) / 5.0;
            pts.push((0.3 + r * angle.cos(), 0.3 + r * angle.sin()));
        }
        // Cluster 2 around (0.7, 0.7)
        for i in 0..30 {
            let angle = (i as f64) * 0.15;
            let r = 0.08 * (i as f64 % 4.0) / 4.0;
            pts.push((0.7 + r * angle.cos(), 0.7 + r * angle.sin()));
        }
        pts
    };

    println!("Point cloud: {} points in two clusters", points.len());
    println!("  Cluster 1: ~50 points around (0.3, 0.3)");
    println!("  Cluster 2: ~30 points around (0.7, 0.7)");

    // Create density estimator
    let estimator = DensityEstimator::new()
        .size(20, 20)
        .x(0.0, 1.0)
        .y(0.0, 1.0)
        .bandwidth(0.1);

    let density_grid = estimator.estimate(&points);

    let max_density = density_grid.iter().cloned().fold(0.0_f64, f64::max);
    let mean_density: f64 = density_grid.iter().sum::<f64>() / density_grid.len() as f64;

    println!("\nDensity estimation (20x20, bandwidth=0.1):");
    println!("  Max density: {:.4}", max_density);
    println!("  Mean density: {:.6}", mean_density);

    // Find peak locations
    let mut max_idx = 0;
    let mut max_val = 0.0;
    for (i, &val) in density_grid.iter().enumerate() {
        if val > max_val {
            max_val = val;
            max_idx = i;
        }
    }
    let peak_x = (max_idx % 20) as f64 / 20.0;
    let peak_y = (max_idx / 20) as f64 / 20.0;
    println!("  Peak location: ({:.2}, {:.2})", peak_x, peak_y);

    // Generate density contours
    println!("\nDensity contours:");
    let density_levels = vec![0.1 * max_density, 0.3 * max_density, 0.5 * max_density];
    let density_contours = contours(&density_grid, 20, 20, &density_levels);

    for c in &density_contours {
        println!("  Level {:.4}: {} ring(s)", c.value, c.coordinates.len());
    }

    // ========================================
    // Simple Density Function
    // ========================================
    println!("\n--- Simple Density API ---\n");

    let simple_points = vec![(0.5, 0.5), (0.6, 0.4), (0.4, 0.6), (0.55, 0.55)];

    let (grid, width, height) = density_2d(&simple_points, 10, 10, 0.15);
    println!("Simple density_2d() for {} points:", simple_points.len());
    println!("  Grid size: {}x{}", width, height);
    println!(
        "  Max value: {:.4}",
        grid.iter().cloned().fold(0.0_f64, f64::max)
    );

    // ========================================
    // Gaussian Kernel
    // ========================================
    println!("\n--- Gaussian Kernel ---\n");

    println!("Gaussian kernel values (bandwidth=1.0):");
    for x in [-2.0, -1.0, 0.0, 1.0, 2.0] {
        let k = gaussian_kernel(x, 1.0);
        let bar: String = std::iter::repeat('#').take((k * 50.0) as usize).collect();
        println!("  x={:5.1}: {:.4} {}", x, k, bar);
    }

    // ========================================
    // Threshold Functions
    // ========================================
    println!("\n--- Threshold Calculation ---\n");

    let values: Vec<f64> = (0..100).map(|i| i as f64).collect();
    let min = 0.0;
    let max = 100.0;

    // Sturges' formula
    let sturges = threshold_sturges(min, max, values.len());
    println!("Sturges' formula ({} values):", values.len());
    println!(
        "  {} thresholds: {:?}",
        sturges.len(),
        sturges.iter().map(|v| v.round()).collect::<Vec<_>>()
    );

    // Scott's rule
    let scott = threshold_scott(&values, min, max);
    println!("\nScott's rule:");
    println!("  {} thresholds", scott.len());

    // Freedman-Diaconis rule
    let fd = threshold_freedman_diaconis(&values, min, max);
    println!("\nFreedman-Diaconis rule:");
    println!("  {} thresholds", fd.len());

    // ========================================
    // ASCII Visualization
    // ========================================
    println!("\n--- ASCII Contour Visualization ---\n");

    // Create a smooth field
    let viz_size = 30;
    let mut viz_grid = vec![0.0; viz_size * viz_size];

    for y in 0..viz_size {
        for x in 0..viz_size {
            let fx = x as f64 / viz_size as f64;
            let fy = y as f64 / viz_size as f64;
            // Create a wave pattern
            let val = 0.5
                + 0.3 * (fx * 6.28).sin() * (fy * 6.28).cos()
                + 0.2 * ((fx - 0.5).powi(2) + (fy - 0.5).powi(2)).sqrt() * -2.0;
            viz_grid[y * viz_size + x] = val.max(0.0).min(1.0);
        }
    }

    // ASCII visualization with contour levels
    let chars = [' ', '.', ':', '-', '=', '+', '*', '#', '@'];
    println!("Contour visualization ({}x{}):", viz_size, viz_size);
    for y in 0..viz_size {
        print!("  ");
        for x in 0..viz_size {
            let val = viz_grid[y * viz_size + x];
            let idx = ((val * 8.0).round() as usize).min(8);
            print!("{}", chars[idx]);
        }
        println!();
    }

    println!("\n=== End of d3-contour Demo ===");
}
