# gpui-toolkit

A collection of libraries for building desktop applications with [GPUI](https://github.com/zed-industries/zed), the GPU-accelerated UI framework from the Zed editor.

[![License](https://img.shields.io/crates/l/gpui-ui-kit)](LICENSE)

## Crates

| Crate | Description | Docs |
|-------|-------------|------|
| [gpui-ui-kit](./gpui-ui-kit/) | Reusable UI components (buttons, inputs, dialogs, etc.) | [![docs.rs](https://docs.rs/gpui-ui-kit/badge.svg)](https://docs.rs/gpui-ui-kit) |
| [gpui-d3rs](./gpui-d3rs/) | D3.js-inspired data visualization library | [![docs.rs](https://docs.rs/gpui-d3rs/badge.svg)](https://docs.rs/gpui-d3rs) |
| [gpui-px](./gpui-px/) | Plotly Express-style high-level charting API | [![docs.rs](https://docs.rs/gpui-px/badge.svg)](https://docs.rs/gpui-px) |
| [gpui-themes](./gpui-themes/) | Theming support for gpui-ui-kit | [![docs.rs](https://docs.rs/gpui-themes/badge.svg)](https://docs.rs/gpui-themes) |

## Overview

### gpui-ui-kit

A comprehensive UI component library with 40+ components including:
- **Core**: Button, Card, Dialog, Menu, Tabs, Toast
- **Forms**: Input, NumberInput, Checkbox, Toggle, Select, Slider, ColorPicker
- **Data Display**: Badge, Progress, Spinner, Avatar, Typography
- **Audio Controls**: Potentiometer, VerticalSlider, VolumeKnob

See the [gpui-ui-kit README](./gpui-ui-kit/README.md) for usage examples.

### gpui-d3rs

A port of D3.js concepts to Rust with idiomatic builder patterns:
- **Scales**: Linear, Log with automatic tick generation
- **Shapes**: Lines, Bars, Areas, Arcs, Pies, Scatter plots
- **Colors**: RGB/HSL, interpolation, categorical schemes
- **Geographic**: Mercator, Orthographic projections
- **Spatial**: QuadTree, Delaunay triangulation, Voronoi
- **Animation**: Transitions, easing functions, timers

See the [gpui-d3rs README](./gpui-d3rs/README.md) for the full feature list and examples.

### gpui-px

High-level charting API inspired by Plotly Express:
- 6 chart types: Scatter, Line, Bar, Heatmap, Contour, Isoline
- Fluent builder API
- Color scales: Viridis, Plasma, Inferno, Magma, Heat, Coolwarm
- Logarithmic scale support

See the [gpui-px README](./gpui-px/README.md) for quick start examples.

## Installation

Add the crates you need to your `Cargo.toml`:

```toml
[dependencies]
gpui-ui-kit = "0.5"
gpui-d3rs = "0.5"
gpui-px = "0.5"
gpui = "0.2"
```

## Quick Example

```rust
use gpui::*;
use gpui_ui_kit::{Button, ButtonVariant, Card};
use gpui_px::scatter;

// UI Component
let button = Button::new("submit", "Submit")
    .variant(ButtonVariant::Primary)
    .on_click(|_, _| println!("Clicked!"));

// Chart
let chart = scatter(&x_data, &y_data)
    .title("My Data")
    .build()?;
```

## Showcases

Each library includes interactive showcases:

```bash
# UI Kit showcase
cargo run -p gpui-ui-kit --example showcase

# D3rs showcase
cargo run -p gpui-d3rs --bin d3rs-showcase --release

# Px showcase
cargo run -p gpui-px --bin gpui-px-showcase
```

## License

[ISC License](LICENSE)
