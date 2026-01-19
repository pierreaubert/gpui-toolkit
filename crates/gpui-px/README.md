# gpui-px

[![Crates.io](https://img.shields.io/crates/v/gpui-px)](https://crates.io/crates/gpui-px)
[![Documentation](https://docs.rs/gpui-px/badge.svg)](https://docs.rs/gpui-px)
[![License](https://img.shields.io/crates/l/gpui-px)](LICENSE)

High-level Plotly Express-style charting API for [GPUI](https://gpui.rs). Create beautiful, interactive charts in just a few lines of Rust.

Built on top of [gpui-d3rs](https://crates.io/crates/gpui-d3rs) primitives.

## Features

- **6 chart types**: Scatter, Line, Bar, Heatmap, Contour, Isoline
- **Fluent builder API**: Chain methods for easy configuration
- **Color scales**: Viridis, Plasma, Inferno, Magma, Heat, Coolwarm, Greys, or custom
- **Logarithmic scales**: Both axes support log scaling for multi-magnitude data
- **Validation**: Comprehensive error handling for invalid data

## Installation

```toml
[dependencies]
gpui-px = "0.1"
```

## Quick Start

```rust
use gpui_px::{scatter, line, bar, heatmap, contour, isoline};

// Create a scatter plot in 3 lines
let chart = scatter(&x_data, &y_data)
    .title("My Chart")
    .build()?;
```

## Chart Types

### Scatter

Displays individual data points with x,y coordinates. Ideal for exploring correlations, identifying clusters, and spotting outliers.

```rust
use gpui_px::scatter;

let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
let y = vec![2.3, 4.1, 3.5, 5.2, 4.8];

let chart = scatter(&x, &y)
    .title("Correlation Analysis")
    .color(0x1f77b4)      // Plotly blue
    .point_radius(6.0)
    .opacity(0.8)
    .size(600.0, 400.0)
    .build()?;
```

### Line

Connects data points to show trends over continuous domains. Perfect for time series, measurements, and sequential data.

```rust
use gpui_px::{line, CurveType};

let time: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
let signal: Vec<f64> = time.iter().map(|&t| (t * 2.0).sin()).collect();

let chart = line(&time, &signal)
    .title("Sine Wave")
    .color(0xff7f0e)       // Orange
    .stroke_width(2.5)
    .curve(CurveType::Linear)
    .show_points(false)
    .size(600.0, 400.0)
    .build()?;
```

### Bar

Compares values across discrete categories. Great for rankings, counts, and distributions.

```rust
use gpui_px::bar;

let categories = vec!["Mon", "Tue", "Wed", "Thu", "Fri"];
let values = vec![45.0, 62.0, 38.0, 78.0, 56.0];

let chart = bar(&categories, &values)
    .title("Weekly Sales")
    .color(0x2ca02c)       // Green
    .bar_gap(4.0)
    .border_radius(3.0)
    .size(600.0, 400.0)
    .build()?;
```

### Heatmap

Visualizes 2D scalar fields using color. Perfect for spectrograms, correlation matrices, and geographic data.

```rust
use gpui_px::{heatmap, ColorScale};

// 10x10 grid of values
let z: Vec<f64> = (0..100).map(|i| {
    let x = (i % 10) as f64 / 10.0;
    let y = (i / 10) as f64 / 10.0;
    (x * 3.14).sin() * (y * 3.14).cos()
}).collect();

let chart = heatmap(&z, 10, 10)
    .title("Interference Pattern")
    .color_scale(ColorScale::Viridis)
    .size(500.0, 500.0)
    .build()?;
```

### Contour

Shows filled bands between threshold values. Great for topographic visualizations and density estimation.

```rust
use gpui_px::{contour, ColorScale};

// Generate a 2D Gaussian
let size = 50;
let z: Vec<f64> = (0..size*size).map(|i| {
    let x = (i % size) as f64 / size as f64 * 4.0 - 2.0;
    let y = (i / size) as f64 / size as f64 * 4.0 - 2.0;
    (-x*x - y*y).exp()
}).collect();

let chart = contour(&z, size, size)
    .title("Gaussian Distribution")
    .thresholds(vec![0.1, 0.3, 0.5, 0.7, 0.9])
    .color_scale(ColorScale::Plasma)
    .size(500.0, 500.0)
    .build()?;
```

### Isoline

Draws unfilled contour lines at specific levels. Useful for elevation maps, pressure fields, and level curves.

```rust
use gpui_px::isoline;

// Same data as contour example
let z: Vec<f64> = /* ... */;

let chart = isoline(&z, 50, 50)
    .title("Elevation Contours")
    .levels(vec![0.2, 0.4, 0.6, 0.8])
    .color(0x333333)
    .stroke_width(1.5)
    .size(500.0, 500.0)
    .build()?;
```

## Logarithmic Scales

All chart types support logarithmic axis scaling for data spanning multiple orders of magnitude.

```rust
use gpui_px::{scatter, line, bar, ScaleType};

// Log-log scatter plot (power-law relationships)
let chart = scatter(&x, &y)
    .x_scale(ScaleType::Log)
    .y_scale(ScaleType::Log)
    .build()?;

// Frequency response (audio engineering)
let freq: Vec<f64> = (0..50).map(|i| 20.0 * 10_f64.powf(i as f64 / 15.0)).collect();
let chart = line(&freq, &magnitude_db)
    .x_scale(ScaleType::Log)
    .title("Frequency Response (20 Hz - 20 kHz)")
    .build()?;

// Bar chart with log Y-axis
let chart = bar(&["10", "100", "1K", "10K"], &[10.0, 100.0, 1000.0, 10000.0])
    .y_scale(ScaleType::Log)
    .build()?;
```

**Note**: Logarithmic scales require all values to be positive. Zero or negative values will cause validation errors.

## Color Scales

For 2D charts (heatmap, contour), use the `ColorScale` enum:

| Scale | Description |
|-------|-------------|
| `Viridis` | Perceptually uniform, colorblind-friendly (default) |
| `Plasma` | Perceptually uniform, purple-orange-yellow |
| `Inferno` | Perceptually uniform, black-purple-orange-yellow |
| `Magma` | Perceptually uniform, black-purple-orange-white |
| `Heat` | Diverging, blue-white-red |
| `Coolwarm` | Diverging, cool blue to warm red |
| `Greys` | Sequential grayscale |

Custom color scales:

```rust
use gpui_px::ColorScale;
use d3rs::color::D3Color;

let custom = ColorScale::custom(|t| {
    // t is in [0, 1]
    D3Color::from_hex(0x0000ff).interpolate(
        &D3Color::from_hex(0xff0000),
        t as f32
    )
});
```

## Color Format

For 1D charts (scatter, line, bar, isoline), colors use 24-bit RGB hex values:

```rust
.color(0x1f77b4)  // Plotly blue
.color(0xff7f0e)  // Plotly orange
.color(0x2ca02c)  // Plotly green
.color(0xd62728)  // Plotly red
.color(0x9467bd)  // Plotly purple
```

## Showcase

Run the interactive showcase to see all chart types:

```bash
cargo run --bin gpui-px-showcase
```

![Showcase Gallery](docs/gallery.png)

## API Overview

All chart builders share a common pattern:

```rust
chart_type(&data)           // Create builder with required data
    .title("...")           // Optional title
    .color(0xRRGGBB)        // Color (1D charts)
    .color_scale(scale)     // Color scale (2D charts)
    .size(w, h)             // Chart dimensions
    .build()?               // Validate and build
```

### Scatter

| Method | Description |
|--------|-------------|
| `title(s)` | Chart title |
| `color(hex)` | Point color |
| `point_radius(r)` | Point size in pixels |
| `opacity(o)` | Point opacity (0.0-1.0) |
| `x_scale(type)` | X-axis scale (Linear/Log) |
| `y_scale(type)` | Y-axis scale (Linear/Log) |
| `size(w, h)` | Chart dimensions |

### Line

| Method | Description |
|--------|-------------|
| `title(s)` | Chart title |
| `color(hex)` | Line color |
| `stroke_width(w)` | Line width in pixels |
| `opacity(o)` | Line opacity (0.0-1.0) |
| `curve(type)` | Interpolation (Linear, etc.) |
| `show_points(b)` | Show data point markers |
| `x_scale(type)` | X-axis scale (Linear/Log) |
| `y_scale(type)` | Y-axis scale (Linear/Log) |
| `size(w, h)` | Chart dimensions |

### Bar

| Method | Description |
|--------|-------------|
| `title(s)` | Chart title |
| `color(hex)` | Bar fill color |
| `opacity(o)` | Bar opacity (0.0-1.0) |
| `bar_gap(g)` | Gap between bars in pixels |
| `border_radius(r)` | Corner radius |
| `y_scale(type)` | Y-axis scale (Linear/Log) |
| `size(w, h)` | Chart dimensions |

### Heatmap

| Method | Description |
|--------|-------------|
| `title(s)` | Chart title |
| `x(values)` | Custom x-axis values |
| `y(values)` | Custom y-axis values |
| `color_scale(scale)` | Color mapping |
| `opacity(o)` | Fill opacity (0.0-1.0) |
| `x_scale(type)` | X-axis scale (Linear/Log) |
| `y_scale(type)` | Y-axis scale (Linear/Log) |
| `size(w, h)` | Chart dimensions |

### Contour

| Method | Description |
|--------|-------------|
| `title(s)` | Chart title |
| `x(values)` | Custom x-axis values |
| `y(values)` | Custom y-axis values |
| `thresholds(vec)` | Threshold values for bands |
| `color_scale(scale)` | Color mapping |
| `opacity(o)` | Fill opacity (0.0-1.0) |
| `x_scale(type)` | X-axis scale (Linear/Log) |
| `y_scale(type)` | Y-axis scale (Linear/Log) |
| `size(w, h)` | Chart dimensions |

### Isoline

| Method | Description |
|--------|-------------|
| `title(s)` | Chart title |
| `x(values)` | Custom x-axis values |
| `y(values)` | Custom y-axis values |
| `levels(vec)` | Level values for lines |
| `color(hex)` | Line color |
| `stroke_width(w)` | Line width in pixels |
| `opacity(o)` | Line opacity (0.0-1.0) |
| `x_scale(type)` | X-axis scale (Linear/Log) |
| `y_scale(type)` | Y-axis scale (Linear/Log) |
| `size(w, h)` | Chart dimensions |

## Coordinate System

All charts use standard mathematical coordinates:
- **Y-axis**: 0 at bottom, increases upward
- **X-axis**: 0 at left, increases rightward

## Error Handling

The `build()` method returns `Result<impl IntoElement, ChartError>`:

```rust
use gpui_px::ChartError;

match scatter(&x, &y).build() {
    Ok(chart) => { /* use chart */ }
    Err(ChartError::EmptyData { field }) => {
        println!("Empty data in {}", field);
    }
    Err(ChartError::DataLengthMismatch { .. }) => {
        println!("X and Y must have same length");
    }
    Err(e) => println!("Error: {:?}", e),
}
```

## License

- [ISC License](https://en.wikipedia.org/wiki/ISC_license)
