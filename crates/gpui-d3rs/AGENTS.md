# gpui-d3rs (lib: `d3rs`, version: 0.6.0)

D3.js-inspired GPU-accelerated plotting library for GPUI.

## Key Features

- 2D and 3D GPU-accelerated charts
- Scales (linear, log, band, time, color)
- Axes, contours, shapes
- Spinorama speaker measurement visualization
- Delaunay triangulation
- Force-directed graphs
- Read `GPUI.md` at the project root before working on GPUI code

## Features

- `gpui` (default) - GPUI rendering integration
- `gpu-2d` (default) - 2D GPU-accelerated rendering
- `gpu-3d` - 3D surface rendering
- `spinorama` - Spinorama API integration for speaker data

## Binaries

- `d3rs-showcase` - Chart gallery
- `d3rs-spinorama` - Spinorama visualization demo

## Examples

15+ examples: scale, color, contour, delaunay, force, line, bar, scatter, etc.

## Testing

```bash
cargo test -p gpui-d3rs --lib
cargo check -p gpui-d3rs && cargo clippy -p gpui-d3rs
```

## Notes

- API design inspired by D3.js but adapted for Rust and GPU rendering
- Used by `app-gpui` for real-time audio visualization
