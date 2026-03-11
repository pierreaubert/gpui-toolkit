# DEVELOPMENT IS HAPPENING [HERE](https://github.com/pierreaubert/sotf) PLEASE JUMP THERE


# gpui-toolkit

A collection of libraries for building desktop applications with [GPUI](https://github.com/zed-industries/zed), the GPU-accelerated UI framework from the Zed editor.

[![License](https://img.shields.io/crates/l/gpui-ui-kit)](LICENSE)

## Crates

| Crate | Description | Docs |
|-------|-------------|------|
| [gpui-ui-kit](./crates/gpui-ui-kit/) | Reusable UI components (buttons, inputs, dialogs, etc.) | [![docs.rs](https://docs.rs/gpui-ui-kit/badge.svg)](https://docs.rs/gpui-ui-kit) |
| [gpui-ui-kit-macros](./crates/gpui-ui-kit-macros/) | Proc macros for theme generation | [![docs.rs](https://docs.rs/gpui-ui-kit-macros/badge.svg)](https://docs.rs/gpui-ui-kit-macros) |
| [gpui-d3rs](./crates/gpui-d3rs/) | D3.js-inspired data visualization library | [![docs.rs](https://docs.rs/gpui-d3rs/badge.svg)](https://docs.rs/gpui-d3rs) |
| [gpui-px](./crates/gpui-px/) | Plotly Express-style high-level charting API | [![docs.rs](https://docs.rs/gpui-px/badge.svg)](https://docs.rs/gpui-px) |
| [gpui-themes](./crates/gpui-themes/) | Theming support for gpui-ui-kit | [![docs.rs](https://docs.rs/gpui-themes/badge.svg)](https://docs.rs/gpui-themes) |
| [gpui-autoeq](./crates/gpui-autoeq/) | AutoEQ parameter form component | [![docs.rs](https://docs.rs/gpui-autoeq/badge.svg)](https://docs.rs/gpui-autoeq) |

## Versions

- **v0.5.x**: Tracks the stable GPUI 0.2.2 release from crates.io.
- **v0.6.x** (current): Tracks GPUI from Zed's main branch (pinned to rev [`450c66c`](https://github.com/zed-industries/zed/commit/450c66ce6e24ec10111fc8dd75711663b2e01b5e)).

## Overview

### gpui-ui-kit

A comprehensive UI component library with 40+ components including:
- **Core**: Button, Card, Dialog, Menu, Tabs, Toast
- **Forms**: Input, NumberInput, Checkbox, Toggle, Select, Slider, ColorPicker
- **Data Display**: Badge, Progress, Spinner, Avatar, Typography, Table
- **Audio Controls**: Potentiometer, VerticalSlider, VolumeKnob
- **Layout**: Stack, PaneDivider, Breadcrumbs
- **Flows**: Wizard, Workflow

See the [gpui-ui-kit README](./crates/gpui-ui-kit/README.md) for usage examples.

### gpui-d3rs

A port of D3.js concepts to Rust with idiomatic builder patterns:
- **Scales**: Linear, Log with automatic tick generation
- **Shapes**: Lines, Bars, Areas, Arcs, Pies, Scatter plots
- **Colors**: RGB/HSL, interpolation, categorical schemes
- **Geographic**: Mercator, Orthographic projections
- **Spatial**: QuadTree, Delaunay triangulation, Voronoi
- **Animation**: Transitions, easing functions, timers

See the [gpui-d3rs README](./crates/gpui-d3rs/README.md) for the full feature list and examples.

### gpui-px

High-level charting API inspired by Plotly Express:
- 6 chart types: Scatter, Line, Bar, Heatmap, Contour, Isoline
- Fluent builder API
- Color scales: Viridis, Plasma, Inferno, Magma, Heat, Coolwarm
- Logarithmic scale support

See the [gpui-px README](./crates/gpui-px/README.md) for quick start examples.

### gpui-autoeq

AutoEQ parameter form component for building speaker/headphone EQ optimization interfaces.

## Installation

Since v0.6 tracks GPUI from git, add the crates via git dependency:

```toml
[dependencies]
gpui-ui-kit = { git = "https://github.com/pierreaubert/gpui-toolkit.git", version = "0.6" }
gpui-d3rs = { git = "https://github.com/pierreaubert/gpui-toolkit.git", version = "0.6" }
gpui-px = { git = "https://github.com/pierreaubert/gpui-toolkit.git", version = "0.6" }
```

For v0.5 (stable crates.io GPUI):

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
