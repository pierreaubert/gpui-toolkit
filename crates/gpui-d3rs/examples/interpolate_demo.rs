//! d3-interpolate demonstration
//!
//! This example demonstrates the interpolation utilities inspired by d3-interpolate:
//! - Numeric interpolation (linear, round, exp, clamped)
//! - Color interpolation (RGB, HSL, LAB)
//! - Transform interpolation (2D affine transforms)
//! - Zoom interpolation (smooth pan/zoom)
//! - String interpolation (with embedded numbers)
//! - Piecewise interpolation

use d3rs::color::D3Color;
use d3rs::interpolate::{
    // Transform
    Transform2D,
    cubehelix_default,
    // Numeric
    interpolate,
    interpolate_basis,
    interpolate_clamped,
    interpolate_exp,
    interpolate_hsl,
    interpolate_lab,
    // Color
    interpolate_rgb,
    interpolate_round,
    // String
    interpolate_string,
    interpolate_transform,
    // Piecewise
    piecewise,
    quantize,
    // Zoom
    zoom::{ZoomView, interpolate_zoom, zoom_duration},
};

fn main() {
    println!("=== d3-interpolate Demonstration ===\n");

    // ========================================
    // Numeric Interpolation
    // ========================================
    println!("--- Numeric Interpolation ---\n");

    // Basic linear interpolation
    let lerp = interpolate(0.0, 100.0);
    println!("Linear interpolation (0 to 100):");
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        println!("  t={:.2}: {:.1}", t, lerp(t));
    }

    // Rounded interpolation
    let round_lerp = interpolate_round(0, 10);
    println!("\nRounded interpolation (0 to 10):");
    for t in [0.0, 0.33, 0.5, 0.67, 1.0] {
        println!("  t={:.2}: {}", t, round_lerp(t));
    }

    // Clamped interpolation
    let clamped = interpolate_clamped(0.0, 100.0);
    println!("\nClamped interpolation (stays in [0,100]):");
    for t in [-0.5, 0.0, 0.5, 1.0, 1.5] {
        println!("  t={:.2}: {:.1}", t, clamped(t));
    }

    // Exponential interpolation (natural logarithm base)
    let exp_lerp = interpolate_exp(1.0, 1000.0);
    println!("\nExponential interpolation (1 to 1000):");
    for t in [0.0, 0.33, 0.67, 1.0] {
        println!("  t={:.2}: {:.1}", t, exp_lerp(t));
    }

    // Basis spline interpolation
    let values = vec![0.0, 10.0, 50.0, 30.0, 100.0];
    let basis = interpolate_basis(&values);
    println!("\nBasis spline through {:?}:", values);
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        println!("  t={:.2}: {:.1}", t, basis(t));
    }

    // ========================================
    // Color Interpolation
    // ========================================
    println!("\n--- Color Interpolation ---\n");

    let red = D3Color::rgb(255, 0, 0);
    let blue = D3Color::rgb(0, 0, 255);

    println!("Interpolating from {} to {}", red.to_hex(), blue.to_hex());

    // RGB interpolation
    let rgb_interp = interpolate_rgb(red, blue);
    println!("\nRGB interpolation:");
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let color = rgb_interp(t);
        println!("  t={:.2}: {}", t, color.to_hex());
    }

    // HSL interpolation (goes through colors)
    let hsl_interp = interpolate_hsl(red, blue);
    println!("\nHSL interpolation (short arc):");
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let color = hsl_interp(t);
        println!("  t={:.2}: {}", t, color.to_hex());
    }

    // LAB interpolation (perceptually uniform)
    let lab_interp = interpolate_lab(red, blue);
    println!("\nLAB interpolation (perceptually uniform):");
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let color = lab_interp(t);
        println!("  t={:.2}: {}", t, color.to_hex());
    }

    // Cubehelix (rainbow-like)
    let cubehelix = cubehelix_default();
    println!("\nCubehelix rainbow:");
    for t in [0.0, 0.2, 0.4, 0.6, 0.8, 1.0] {
        let color = cubehelix(t);
        println!("  t={:.2}: {}", t, color.to_hex());
    }

    // ========================================
    // Transform Interpolation
    // ========================================
    println!("\n--- Transform Interpolation ---\n");

    // Create transforms
    let start = Transform2D::identity();
    let end = Transform2D::translate(100.0, 50.0)
        .interpolate(&Transform2D::rotate_deg(45.0), 1.0)
        .interpolate(&Transform2D::scale(2.0, 2.0), 1.0);

    println!("Start: identity");
    println!("End: translate(100,50) + rotate(45) + scale(2,2)");

    let transform_interp = interpolate_transform(start, end);
    println!("\nTransform interpolation:");
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let tr = transform_interp(t);
        println!(
            "  t={:.2}: translate({:.1}, {:.1}), rotate={:.1} deg, scale=({:.2}, {:.2})",
            t,
            tr.translate_x,
            tr.translate_y,
            tr.rotate.to_degrees(),
            tr.scale_x,
            tr.scale_y
        );
    }

    // Point transformation
    println!("\nTransforming point (10, 10):");
    for t in [0.0, 0.5, 1.0] {
        let tr = transform_interp(t);
        let (x, y) = tr.apply(10.0, 10.0);
        println!("  t={:.1}: ({:.1}, {:.1})", t, x, y);
    }

    // CSS output
    println!("\nCSS transform strings:");
    for t in [0.0, 0.5, 1.0] {
        let tr = transform_interp(t);
        println!("  t={:.1}: {}", t, tr.to_css());
    }

    // ========================================
    // Zoom Interpolation
    // ========================================
    println!("\n--- Zoom Interpolation ---\n");

    // Zoom from wide view to zoomed-in view
    let view_start = ZoomView::new(0.0, 0.0, 1000.0); // Wide view at origin
    let view_end = ZoomView::new(500.0, 300.0, 50.0); // Zoomed in at (500, 300)

    println!(
        "Zoom from: center=({}, {}), size={}",
        view_start.cx, view_start.cy, view_start.size
    );
    println!(
        "Zoom to:   center=({}, {}), size={}",
        view_end.cx, view_end.cy, view_end.size
    );

    let zoom_interp = interpolate_zoom(view_start, view_end);
    println!("\nSmooth zoom trajectory (van Wijk & Nuij algorithm):");
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let view = zoom_interp(t);
        println!(
            "  t={:.2}: center=({:6.1}, {:5.1}), size={:6.1}",
            t, view.cx, view.cy, view.size
        );
    }

    let duration = zoom_duration(view_start, view_end);
    println!("\nZoom duration (normalized): {:.2}", duration);

    // ========================================
    // String Interpolation
    // ========================================
    println!("\n--- String Interpolation ---\n");

    let string_interp = interpolate_string(
        "translate(0px, 0px) scale(1)",
        "translate(100px, 50px) scale(2)",
    );

    println!("String interpolation:");
    for t in [0.0, 0.5, 1.0] {
        println!("  t={:.1}: {}", t, string_interp(t));
    }

    // ========================================
    // Piecewise Interpolation
    // ========================================
    println!("\n--- Piecewise Interpolation ---\n");

    // Piecewise numeric interpolation
    let values = vec![0.0, 10.0, 30.0, 60.0, 100.0];
    let pw = piecewise(&values);
    println!("Piecewise numeric (0, 10, 30, 60, 100):");
    for i in 0..=10 {
        let t = i as f64 / 10.0;
        println!("  t={:.1}: {:.1}", t, pw(t));
    }

    // Quantize creates a step function
    let step_values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let quantized = quantize(&step_values);
    println!("\nQuantize ({:?}):", step_values);
    for i in 0..=10 {
        let t = i as f64 / 10.0;
        println!("  t={:.1}: {:.0}", t, quantized(t));
    }

    println!("\n=== End of d3-interpolate Demo ===");
}
