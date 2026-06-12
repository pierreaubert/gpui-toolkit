# GPUI Toolkit Tutorial

This tutorial builds a small dashboard app step by step with the main toolkit
crates:

- `gpui-miniapp` for the application shell
- `gpui-ui-kit` for components
- `gpui-themes` and `gpui-design` for theme and design-system behavior
- `gpui-keybinding` for documented shortcut sets
- `gpui-builder` for responsive layout solving
- `gpui-design-tools` for token export and validation
- `crates/figma/` assets for design handoff
- `gpui-component-lab` for component conformance
- `gpui-px` and `gpui-d3rs` for graphs
- `gpui-python-runtime` for retained scene demos

The code snippets are intentionally small. For complete runnable examples, use
the workspace recipes:

```bash
just demo
just examples
just qa-gpui-obvious
```

## 1. Create a Workspace App

Inside this workspace, add a new app crate or use one of the examples as a
starting point. The minimum dependencies for a desktop app are:

```toml
[dependencies]
gpui = { workspace = true }
gpui-miniapp = { workspace = true }
gpui-ui-kit = { workspace = true }
gpui-design = { workspace = true, features = ["gpui"] }
gpui-themes = { workspace = true }
gpui-keybinding = { workspace = true }
gpui-builder = { workspace = true, features = ["showcase"] }
gpui-px = { workspace = true, features = ["gpui"] }
gpui-d3rs = { workspace = true, features = ["gpui", "gpu-2d"] }
```

## 2. Start with MiniApp

`gpui-miniapp` creates a small GPUI application with menus, theme globals,
design-system globals, optional i18n, and the right platform backend.

```rust
use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::{Button, ButtonVariant, Heading, Text};
use gpui_ui_kit::theme::ThemeExt;

struct Dashboard;

impl Dashboard {
    fn new(_: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for Dashboard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .p_4()
            .bg(theme.background)
            .text_color(theme.text)
            .child(Heading::new("GPUI Toolkit Dashboard"))
            .child(Text::new("A small app composed from toolkit crates."))
            .child(Button::new("refresh", "Refresh").variant(ButtonVariant::Primary))
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Toolkit Dashboard")
            .size(1200.0, 780.0)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(Dashboard::new),
    );
}
```

Run a MiniApp-based example:

```bash
cargo run -p gpui-ui-kit --example showcase
```

## 3. Add Theme Controls

`gpui-miniapp` installs theme state when `.with_theme(true)` is enabled.
Components can read it through `ThemeExt`.

```rust
use gpui_ui_kit::theme::ThemeExt;

let theme = cx.theme();
div()
    .bg(theme.surface)
    .border_color(theme.border)
    .text_color(theme.text)
```

For a full theme editor:

```bash
cargo run -p gpui-themes --bin theme-editor
cargo run -p gpui-themes --bin theme-showcase
```

## 4. Use the Design System

`gpui-design` stores platform-adaptive rules for spacing, typography, corners,
elevation, interaction, and animation. `MiniApp` installs
`DesignSystemState::new()` automatically.

Use design values when writing shared UI surfaces:

```rust
use gpui_design::DesignExt;

let ds = cx.design();
div()
    .gap(ds.spacing.md)
    .rounded(ds.corners.card)
```

Validate design tokens:

```bash
cargo run -p gpui-design-tools --bin gpui-validate-design-tokens
```

Export tokens for external tools:

```bash
cargo run -p gpui-design-tools --bin gpui-export-design-tokens -- \
  --format style-dictionary-json \
  --output target/design-tokens.json
```

## 5. Bring in Figma Handoff

The [figma](./crates/figma/) directory contains design-system rules and Code
Connect mappings. A practical handoff loop is:

1. Keep Figma component names aligned with `gpui-ui-kit` component names.
2. Export or review tokens with `gpui-design-tools`.
3. Use [crates/figma/DESIGN_SYSTEM_RULES.md](./crates/figma/DESIGN_SYSTEM_RULES.md)
   to check naming, density, and token expectations.
4. Use [crates/figma/CODE_CONNECT_MAPPINGS.md](./crates/figma/CODE_CONNECT_MAPPINGS.md)
   to map Figma components to GPUI component constructors.
5. Run `just qa-gpui-conformance` before accepting a design-system change.

## 6. Designer Workflow: Figma to Component Lab

Use this workflow when a designer changes a component, token, or responsive
behavior:

1. Start in Figma and name the component, variants, and slots after the closest
   `gpui-ui-kit` component.
2. Check the mapping in
   [CODE_CONNECT_MAPPINGS.md](./crates/figma/CODE_CONNECT_MAPPINGS.md).
3. Export or validate design tokens with `gpui-design-tools`.
4. Implement the GPUI component or prop change.
5. Add a story in `gpui-component-lab` for every important state: default,
   hover, disabled, selected, dense, narrow viewport, and high-contrast theme.
6. Run `just qa-gpui-conformance`.
7. Attach the generated markdown report to the design review.

## 7. Add Keybindings

`gpui-keybinding` lets each feature provide a set of bindings per preset, plus
human-readable documentation for help or command-palette UI.

```rust
use gpui::{actions, KeyBinding};
use gpui_keybinding::{
    DocumentedKeybinding, KeybindingCategory, KeybindingProvider,
    KeybindingRegistry, KeymapPreset,
};

actions!(dashboard, [RefreshDashboard]);

struct DashboardBindings;

impl KeybindingProvider for DashboardBindings {
    fn bindings(&self, preset: KeymapPreset) -> Vec<KeyBinding> {
        match preset {
            KeymapPreset::Default | KeymapPreset::VSCode => {
                vec![KeyBinding::new("secondary-r", RefreshDashboard, None)]
            }
            KeymapPreset::Vim => vec![KeyBinding::new("r", RefreshDashboard, None)],
            KeymapPreset::Emacs => vec![KeyBinding::new("ctrl-r", RefreshDashboard, None)],
        }
    }

    fn documented_bindings(&self, _: KeymapPreset) -> Vec<DocumentedKeybinding> {
        vec![
            DocumentedKeybinding::new(
                "Refresh",
                "Refresh dashboard data",
                KeybindingCategory::View,
            )
            .with_raw_key_spec("refresh"),
        ]
    }
}

let mut registry = KeybindingRegistry::new();
registry.register(DashboardBindings);
let conflicts = registry.detect_conflicts(KeymapPreset::Default);
```

Bind the returned `KeyBinding` values during app initialization, then handle the
action with `cx.on_action::<RefreshDashboard>(...)`.

## 8. Solve Responsive Layouts

Use `gpui-builder` when you need a layout contract that can be tested outside
the visual component.

```rust
use gpui_builder::{Axis, ContainerNode, LayoutPreferences, Sizing, SlotNode, solve};

let children = [
    SlotNode::new("sidebar", Sizing::Fixed(260.0)).into_node(),
    SlotNode::new("content", Sizing::flex(480.0)).into_node(),
];

let root = ContainerNode::new(
    "root",
    Axis::Horizontal,
    Sizing::flex(0.0),
    &children,
)
.into_node();
let solved = solve(&root, 1200.0, 780.0, &LayoutPreferences::default());
```

Explore the solver:

```bash
cargo run -p gpui-builder --bin layout-showcase --features showcase
cargo run -p gpui-builder --example responsive_dashboard
```

## 9. Add Graphs with gpui-px

Use `gpui-px` for high-level charts.

```rust
use gpui_px::{bar, line, scatter, ScaleType};

let x = vec![1.0, 10.0, 100.0, 1000.0];
let y = vec![2.0, 8.0, 32.0, 128.0];

let scatter_chart = scatter(&x, &y)
    .title("Latency by load")
    .x_scale(ScaleType::Log)
    .build()?;

let line_chart = line(&x, &y)
    .title("Trend")
    .build()?;

let bars = bar(&["A", "B", "C"], &[4.0, 7.0, 3.0])
    .title("Buckets")
    .build()?;
```

Run graph demos:

```bash
cargo run -p gpui-px --bin px-showcase --features gpui
cargo run -p gpui-px --example logscale_demo --features gpui
```

## 10. Drop Down to d3rs for Custom Visuals

Use `gpui-d3rs` for D3-style primitives such as scales, shapes, color,
hierarchy, force layout, geo projections, Delaunay/Voronoi, and custom GPU
rendering.

```rust
use d3rs::delaunay::Delaunay;
use d3rs::scale::LinearScale;

let scale = LinearScale::new()
    .domain([0.0, 100.0])
    .range([0.0, 600.0]);

let points = vec![(0.0, 0.0), (1.0, 0.0), (0.4, 0.8)];
let mesh = Delaunay::new(&points);
let nearest = mesh.find(0.5, 0.5, None);
```

Run:

```bash
cargo run -p gpui-d3rs --bin d3rs-showcase --features "gpui,gpu-2d"
cargo run -p gpui-d3rs --example delaunay_demo
```

## 11. Register Stories in Component Lab

`gpui-component-lab` is the review surface for component states, responsive
viewports, theme presets, and design-token conformance.

Start with the built-in stories:

```bash
cargo run -p gpui-component-lab --bin gpui-component-lab
```

Generate conformance reports:

```bash
mkdir -p target/gpui-conformance
cargo run -p gpui-component-lab --bin gpui-component-lab -- \
  --conformance \
  --report-json target/gpui-conformance/component-lab.json \
  --report-markdown target/gpui-conformance/component-lab.md
```

When adding a component:

1. Add the component to `gpui-ui-kit`.
2. Add focused examples under `crates/gpui-ui-kit/examples/`.
3. Add or extend component-lab stories.
4. Run `just qa-gpui-conformance`.
5. Check the markdown report before merging visual changes.

## 12. Python Runtime Demo

`gpui-python-runtime` keeps a retained scene description that can be adapted to
GPUI. Its showcase combines UI-kit components and `gpui-px` charts.

Run:

```bash
cargo run -p gpui-python-runtime --bin gpui-python-showcase --features showcase
```

Use it as a reference when building a Python-facing layer:

1. Represent UI as `PythonAppIr`.
2. Represent 3D or chart scene state with `Scene3D` types.
3. Cache retained resources in `RetainedSceneCache`.
4. Render the scene through the GPUI adapter feature.
5. Keep the Rust side responsible for layout, themes, and chart rendering.

## 13. Mobile Extension

The same component library can be shown on iOS through the bundled showcase:

```bash
just ios-sim
```

Build the tvOS Rust library artifacts:

```bash
just tvos-sim
```

See [crates/gpui-ui-kit/ios/TUTORIAL.md](./crates/gpui-ui-kit/ios/TUTORIAL.md)
for mobile-specific steps.

## 14. Development Loop

Use this loop for most feature work:

1. Prototype in a MiniApp example.
2. Extract stable UI into `gpui-ui-kit`, `gpui-audio-kit`, or `gpui-px`.
3. Add responsive layout constraints in `gpui-builder` if the layout is shared.
4. Add design tokens or conformance expectations in `gpui-design`.
5. Add component-lab coverage.
6. Run `just examples`, then `just qa-gpui-obvious`.
7. Use iOS/tvOS recipes only after desktop checks are green.
