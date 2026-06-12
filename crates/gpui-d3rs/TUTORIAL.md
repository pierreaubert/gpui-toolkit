# gpui-d3rs Tutorial

`gpui-d3rs` provides D3.js-inspired visualization primitives in Rust.

## 1. Add the crate

```toml
[dependencies]
gpui-d3rs = { workspace = true, features = ["gpui", "gpu-2d"] }
```

Use `gpu-3d` for surface and sphere-gallery demos.

## 2. Start with scales

```rust
use d3rs::scale::LinearScale;

let x = LinearScale::new()
    .domain([0.0, 100.0])
    .range([0.0, 640.0]);

let px = x.scale(42.0);
```

## 3. Add geometry

```rust
use d3rs::delaunay::Delaunay;

let points = vec![(0.0, 0.0), (1.0, 0.0), (0.4, 0.8)];
let mesh = Delaunay::new(&points);
let nearest = mesh.find(0.5, 0.5, None);
let triangles: Vec<_> = mesh.triangles().collect();
```

## 4. Use modules by visualization type

- `scale` for linear, log, ordinal, quantile, threshold, and symlog scales
- `shape` for lines, arcs, pies, bars, stacks, symbols, and paths
- `color` and `interpolate` for palettes and transitions
- `geo` for map projections
- `hierarchy`, `force`, `quadtree`, `delaunay`, and `sankey` for spatial data
- `gpu2d` and `gpu3d` for accelerated rendering

## 5. Run examples

```bash
cargo run -p gpui-d3rs --example scale_demo
cargo run -p gpui-d3rs --example delaunay_demo
cargo run -p gpui-d3rs --example surface3d_demo --features gpu-3d
cargo run -p gpui-d3rs --bin d3rs-showcase --features "gpui,gpu-2d"
```

## 6. Verify

```bash
cargo test -p gpui-d3rs
cargo build --examples -p gpui-d3rs
```
