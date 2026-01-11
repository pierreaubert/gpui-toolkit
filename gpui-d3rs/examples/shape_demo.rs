//! d3-shape demonstration
//!
//! This example demonstrates the shape utilities inspired by d3-shape:
//! - Pie and Arc generators
//! - Area generators
//! - Curve types
//! - Symbols
//! - Stack layouts
//! - Path building

use d3rs::shape::{
    arc::{Arc, ArcDatum},
    // Area
    area::Area,
    // Curves
    curve::Curve,
    // Path
    path::{PathBuilder, Point},
    // Pie/Arc
    pie::{Pie, donut, half_pie, pie},
    // Stack
    stack::{Stack, StackOffset, StackOrder},
    // Symbols
    symbol::{Symbol, SymbolType},
};

fn main() {
    println!("=== d3-shape Demonstration ===\n");

    // ========================================
    // Pie Charts
    // ========================================
    println!("--- Pie Charts ---\n");

    let data = vec![30.0, 20.0, 15.0, 25.0, 10.0];
    let labels = vec!["A", "B", "C", "D", "E"];

    println!("Data: {:?}", data);
    println!("Labels: {:?}\n", labels);

    // Full pie using helper function
    let slices = pie(&data, 100.0);

    println!("Full Pie Layout:");
    for (i, slice) in slices.iter().enumerate() {
        let degrees_start = slice.arc.start_angle.to_degrees();
        let degrees_end = slice.arc.end_angle.to_degrees();
        println!(
            "  {}: value={:5.1}, angle=[{:6.1} deg, {:6.1} deg]",
            labels[i], slice.value, degrees_start, degrees_end
        );
    }

    // Donut (with inner radius)
    let donut_slices = donut(&data, 50.0, 100.0);
    println!("\nDonut Layout (inner=50, outer=100):");
    for (i, slice) in donut_slices.iter().enumerate() {
        println!(
            "  {}: [{:6.1} deg, {:6.1} deg], inner={}, outer={}",
            labels[i],
            slice.arc.start_angle.to_degrees(),
            slice.arc.end_angle.to_degrees(),
            slice.arc.inner_radius,
            slice.arc.outer_radius
        );
    }

    // Half pie using the Pie builder
    let half = half_pie(&data, 100.0);
    println!("\nHalf Pie (semicircle):");
    for (i, slice) in half.iter().enumerate() {
        println!(
            "  {}: [{:6.1} deg, {:6.1} deg]",
            labels[i],
            slice.arc.start_angle.to_degrees(),
            slice.arc.end_angle.to_degrees()
        );
    }

    // Custom pie with padding
    let custom_pie = Pie::new().outer_radius(100.0).pad_angle(0.05);
    let custom_slices = custom_pie.generate(&data, |v| *v);
    println!("\nCustom Pie (with pad_angle=0.05):");
    for (i, slice) in custom_slices.iter().enumerate() {
        println!(
            "  {}: [{:6.1} deg, {:6.1} deg]",
            labels[i],
            slice.arc.start_angle.to_degrees(),
            slice.arc.end_angle.to_degrees()
        );
    }

    // ========================================
    // Arc Generator
    // ========================================
    println!("\n--- Arc Generator ---\n");

    let arc = Arc::new();

    // Create an arc datum manually
    let arc_datum = ArcDatum::new()
        .inner_radius(40.0)
        .outer_radius(80.0)
        .start_angle(0.0)
        .end_angle(std::f64::consts::PI);

    let path = arc.generate(&arc_datum);

    println!("Arc for semicircle (inner=40, outer=80):");
    println!("  Centroid: {:?}", arc_datum.centroid());
    println!("  Path commands: {} points", path.commands().len());
    let svg = path.to_svg_string();
    println!("  SVG: {}", if svg.len() > 80 { &svg[..80] } else { &svg });

    // ========================================
    // Area Generator
    // ========================================
    println!("\n--- Area Generator ---\n");

    let area_data: Vec<(f64, f64)> = vec![
        (0.0, 30.0),
        (1.0, 50.0),
        (2.0, 40.0),
        (3.0, 60.0),
        (4.0, 35.0),
    ];

    let area = Area::new()
        .x(|d: &(f64, f64)| d.0 * 50.0)
        .y0(|_| 100.0)
        .y1(|d: &(f64, f64)| 100.0 - d.1)
        .curve(Curve::Cardinal { tension: 0.5 });

    let area_path = area.generate(&area_data);
    println!("Area shape for data points:");
    println!("  Data points: {}", area_data.len());
    println!("  Curve type: Cardinal (tension=0.5)");
    println!("  Path commands: {}", area_path.commands().len());

    // ========================================
    // Curve Interpolation
    // ========================================
    println!("\n--- Curve Interpolation ---\n");

    let control_points = vec![
        Point::new(0.0, 50.0),
        Point::new(50.0, 10.0),
        Point::new(100.0, 80.0),
        Point::new(150.0, 30.0),
        Point::new(200.0, 60.0),
    ];

    println!(
        "Control points: {:?}",
        control_points
            .iter()
            .map(|p| (p.x, p.y))
            .collect::<Vec<_>>()
    );

    let curve_types = [
        ("Linear", Curve::Linear),
        ("Step", Curve::Step),
        ("StepBefore", Curve::StepBefore),
        ("StepAfter", Curve::StepAfter),
        ("Basis", Curve::Basis),
        ("Cardinal(0.0)", Curve::Cardinal { tension: 0.0 }),
        ("Cardinal(0.5)", Curve::Cardinal { tension: 0.5 }),
        ("CatmullRom(0.5)", Curve::CatmullRom { alpha: 0.5 }),
        ("MonotoneX", Curve::MonotoneX),
        ("Natural", Curve::Natural),
    ];

    println!("\nCurve interpolation results:");
    for (name, curve) in curve_types.iter() {
        let interpolated = curve.interpolate(&control_points);
        println!("  {:20}: {} points", name, interpolated.len());
    }

    // ========================================
    // Symbols
    // ========================================
    println!("\n--- Symbols ---\n");

    let symbol_types = [
        SymbolType::Circle,
        SymbolType::Cross,
        SymbolType::Diamond,
        SymbolType::Square,
        SymbolType::Star,
        SymbolType::Triangle,
        SymbolType::TriangleDown,
        SymbolType::Wye,
    ];

    println!("Available symbol types:");
    for symbol_type in symbol_types.iter() {
        let symbol = Symbol::new(*symbol_type, 64.0);
        let path = symbol.generate();
        println!(
            "  {:15}: {} commands",
            format!("{:?}", symbol_type),
            path.commands().len()
        );
    }

    // Star with custom points
    let custom_star = Symbol::new(SymbolType::Star, 100.0);
    let star_path = custom_star.generate();
    println!("\nCustom Star (size=100):");
    println!("  Path commands: {}", star_path.commands().len());

    // ========================================
    // Stack Layout
    // ========================================
    println!("\n--- Stack Layout ---\n");

    // Multi-series data for stacked chart
    // Each row is a time point, each column is a series
    let stack_data = vec![
        vec![10.0, 15.0, 12.0], // Time 0: Series A, B, C
        vec![20.0, 25.0, 18.0], // Time 1
        vec![30.0, 20.0, 25.0], // Time 2
    ];

    println!("Input data (3 time points, 3 series):");
    for (i, row) in stack_data.iter().enumerate() {
        println!("  Time {}: {:?}", i, row);
    }

    // Standard stack (None offset)
    let stack = Stack::new()
        .keys(vec!["A".to_string(), "B".to_string(), "C".to_string()])
        .order(StackOrder::None)
        .offset(StackOffset::None);

    let stacked = stack.generate(&stack_data);
    println!("\nStandard stack (zero baseline):");
    for series in &stacked {
        let y0s: Vec<f64> = series.values.iter().map(|v| v[0]).collect();
        let y1s: Vec<f64> = series.values.iter().map(|v| v[1]).collect();
        println!(
            "  {}: y0={:?}, y1={:?}",
            series.key,
            y0s.iter().map(|v| v.round()).collect::<Vec<_>>(),
            y1s.iter().map(|v| v.round()).collect::<Vec<_>>()
        );
    }

    // Expanding stack (normalized to 100%)
    let expand_stack = Stack::new()
        .keys(vec!["A".to_string(), "B".to_string(), "C".to_string()])
        .offset(StackOffset::Expand);

    let expanded = expand_stack.generate(&stack_data);
    println!("\nExpanded stack (normalized 0-1):");
    for series in &expanded {
        println!(
            "  {}: y0=[{:.2}, {:.2}, {:.2}], y1=[{:.2}, {:.2}, {:.2}]",
            series.key,
            series.values[0][0],
            series.values[1][0],
            series.values[2][0],
            series.values[0][1],
            series.values[1][1],
            series.values[2][1]
        );
    }

    // Wiggle (stream graph)
    let wiggle_stack = Stack::new()
        .keys(vec!["A".to_string(), "B".to_string(), "C".to_string()])
        .offset(StackOffset::Wiggle);

    let wiggled = wiggle_stack.generate(&stack_data);
    println!("\nWiggle stack (stream graph):");
    for series in &wiggled {
        println!(
            "  {}: y0=[{:6.1}, {:6.1}, {:6.1}]",
            series.key, series.values[0][0], series.values[1][0], series.values[2][0]
        );
    }

    // ========================================
    // Path Builder
    // ========================================
    println!("\n--- Path Builder ---\n");

    // Build a custom path
    let path = PathBuilder::new()
        .move_to(0.0, 0.0)
        .line_to(100.0, 0.0)
        .line_to(100.0, 50.0)
        .quadratic_curve_to(50.0, 100.0, 0.0, 50.0)
        .close_path()
        .build();

    println!("Custom path with quadratic bezier:");
    println!("  Commands: moveto, lineto, lineto, quadratic, close");
    println!("  SVG d: {}", path.to_svg_string());
    println!("  Bounds: {:?}", path.bounds());

    // Arc in path
    let arc_path = PathBuilder::new()
        .move_to(50.0, 0.0)
        .arc(50.0, 50.0, 50.0, 0.0, std::f64::consts::PI, false)
        .close_path()
        .build();

    println!("\nSemicircle arc:");
    println!("  SVG d: {}", arc_path.to_svg_string());

    println!("\n=== End of d3-shape Demo ===");
}
