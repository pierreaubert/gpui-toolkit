//! Surface plot with logarithmic axis demonstration
//!
//! This example demonstrates the logarithmic axis sampling capabilities
//! for surface plots, which is particularly useful for frequency domain
//! visualizations in audio applications.

use d3rs::surface::SurfaceData;

fn main() {
    println!("=== gpui-d3rs Logarithmic Surface Plot Demo ===\n");

    // Example 1: Logarithmic X-axis (common for frequency response plots)
    println!("Example 1: Frequency Response Plot (Log X-axis)");
    println!("------------------------------------------------");

    let freq_response = SurfaceData::from_z_function_logx(
        (20.0, 20000.0), // 20 Hz to 20 kHz (logarithmic)
        (0.0, 1.0),      // Linear Y axis
        10,
        |freq, _time| {
            // Simulated frequency response with rolloffs
            if freq < 100.0 {
                -12.0 * (100.0 - freq) / 80.0 // Low freq rolloff
            } else if freq > 10000.0 {
                -6.0 * (freq - 10000.0) / 10000.0 // High freq rolloff
            } else {
                0.0 // Flat response
            }
        },
    );

    println!("Surface created:");
    println!(
        "  Grid size: {}x{}",
        freq_response.rows(),
        freq_response.cols()
    );
    println!(
        "  X range (frequency): ({:.1}, {:.1}) Hz",
        freq_response.x_range.0, freq_response.x_range.1
    );
    println!("  Y range: {:?}", freq_response.y_range);
    println!(
        "  Z range (magnitude): ({:.2}, {:.2}) dB",
        freq_response.z_range.0, freq_response.z_range.1
    );

    // Show logarithmic spacing of X values
    println!("\n  Sample X values (logarithmically spaced):");
    for i in 0..5 {
        if let Some(p) = freq_response.get(0, i * 2) {
            println!("    Column {}: {:.1} Hz", i * 2, p.x);
        }
    }

    // Example 2: Logarithmic Y-axis
    println!("\n\nExample 2: Time-Frequency Analysis (Log Y-axis)");
    println!("------------------------------------------------");

    let time_freq = SurfaceData::from_z_function_logy(
        (0.0, 1.0),      // Time (linear)
        (20.0, 20000.0), // Frequency (logarithmic)
        10,
        |time, freq| {
            // Simulated time-varying frequency content
            ((time * freq / 100.0).sin() * (1.0 + time)).abs()
        },
    );

    println!("Surface created:");
    println!("  Grid size: {}x{}", time_freq.rows(), time_freq.cols());
    println!("  X range (time): {:?}", time_freq.x_range);
    println!(
        "  Y range (frequency): ({:.1}, {:.1}) Hz",
        time_freq.y_range.0, time_freq.y_range.1
    );

    // Example 3: Both axes logarithmic (2D frequency domain)
    println!("\n\nExample 3: 2D Frequency Domain (Both axes logarithmic)");
    println!("--------------------------------------------------------");

    let freq_2d = SurfaceData::from_z_function_logxy(
        (20.0, 20000.0), // X: Frequency 1 (log)
        (20.0, 20000.0), // Y: Frequency 2 (log)
        10,
        |fx, fy| {
            // Simulated 2D frequency response
            let product = (fx * fy).sqrt();
            if product < 1000.0 {
                -6.0
            } else if product > 5000.0 {
                -3.0
            } else {
                0.0
            }
        },
    );

    println!("Surface created:");
    println!("  Grid size: {}x{}", freq_2d.rows(), freq_2d.cols());
    println!(
        "  X range (freq 1): ({:.1}, {:.1}) Hz",
        freq_2d.x_range.0, freq_2d.x_range.1
    );
    println!(
        "  Y range (freq 2): ({:.1}, {:.1}) Hz",
        freq_2d.y_range.0, freq_2d.y_range.1
    );
    println!(
        "  Z range (magnitude): ({:.2}, {:.2}) dB",
        freq_2d.z_range.0, freq_2d.z_range.1
    );

    // Example 4: Custom color mapping with log scale
    println!("\n\nExample 4: Custom Color Mapping (Log X-axis)");
    println!("----------------------------------------------");

    let custom_color = SurfaceData::from_function_logx(
        (100.0, 10000.0), // 100 Hz to 10 kHz
        (0.0, 1.0),
        10,
        |freq, y| {
            let z = (freq / 1000.0).ln(); // Height based on log(freq)
            let t = y; // Color based on Y position
            (z, t)
        },
    );

    println!("Surface created with separate z and t values:");
    println!(
        "  Grid size: {}x{}",
        custom_color.rows(),
        custom_color.cols()
    );
    println!(
        "  Z range (log frequency): ({:.3}, {:.3})",
        custom_color.z_range.0, custom_color.z_range.1
    );
    println!(
        "  T range (for coloring): ({:.3}, {:.3})",
        custom_color.t_range.0, custom_color.t_range.1
    );

    // Example 5: Demonstrate logarithmic spacing
    println!("\n\nExample 5: Verifying Logarithmic Spacing");
    println!("------------------------------------------");

    let log_demo = SurfaceData::from_z_function_logx(
        (10.0, 10000.0),
        (0.0, 1.0),
        6, // Small grid for clear demonstration
        |x, _y| x,
    );

    println!("X values with logarithmic spacing:");
    for i in 0..log_demo.cols() {
        if let Some(p) = log_demo.get(0, i) {
            println!(
                "  Column {}: x = {:.2} Hz (log10 = {:.2})",
                i,
                p.x,
                p.x.log10()
            );
        }
    }

    println!("\nLog differences (should be constant):");
    let x_values: Vec<f64> = (0..log_demo.cols())
        .filter_map(|i| log_demo.get(0, i).map(|p| p.x))
        .collect();

    for i in 0..x_values.len() - 1 {
        let log_diff = x_values[i + 1].ln() - x_values[i].ln();
        println!("  ln(x[{}]) - ln(x[{}]) = {:.6}", i + 1, i, log_diff);
    }

    println!("\n=== Demo Complete ===");
    println!("\nNote: For visual rendering of these surfaces, use the showcase");
    println!("binary with GPUI enabled: cargo run --bin d3rs-showcase --release");
}
