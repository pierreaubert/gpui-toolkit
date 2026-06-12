# gpui-toolkit

Libraries, examples, and tooling for building GPUI applications with native
`div()`-based rendering, reusable components, charting primitives, design
tokens, and Apple mobile backends.

[![License](https://img.shields.io/crates/l/gpui-ui-kit)](LICENSE)

## Workspace Crates

| Crate | Purpose |
| --- | --- |
| [gpui-au](./crates/gpui-au/) | macOS Audio Unit platform backend for embedding GPUI rendering inside AUv3 view controllers. |
| [gpui-audio-kit](./crates/gpui-audio-kit/) | Audio-focused UI controls for plugin and playback interfaces, including knobs and vertical sliders. |
| [gpui-builder](./crates/gpui-builder/) | Constraint-based layout solver with responsive display tiers, dividers, auto-axis behavior, and showcase examples. |
| [gpui-component-lab](./crates/gpui-component-lab/) | Prop-driven component lab and responsive preview matrix for design-system conformance work. |
| [gpui-d3rs](./crates/gpui-d3rs/) | D3.js-inspired visualization primitives: scales, shapes, colors, geo projections, force layouts, Delaunay/Voronoi, GPU 2D, and GPU 3D. |
| [gpui-design](./crates/gpui-design/) | Platform-adaptive design system tokens for spacing, corners, typography, animation, and GPUI integration. |
| [gpui-design-tools](./crates/gpui-design-tools/) | CLI tooling for exporting, importing, and validating design tokens and conformance reports. |
| [gpui-ios](./crates/gpui-ios/) | iOS/tvOS platform backend for GPUI with Metal rendering, touch input, text input, accessibility, platform views, and hot reload hooks. |
| [gpui-keybinding](./crates/gpui-keybinding/) | Reusable keybinding framework with editor-style preset support for GPUI applications. |
| [gpui-miniapp](./crates/gpui-miniapp/) | Small application shell used by examples and showcases to select the right GPUI platform backend. |
| [gpui-pretext](./crates/gpui-pretext/) | High-performance text measurement and multiline layout utilities. |
| [gpui-px](./crates/gpui-px/) | Plotly Express-style charting API built on `gpui-d3rs` for scatter, line, bar, heatmap, contour, surface, pie, boxplot, and treemap views. |
| [gpui-python-runtime](./crates/gpui-python-runtime/) | Retained scene specification runtime for a GPUI Python wrapper, with an optional showcase. |
| [gpui-scaffolder](./crates/gpui-scaffolder/) | CLI for creating standalone GPUI mini-app projects backed by `gpui-miniapp`. |
| [gpui-themes](./crates/gpui-themes/) | Theme editor and theme showcase infrastructure for GPUI applications. |
| [gpui-ui-kit](./crates/gpui-ui-kit/) | Reusable UI component library: buttons, inputs, dialogs, menus, tabs, tables, QR, command palette, sidebar, wizard, workflow canvas, and more. |
| [gpui-ui-kit-macros](./crates/gpui-ui-kit-macros/) | Procedural macros used by `gpui-ui-kit`, including builder and theme derivation helpers. |
| [gpui-ui-kit-ios-showcase](./crates/gpui-ui-kit/ios/) | Static library and Swift host project for showing `gpui-ui-kit` on iOS, with tvOS Rust library build support. |

## Related Assets

| Path | Purpose |
| --- | --- |
| [figma/](./crates/figma/) | Figma-to-GPUI design-system rules and Code Connect mappings. |
| [MIGRATION.md](./MIGRATION.md) | Migration notes for moving toolkit code out of the larger SOTF workspace. |
| [AGENTS.md](./AGENTS.md) | Short working guide for agents and contributors. |

## GPUI Version

This workspace is currently on the `0.7.x` GPUI toolkit line and pins GPUI
packages to Zed `v1.0.0` through the root [Cargo.toml](./Cargo.toml).

The workspace uses local path dependencies for toolkit crates and git
dependencies for GPUI platform crates from `zed-industries/zed`.

## Common Commands

```bash
# List tasks
just --list

# Check the workspace
just check

# Build all showcase-style demo targets
just demo

# Build maintained examples by crate family
just examples

# Run focused QA
just qa-gpui-obvious
```

If the pinned Rust toolchain is unavailable locally, use an installed toolchain
explicitly:

```bash
cargo +stable check --workspace --all-targets
```

## Scaffolding Mini Apps

Use `gpui-scaffolder` to generate a small standalone GPUI app directory:

```bash
cargo run -p gpui-scaffolder -- my-app
cd my-app
cargo run
```

Generated projects also include a `just run` recipe:

```bash
just run
```

To create the app somewhere else, pass `--output-dir`:

```bash
cargo run -p gpui-scaffolder -- my-app --output-dir /tmp
```

## Demos

The aggregate `just demo` builds:

- `gpui-audio-kit` examples
- `gpui-builder` layout showcase
- `gpui-component-lab`
- `gpui-d3rs` showcase and spinorama demo
- `gpui-px` showcase and spinorama demo
- `gpui-python-runtime` showcase
- `gpui-themes` showcase
- `gpui-ui-kit` showcase

Individual recipes are available as `just demo-ui-kit`, `just demo-d3rs`,
`just demo-px`, `just demo-builder`, `just demo-component-lab`,
`just demo-audio-kit`, `just demo-python`, and `just demo-themes`.

## Examples

The aggregate `just examples` builds maintained example families:

```bash
just examples-audio-kit
just examples-builder
just examples-d3rs
just examples-px
just examples-ui-kit
```

For the QR camera example:

```bash
just run-qr-debug
```

On macOS this recipe creates a small `.app` bundle so the camera permission
prompt has the right `Info.plist` metadata.

## iOS

The iOS showcase lives in [crates/gpui-ui-kit/ios](./crates/gpui-ui-kit/ios/).
It builds the Rust static library and links it into the bundled Swift
`GPUIShowcase.xcodeproj`.

```bash
# Rust static libraries
just ios-rust-sim
just ios-rust-device

# Copy static libraries into the Xcode project
just ios-build-rust-sim
just ios-build-rust-device

# Generate/update the Xcode project with XcodeGen
just ios-xcodegen

# Build the Swift host app
just ios-sim
just ios-device

# Build simulator hot-reload dylib and manifest
just ios-hot-reload
```

The `showcase-*` recipe names from the old SOTF workspace are also available,
for example `just showcase-build-sim`.

## tvOS

tvOS is a Tier 3 Rust target, so the tvOS recipes use nightly with
`-Zbuild-std`. Install the prerequisites first:

```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

Then build the showcase Rust static library for tvOS:

```bash
just tvos-rust-sim
just tvos-rust-device
just tvos-build-rust-sim
just tvos-build-rust-device
just tvos-sim
just tvos-device
```

This repo currently ships an iOS Swift host project. The tvOS recipes produce
the Rust library artifacts and copy them next to the mobile showcase assets.

## Quick Example

```rust
use gpui::*;
use gpui_ui_kit::{Button, ButtonVariant};
use gpui_px::scatter;

let button = Button::new("submit", "Submit")
    .variant(ButtonVariant::Primary)
    .on_click(|_, _| println!("Clicked"));

let chart = scatter(&x_data, &y_data)
    .title("My Data")
    .build()?;
```

## License

[ISC License](LICENSE)
