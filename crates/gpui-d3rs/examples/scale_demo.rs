//! Scale demonstration example
//!
//! This example demonstrates the linear and logarithmic scales with tick generation.

use d3rs::color::{ColorScheme, D3Color};
use d3rs::scale::{LinearScale, LogScale, Scale};

fn main() {
    println!("=== d3rs Scale Demo ===\n");

    // Linear scale example
    println!("Linear Scale (0-100 → 0-500):");
    let linear = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);

    for value in [0.0, 25.0, 50.0, 75.0, 100.0] {
        let scaled = linear.scale(value);
        println!("  {:.1} → {:.1}", value, scaled);
    }

    println!("\nLinear scale ticks:");
    let ticks = linear.ticks(10);
    for tick in &ticks {
        println!("  {:.1}", tick);
    }

    // Logarithmic scale example
    println!("\n\nLogarithmic Scale (20Hz-20kHz → 0-1):");
    let log_scale = LogScale::new().domain(20.0, 20000.0).range(0.0, 1.0);

    for freq in [20.0, 100.0, 1000.0, 10000.0, 20000.0] {
        let position = log_scale.scale(freq);
        println!("  {:.0}Hz → {:.3}", freq, position);
    }

    println!("\nLog scale ticks:");
    let log_ticks = log_scale.ticks(10);
    for tick in &log_ticks {
        println!("  {:.0}Hz", tick);
    }

    // Inverted range example (for screen coordinates)
    println!("\n\nInverted Range (dB scale: -24 to +24 → 400 to 0):");
    let db_scale = LinearScale::new().domain(-24.0, 24.0).range(400.0, 0.0); // Inverted: higher dB = lower y position

    for db in [-24.0, -12.0, 0.0, 12.0, 24.0] {
        let y_pos = db_scale.scale(db);
        println!("  {:+.0}dB → y={:.0}px", db, y_pos);
    }

    // Scale inversion example
    println!("\n\nScale Inversion:");
    let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);

    for range_val in [0.0, 125.0, 250.0, 375.0, 500.0] {
        if let Some(domain_val) = scale.invert(range_val) {
            println!("  {:.0}px → {:.1}", range_val, domain_val);
        }
    }

    // Color scheme example
    println!("\n\n=== Color Schemes ===\n");

    println!("Category10 colors:");
    let scheme = ColorScheme::category10();
    for i in 0..10 {
        let color = scheme.color(i);
        println!(
            "  Color {}: R={:.2} G={:.2} B={:.2}",
            i, color.r, color.g, color.b
        );
    }

    println!("\nColor interpolation (Red → Blue):");
    let red = D3Color::rgb(255, 0, 0);
    let blue = D3Color::rgb(0, 0, 255);

    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let color = red.interpolate(&blue, t);
        println!(
            "  t={:.2}: R={:.2} G={:.2} B={:.2}",
            t, color.r, color.g, color.b
        );
    }

    println!("\n=== Demo Complete ===");
}
