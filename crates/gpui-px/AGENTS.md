# gpui-px (lib: `gpui-px`, version: 0.6.0)

High-level Plotly Express-style charting API built on gpui-d3rs.

## Purpose

Provides a simple, high-level API for creating charts, similar to Plotly Express in Python. Built on top of `gpui-d3rs`. Read `GPUI.md` at the project root before working on GPUI code.

## Features

- `gpui` (default)
- `gpu-2d` (default) - 2D GPU rendering
- `gpu-3d` - 3D GPU rendering

## Binaries

- `px-showcase` - Chart showcase
- `px-spinorama` - Spinorama visualization demo

## Examples

```bash
cargo run --release --example logscale_demo -p gpui-px
```

## Testing

```bash
cargo check -p gpui-px && cargo clippy -p gpui-px
```
