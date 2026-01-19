//! d3-color demonstration
//!
//! This example demonstrates the color utilities:
//! - RGB color creation and manipulation
//! - HSL color space
//! - Color schemes (categorical)
//! - Color interpolation

use d3rs::color::{ColorScheme, D3Color, interpolate_colors, sequential_color};
use d3rs::interpolate::{interpolate_hsl, interpolate_lab, interpolate_rgb};

fn main() {
    println!("=== d3-color Demonstration ===\n");

    // ========================================
    // Color Creation
    // ========================================
    println!("--- Color Creation ---\n");

    // RGB
    let red = D3Color::rgb(255, 0, 0);
    let green = D3Color::rgb(0, 255, 0);
    let blue = D3Color::rgb(0, 0, 255);

    println!("RGB colors:");
    println!(
        "  Red:   {} | r={:.2}, g={:.2}, b={:.2}",
        red.to_hex(),
        red.r,
        red.g,
        red.b
    );
    println!(
        "  Green: {} | r={:.2}, g={:.2}, b={:.2}",
        green.to_hex(),
        green.r,
        green.g,
        green.b
    );
    println!(
        "  Blue:  {} | r={:.2}, g={:.2}, b={:.2}",
        blue.to_hex(),
        blue.r,
        blue.g,
        blue.b
    );

    // From hex
    let coral = D3Color::from_hex(0xFF7F50);
    let teal = D3Color::from_hex(0x008080);
    let gold = D3Color::from_hex(0xFFD700);

    println!("\nFrom hex:");
    println!("  Coral:  {}", coral.to_hex());
    println!("  Teal:   {}", teal.to_hex());
    println!("  Gold:   {}", gold.to_hex());

    // From HSL
    let orange_hsl = D3Color::from_hsl(30.0, 1.0, 0.5); // Orange
    let purple_hsl = D3Color::from_hsl(270.0, 0.8, 0.4); // Purple
    let cyan_hsl = D3Color::from_hsl(180.0, 1.0, 0.5); // Cyan

    println!("\nFrom HSL:");
    println!("  HSL(30 deg, 100%, 50%) = {}", orange_hsl.to_hex());
    println!("  HSL(270 deg, 80%, 40%) = {}", purple_hsl.to_hex());
    println!("  HSL(180 deg, 100%, 50%) = {}", cyan_hsl.to_hex());

    // ========================================
    // Color Properties
    // ========================================
    println!("\n--- Color Properties ---\n");

    let sample = D3Color::from_hex(0x4682B4); // Steel Blue
    println!("Steel Blue ({}):", sample.to_hex());
    println!(
        "  RGB (0-1): ({:.2}, {:.2}, {:.2})",
        sample.r, sample.g, sample.b
    );
    println!("  Opacity: {}", sample.opacity());
    println!("  Luminance: {:.3}", sample.luminance());

    // Brightness comparison
    let colors_to_compare = vec![
        ("White", D3Color::rgb(255, 255, 255)),
        ("Light Gray", D3Color::rgb(192, 192, 192)),
        ("Gray", D3Color::rgb(128, 128, 128)),
        ("Dark Gray", D3Color::rgb(64, 64, 64)),
        ("Black", D3Color::rgb(0, 0, 0)),
    ];

    println!("\nLuminance values:");
    for (name, color) in &colors_to_compare {
        println!("  {:12}: {:.3}", name, color.luminance());
    }

    // ========================================
    // Color Manipulation
    // ========================================
    println!("\n--- Color Manipulation ---\n");

    let base = D3Color::from_hex(0x3366CC);
    println!("Base color: {}", base.to_hex());

    // Lighten/darken
    let lighter = base.brighter(1.0);
    let darkened = base.darker(1.0);
    println!("  Brighter: {}", lighter.to_hex());
    println!("  Darker:   {}", darkened.to_hex());

    // With opacity
    let transparent = base.with_opacity(0.5);
    println!(
        "  50% opacity: {} (a={})",
        transparent.to_hex_alpha(),
        transparent.opacity()
    );

    // ========================================
    // Color Interpolation
    // ========================================
    println!("\n--- Color Interpolation ---\n");

    let from = D3Color::rgb(255, 0, 0); // Red
    let to = D3Color::rgb(0, 0, 255); // Blue

    println!("Interpolating from {} to {}:\n", from.to_hex(), to.to_hex());

    // RGB interpolation
    let rgb_interp = interpolate_rgb(from, to);
    print!("  RGB:   ");
    for i in 0..=8 {
        let t = i as f64 / 8.0;
        print!("{} ", rgb_interp(t).to_hex());
    }
    println!();

    // HSL interpolation
    let hsl_interp = interpolate_hsl(from, to);
    print!("  HSL:   ");
    for i in 0..=8 {
        let t = i as f64 / 8.0;
        print!("{} ", hsl_interp(t).to_hex());
    }
    println!();

    // LAB interpolation (perceptually uniform)
    let lab_interp = interpolate_lab(from, to);
    print!("  LAB:   ");
    for i in 0..=8 {
        let t = i as f64 / 8.0;
        print!("{} ", lab_interp(t).to_hex());
    }
    println!();

    // ========================================
    // Multi-color Gradient
    // ========================================
    println!("\n--- Multi-color Gradient ---\n");

    let gradient_colors = vec![
        D3Color::from_hex(0x440154), // Viridis start
        D3Color::from_hex(0x21918c),
        D3Color::from_hex(0x5ec962),
        D3Color::from_hex(0xfde725), // Viridis end
    ];

    print!("Viridis-like gradient: ");
    for i in 0..=10 {
        let t = i as f32 / 10.0;
        let color = interpolate_colors(&gradient_colors, t);
        print!("{} ", color.to_hex());
    }
    println!();

    // ========================================
    // Sequential Color Scale
    // ========================================
    println!("\n--- Sequential Color Scale (diverging) ---\n");

    print!("Diverging (blue-white-red): ");
    for i in 0..=10 {
        let t = i as f32 / 10.0;
        let color = sequential_color(t);
        print!("{} ", color.to_hex());
    }
    println!();

    // ========================================
    // Categorical Color Schemes
    // ========================================
    println!("\n--- Categorical Color Schemes ---\n");

    let schemes: Vec<(&str, ColorScheme)> = vec![
        ("Category10", ColorScheme::category10()),
        ("Tableau10", ColorScheme::tableau10()),
        ("Pastel", ColorScheme::pastel()),
    ];

    for (name, scheme) in &schemes {
        let colors = scheme.colors();
        print!("  {:12}: ", name);
        for color in colors.iter().take(8) {
            print!("{} ", color.to_hex());
        }
        if colors.len() > 8 {
            print!("...");
        }
        println!();
    }

    // ========================================
    // Using Colors for Data
    // ========================================
    println!("\n--- Using Colors for Data ---\n");

    let categories = vec!["Apple", "Banana", "Cherry", "Date", "Elderberry"];
    let scheme = ColorScheme::category10();
    let colors = scheme.colors();

    println!("Assigning colors to categories:");
    for (i, category) in categories.iter().enumerate() {
        let color = &colors[i % colors.len()];
        println!("  {}: {}", category, color.to_hex());
    }

    // ========================================
    // Heat Map Colors
    // ========================================
    println!("\n--- Heat Map Visualization ---\n");

    let data = vec![
        vec![0.1, 0.3, 0.5, 0.2],
        vec![0.4, 0.8, 0.6, 0.3],
        vec![0.2, 0.5, 0.9, 0.7],
        vec![0.1, 0.2, 0.4, 0.3],
    ];

    let heat_colors = vec![
        D3Color::from_hex(0x0571b0), // Cool (blue)
        D3Color::from_hex(0xf7f7f7), // Neutral (white)
        D3Color::from_hex(0xca0020), // Hot (red)
    ];

    println!("Heat map (4x4):");
    for row in &data {
        print!("  ");
        for &val in row {
            let color = interpolate_colors(&heat_colors, val as f32);
            print!("{} ", color.to_hex());
        }
        println!();
    }

    // ========================================
    // Contrast and Accessibility
    // ========================================
    println!("\n--- Contrast & Accessibility ---\n");

    let test_colors = vec![
        ("Red", D3Color::rgb(255, 0, 0)),
        ("Green", D3Color::rgb(0, 128, 0)),
        ("Blue", D3Color::rgb(0, 0, 255)),
        ("Yellow", D3Color::rgb(255, 255, 0)),
        ("Purple", D3Color::rgb(128, 0, 128)),
    ];

    println!("Luminance-based contrast:");
    for (name, color) in &test_colors {
        let lum = color.luminance();
        let better_bg = if lum > 0.5 { "black" } else { "white" };
        println!(
            "  {:8} ({}) - luminance: {:.3} - better on {}",
            name,
            color.to_hex(),
            lum,
            better_bg
        );
    }

    println!("\n=== End of d3-color Demo ===");
}
