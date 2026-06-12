# gpui-builder Tutorial

`gpui-builder` is a constraint-based layout solver for GPUI applications.

## 1. Add the crate

```toml
[dependencies]
gpui-builder = { workspace = true }
```

Enable the GPUI showcase binary when needed:

```bash
cargo run -p gpui-builder --bin layout-showcase --features showcase
```

## 2. Model a layout tree

```rust
use gpui_builder::{Axis, ContainerNode, LayoutPreferences, Sizing, SlotNode, solve};

let children = [
    SlotNode::new("sidebar", Sizing::Fixed(260.0)).into_node(),
    SlotNode::new("main", Sizing::flex(480.0)).into_node(),
];

let root = ContainerNode::new(
    "root",
    Axis::Horizontal,
    Sizing::flex(0.0),
    &children,
)
.into_node();
```

## 3. Solve for a viewport

```rust
let solved = solve(&root, 1200.0, 800.0, &LayoutPreferences::default());
```

Use solved node bounds to drive GPUI rendering or to validate layout behavior in
tests.

## 4. Test multiple viewports

```rust
use gpui_builder::{LayoutViewport, solve_snapshot_matrix};

let viewports = vec![
    LayoutViewport::new("desktop", 1440.0, 900.0),
    LayoutViewport::new("tablet", 900.0, 700.0),
    LayoutViewport::new("narrow", 420.0, 800.0),
];
let matrix = solve_snapshot_matrix(&root, &viewports, &LayoutPreferences::default());
```

## 5. Learn from examples

```bash
cargo run -p gpui-builder --example app_layout
cargo run -p gpui-builder --example responsive_dashboard
cargo run -p gpui-builder --example plugin_layout
```

## 6. Verify

```bash
cargo test -p gpui-builder
cargo check -p gpui-builder --features showcase
```
