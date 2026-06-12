# gpui-px Tutorial

`gpui-px` is a high-level charting API built on `gpui-d3rs`.

## 1. Add the crate

```toml
[dependencies]
gpui-px = { workspace = true, features = ["gpui"] }
```

Use `gpu-3d` for 3D surface charts.

## 2. Create a chart

```rust
use gpui_px::{scatter, line, bar, ScaleType};

let x = vec![1.0, 10.0, 100.0, 1000.0];
let y = vec![2.0, 8.0, 32.0, 128.0];

let chart = scatter(&x, &y)
    .title("Latency")
    .x_scale(ScaleType::Log)
    .build()?;
```

## 3. Pick the right constructor

- `scatter(&x, &y)` for point clouds
- `line(&x, &y)` for trends
- `bar(&categories, &values)` for categories
- `heatmap(...)` for matrix data
- `contour(...)` and `isoline(...)` for scalar fields
- `pie(...)` and `donut(...)` for proportions
- `boxplot(...)` for distributions
- `treemap(...)` for hierarchy
- `surface3d(...)` for 3D surfaces

## 4. Render in a MiniApp

Look at the examples for full GPUI render integration:

```bash
cargo run -p gpui-px --bin px-showcase --features gpui
cargo run -p gpui-px --example logscale_demo --features gpui
```

## 5. Verify

```bash
cargo test -p gpui-px
cargo test -p gpui-px --features gpu-3d
cargo build --examples -p gpui-px
```
