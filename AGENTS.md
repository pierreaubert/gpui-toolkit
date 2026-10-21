# GPUI Toolkit - Agent's Guide

Quick reference for AI agents working with the GPUI toolkit libraries.

## Overview

The `gpui-toolkit` workspace contains several related crates for building GPUI applications:

| Crate | Purpose | Documentation |
|-------|---------|---------------|
| `gpui-au` | macOS Audio Unit platform backend — embeds GPUI inside AUv3 ViewControllers via Metal/wgpu | [lib.rs](gpui-au/src/lib.rs) |
| `gpui-hello-web` | Minimal wasm/browser spike app (gpui_web + gpui_wgpu canvas) | this repo, `just wasm-serve-hello` |
| `gpui-showcase` | Component showcase with a wasm/browser entry point (gpui_web + gpui_wgpu canvas) | `just wasm-serve-showcase` |
| `gpui-px` (px-showcase) | Chart showcase with a wasm/browser entry point | `just wasm-serve-px` |
| `gpui-builder` | Generic constraint-based layout solver — priority collapse, auto-axis, display tiers, dividers | [README](gpui-builder/README.md) |
| `gpui-d3rs` | Low-level D3.js-inspired visualization primitives, incl. vello-backed 2D rasterization (`vello2d` module, zero-copy via `WgpuCustomDraw`, vello_cpu fallback); `d3rs-showcase` example doubles as the wasm demo (route `/d3rs/`) | [README](gpui-d3rs/README.md) |
| `gpui-design` | Platform-adaptive design system (Apple HIG, Material 3, Fluent, Neutral) — spacing, corners, typography, animation | [README](gpui-design/README.md) |
| `gpui-foreign-parent` | Platform-neutral core for foreign-parent editor embedding (registry, frame throttle, resize debounce, callback slots, FFI helpers) | [contract](docs/foreign-parent-editor-embedding.md) |
| `gpui-web-foreign` | Browser binding: page-owned canvas host shell (observers, rAF pump, listeners) plus DOM input tables; renderer provider needs the §16 fork change | [contract](docs/foreign-parent-editor-embedding.md) |
| `gpui-win-foreign` | Windows binding: GPUI editors in host-owned HWNDs (VST3); WS_CHILD creation hook in the gpui_windows fork | [contract](docs/foreign-parent-editor-embedding.md) |
| `gpui-linux-foreign` | Linux binding: X11 child windows (VST3/CLAP-x11) and Wayland subsurfaces (CLAP-wayland); creation hooks in the gpui_linux fork | [contract](docs/foreign-parent-editor-embedding.md) |
| `gpui-ios` | iOS platform backend (Metal rendering, touch, text) | [README](gpui-ios/README.md), [AGENTS.md](gpui-ios/AGENTS.md) |
| `gpui-pretext` | High-performance text measurement and multiline layout (Rust port of chenglou/pretext) | [README](gpui-pretext/README.md) |
| `gpui-profiler` | Lightweight allocation profiling utilities for interactive UI hot paths | [README](gpui-profiler/README.md) |
| `gpui-px` | High-level Plotly Express-style charting API, including unstructured `MeshPlot` | [README](gpui-px/README.md) |
| `gpui-mesh` | Mesh data model only: VTK-style cells (tet/hex/wedge/pyramid), scalar/vector/tensor/complex fields, boundary extraction, bounded surface chunking for large datasets; no GPUI/wgpu/IO, wasm-safe | [README](crates/gpui-mesh/README.md) |
| `gpui-dataset-transformation` | Pure `DataSet -> DataSet` filters: plane slice/clip, threshold, marching-tet isosurface, calculators, flow diagnostics (vorticity/Q/divergence, optional wgpu compute via `gpu` feature), warp, glyphs, streamlines, probe, VTK/VTU/XDMF I/O; wasm-safe by default | [README](crates/gpui-dataset-transformation/README.md) |
| `gpui-scene3d` | Scene rendering only: orbit camera, transfer functions, WGSL clipping-plane shaders, chunked GPU-ready buffers, software rasterizer (clipping + sorted transparency), volume raycasting, time playback, PNG sequences and MP4 muxing (`mp4` feature, native ffmpeg), optional GPUI element (`gpui` feature); wasm-safe by default; mesh browser demo via `just wasm-serve-mesh` | [README](crates/gpui-scene3d/README.md) |
| `gpui-spinorama` | Spinorama-style speaker measurement showcase (CEA-2034 + dispersion, wasm route `/spinorama/`) | this repo, `just wasm-serve-spinorama` |
| `gpui-themes` | Theme editor and management infrastructure | [AGENTS.md](gpui-themes/AGENTS.md) |
| `gpui-ui-kit` | Reusable UI components (buttons, forms, layout) with ARIA accessibility support | [lib.rs](gpui-ui-kit/src/lib.rs), [AGENTS.md](crates/gpui-ui-kit/AGENTS.md) |
| `gpui-ui-kit-macros` | Procedural macros for theme derivation and the `layout!` DSL | [README](gpui-ui-kit-macros/README.md) |
| `gpui-toolkit-cli` | Unified `gpui-toolkit` binary: catalog, templates, themes, doctor, upgrade, layout tools | [README](README.md#toolkit-cli), [TUTORIAL](TUTORIAL.md) §15 |
| `gpui-layout-expr` | Layout-expression AST, parser, validation, custom aliases (no GPUI deps) | [layout spec](docs/superpowers/2026-10-07-layout-expressions.md) |
| `figma/` | Figma-to-GPUI design system rules and Code Connect mappings | [DESIGN_SYSTEM_RULES.md](figma/DESIGN_SYSTEM_RULES.md), [CODE_CONNECT_MAPPINGS.md](figma/CODE_CONNECT_MAPPINGS.md) |

**Key Principle**: All crates use GPUI's native `div()`-based rendering, not HTML/SVG. Components return `impl IntoElement`.

**MeshPlot**: `gpui-px::mesh_plot` owns the validated triangle-mesh builder,
scalar-field modes, contour/isoline export, equal-aspect views, and
accessibility metadata. Geometry preparation, BVH picking, GPU upload state,
and compute/reference kernels belong to `gpui-d3rs`; Python declarations and
revisioned patches belong to `gpui-python-runtime`. Keep application-specific
complex/vector/tensor projection and durable result storage outside these UI
crates.

## Building an app for a user

Follow the [first-app tutorial](TUTORIAL.md) and its
[agent workflow](TUTORIAL.md#agent-workflow) for Rust and Python projects.
`gpui-scaffolder` generates Rust applications; `gpui-toolkit init` writes a
catalog into an existing project's agent documentation. Check the generated
GPUI dependency against this workspace before building. Deliver the project,
an exact launch command, and artifact paths; distinguish native-window
verification from Python IR validation and name missing platform prerequisites.

## Toolkit CLI (for agents)

Prefer the `gpui-toolkit` binary over reading sources when you need
catalog facts: run `cargo run -p gpui-toolkit-cli -- manifest --json`
for the full command surface (response types, exit codes, examples),
then query with `--json` and branch on `type` / `ERR_*` codes, never
on prose. Useful calls: `component <id> --props`, `search <query>`,
`template <id> --skeleton`, `doctor` (CI gate, exits 1 on failure),
`layout check '<expr>'`, and `upgrade --detect --dir <src>`.
Project overrides live in `toolkit.toml` (custom layout components,
tracker URL, extra upgrade rules). Before changing CLI behavior, read
the [API conventions](docs/superpowers/2026-10-07-api-conventions.md)
and keep envelopes, codes, and the manifest drift test in sync.

## Forms

Form components are in `gpui-ui-kit`. The main form modules are:

- `input.rs` - Text input with validation
- `number_input.rs` - Numeric input with step controls
- `select.rs` - Dropdown selection
- `slider.rs` - Range slider
- `checkbox.rs` - Boolean checkbox
- `toggle.rs` - Switch toggle
- `color_picker/` - Color selection

**Pattern for forms:**

```rust
use gpui_ui_kit::{Input, Checkbox, Slider};

// Each component takes a GPUI context and returns an element
div()
    .child(Input::new(cx).placeholder("Enter text..."))
    .child(Slider::new(cx).min(0.0).max(100.0))
```

**State management:** Form components typically use `global_mut(cx)` for state persistence across renders.

**Reference:** See `gpui-ui-kit/src/lib.rs` lines 36-45 for the complete form module list.

## Keyboard Management

Keyboard handling in GPUI uses event handlers attached to elements:

```rust
use gpui::{KeyDownEvent, KeyUpEvent};

my_element
    .on_key_down(|event: &KeyDownEvent, _window, cx| {
        match event.keystroke.key.as_str() {
            "enter" => { /* handle enter */ }
            "escape" => { /* handle escape */ }
            _ => {}
        }
    })
```

**Focus management:** Use the `focus.rs` module in `gpui-ui-kit`:

```rust
use gpui_ui_kit::{FocusGroup, FocusDirection};

// Tab order and arrow key navigation
FocusGroup::new(cx).add(child1).add(child2)
```

**Key modules with keyboard support:**
- `menu.rs` - Menu keyboard navigation (arrows, enter, escape)
- `input.rs` - Text editing keys
- `select.rs` - Dropdown keyboard controls
- `workflow/canvas.rs` - Canvas shortcuts

## Mouse Management

Mouse events attach directly to elements:

```rust
use gpui::{MouseDownEvent, MouseMoveEvent, ClickEvent};

my_element
    .on_mouse_down(|event: &MouseDownEvent, _window, cx| {
        let button = event.button;  // Left, Right, Middle
    })
    .on_click(|event: &ClickEvent, _window, cx| {
        let clicks = event.click_count();  // For double-click detection
    })
    .on_scroll_wheel(|event: &ScrollWheelEvent, _window, cx| {
        // Handle scroll
    })
    .on_hover(|hovered, _window, cx| {
        // hovered is bool
    })
```

**Drag patterns:** Store drag state in the component's model, update on `on_mouse_move`:

```rust
// In your component
.on_mouse_down(|event, window, cx| {
    self.drag_start = Some(event.position);
})
.on_mouse_move(|event, window, cx| {
    if let Some(start) = self.drag_start {
        let delta = event.position - start;
        // Apply delta
    }
})
```

**Reference implementations:**
- `slider.rs` - Drag to change value
- `audio/potentiometer.rs` - Rotary drag interaction
- `workflow/canvas.rs` - Node dragging and selection

## Common Patterns

**Building elements:** Always chain `.child()` or use `div().children(vec![])`

**Theming:** Access via `cx.global::<ThemeState>().active_theme(cx)`

**Sizing:** Use `gpui::px()` for pixel values, `gpui::rem()` for relative units

**Error handling:** Form validation returns `Result<(), ValidationError>`

## Testing

```bash
# Check a specific crate
cargo check -p gpui-ui-kit

# Run tests for a crate
cargo test -p gpui-px --features=gpui

# Build showcase (visual verification)
cargo run --bin gpui-px-showcase

# Check the wasm/web backend closure (nightly + build-std, injected by the recipe)
just wasm-check

# Serve apps in the browser (WebGPU-only; COOP/COEP headers set by Trunk)
just wasm-serve-hello      # http://127.0.0.1:8080
just wasm-serve-showcase   # http://127.0.0.1:8081
just wasm-serve-px         # http://127.0.0.1:8082
just wasm-serve-d3rs       # http://127.0.0.1:8083
just wasm-serve-spinorama  # http://127.0.0.1:8084

# Headless-Chrome smoke test + visual diff
just wasm-test
just wasm-visual           # positional args: `just wasm-visual hello 8080 gpui-hello-web [record] [click_x click_y]`
                           # e.g. chart section: `just wasm-visual px-scatter 8082 gpui-px '' 80 137`
```

The browser target is WebGPU-only (Chrome/Edge 113+, Firefox 141+ on Windows,
Safari 26+); design in
[docs/superpowers/2026-08-15-wasm-browser-target-design.md](docs/superpowers/2026-08-15-wasm-browser-target-design.md).
wasm builds need nightly (`-Z build-std`), env-injected by the `just` recipes
so the stable host toolchain is untouched. Visual QA baselines live at
`qa/visual/wasm/baselines/`. The px showcase honors `PX_SHOWCASE_SECTION`
(case-insensitive section label) for headless section selection.

gpu2d charts (`d3rs::gpu2d`, e.g. the px-showcase Heatmap/Contour/Isoline
sections) render on wasm: device init is async and pixel readback is deferred
via `map_async`, so painted content lags one frame behind interaction/resize.
The gpu3d / sphere_gallery renderers and the gpu-compute path
(`gpui-d3rs/src/mesh/gpu/compute.rs`) still assume blocking wgpu
(`device.poll`/`pollster`) and must not be exercised on wasm. Caveat: some
toolkit code paths still assume native time APIs.

## Quick Links

- [GPUI Documentation](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [D3.js Reference](https://d3js.org/) (for understanding gpui-d3rs concepts)
- Main project [GPUI.md](../../GPUI.md) for project-wide GPUI conventions
