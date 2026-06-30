---
name: gpui-toolkit
description: Use when building or modifying GPUI applications in the gpui-toolkit workspace, especially when choosing reusable toolkit crates, composing UI, adding components, charts, themes, layouts, audio controls, mobile surfaces, or validation coverage. Prefer existing toolkit APIs over custom one-off implementations.
---

# GPUI Toolkit Skill

Use this skill for GPUI app and component work in this workspace. The prime directive is: build with the toolkit first. Do not recreate buttons, forms, charts, layout solvers, theme systems, keyboard handling, audio widgets, or component review surfaces that already exist here.

## Start Here

1. Before reading files, query TokenSave for the feature area: `tokensave_context` when available, otherwise `tokensave_search` / `tokensave_files`.
2. Read [README.md](README.md) for the crate map and [TUTORIAL.md](TUTORIAL.md) for the intended app composition flow.
3. For crate-specific rules, read the nearest `AGENTS.md` or `README.md` under `crates/<crate>/`.
4. Prefer examples, showcase includes, and component-lab stories over inventing API style from memory.
5. Use `rg` for textual searches after TokenSave when the semantic graph is not enough.

## Composition Defaults

For desktop apps, start with `gpui-miniapp`. It supplies the app shell, menus, theme globals, design-system globals, language switching, and platform backend setup.

Use `gpui-ui-kit` for ordinary app UI:

- Core: `Button`, `IconButton`, `Card`, `Dialog`, `ConfirmDialog`, `Popover`, `Tabs`, `Menu`, `ContextMenu`, `Toast`.
- Forms: `Input`, `NumberInput`, `Checkbox`, `Toggle`, `Select`, `ButtonSet`, `ColorPickerView`, `Slider`, `Wizard`.
- Data and feedback: `Table`, `Badge`, `Avatar`, `Progress`, `Spinner`, `QrCode`, `KeyboardShortcutLabel`, `EmptyState`, `Alert`, `SearchBar`, `Tooltip`.
- Layout and navigation: `VStack`, `HStack`, `Spacer`, `Divider`, `PaneDivider`, `SplitPane`, `Sidebar`, `StatusBar`, `Accordion`, `Breadcrumbs`.
- Advanced surfaces: `CommandPalette`, `Toolbar`, `TreeView`, `DragList`, `WorkflowCanvas`.

Use `gpui-audio-kit` for audio and plugin UIs: `Potentiometer`, `VerticalSlider`, `VolumeKnob`, `AudioDesignTokens`, meters, spectrum elements, tick rows, and audio scale helpers. `gpui-ui-kit` intentionally does not re-export these.

Use `gpui-builder` when layout behavior needs a reusable or testable contract: responsive shells, split panes, collapsible panels, auto-axis behavior, display tiers, or draggable divider math. Keep the solver tree pure and render the solved result in GPUI.

Use `gpui-px` for high-level charts: scatter, line, bar, heatmap, contour, isoline, pie/donut, boxplot, treemap, and surface-style chart demos. Drop to `gpui-d3rs` only when you need lower-level D3-style primitives such as custom scales, shapes, axes, grids, Delaunay/Voronoi, force layouts, geo, brush/zoom, or GPU rendering internals.

Use `gpui-design` for spacing, radius, typography, touch targets, motion, elevation, and platform design language. Use `gpui-themes` or `gpui-ui-kit::theme` for colors and theme state. Keep these layers separate: design system is geometry/behavior; theme is color.

Use `gpui-keybinding` for shortcuts, presets, conflict detection, command-palette data, and which-key hints. Do not hand-roll keybinding registries for apps.

Use `gpui-component-lab` for component stories, responsive preview matrices, design conformance, and visual review surfaces. Any new shared component should get story/conformance coverage.

Use `gpui-pretext` for text measurement and multiline layout. Use `gpui-profiler` when diagnosing interactive UI allocation hot paths. Use `gpui-python-runtime` only for Python-facing retained scene demos or wrappers.

## GPUI Rendering Patterns

GPUI code uses native `div()`-based rendering, not HTML or SVG. Components should return `impl IntoElement` and compose with `.child()` / `.children(...)`.

Typical render shape:

```rust
use gpui::*;
use gpui_ui_kit::{Button, ButtonVariant, Heading, Text};
use gpui_ui_kit::theme::ThemeExt;

div()
    .p_4()
    .bg(cx.theme().background)
    .child(Heading::new("Dashboard"))
    .child(Text::new("Composed from toolkit components."))
    .child(Button::new("refresh", "Refresh").variant(ButtonVariant::Primary))
```

Use `cx.theme()` for colors and `cx.design()` for platform-adaptive sizing/motion:

```rust
use gpui_design::DesignExt;
use gpui_ui_kit::theme::ThemeExt;

let theme = cx.theme();
let ds = cx.design();

div()
    .p(px(ds.spacing.card_padding))
    .rounded(px(ds.corners.md))
    .bg(theme.surface)
    .text_color(theme.text)
```

Attach mouse and keyboard behavior through GPUI event handlers. Before adding bespoke logic, inspect existing controls such as `slider`, `input`, `menu`, `workflow/canvas`, `Potentiometer`, and `VerticalSlider`.

Use `FocusGroup` / `FocusGroupExt` for focus navigation. Use accessibility APIs from `gpui-ui-kit::accessibility` so controls register ARIA role, label, state, and live-region metadata.

## Adding Shared Components

Only add a new `gpui-ui-kit` component when an existing component cannot be extended cleanly.

Follow the local conventions:

- Component type with builder-style setters returning `Self`.
- Variant and size enums with `Clone`, `Copy`, `Debug`, `Default`, `PartialEq`, `Eq` when practical.
- Event handlers stored as `Option<Box<dyn Fn(...) + 'static>>`.
- `RenderOnce` / `IntoElement` implementation using `cx.theme()` and `cx.design()` as appropriate.
- Accessibility fields and builder methods: `aria_label`, `aria_role`, plus `cx.register_accessible(...)` in render.
- Re-export from `crates/gpui-ui-kit/src/lib.rs`.
- Tests under `crates/gpui-ui-kit/tests/components` and integration coverage when behavior matters.
- Example or showcase include when a human should visually inspect the component.
- Component-lab story for relevant states: default, hover/focus, disabled, selected, dense/narrow, high contrast, and touch-safe layouts.

When changing i18n-visible UI-kit sections, update all language tables in `i18n.rs`; missing languages are treated as test failures.

## Layout Guidance

Prefer fixed + flex + fractional constraints in `gpui-builder` over ad hoc width math in render code. Use:

- `Sizing::Fixed(px)` for toolbars, headers, footers, and known chrome.
- `Sizing::fractional(ratio, min)` for panes with a preferred proportion.
- `Sizing::flex(min)` for content that should absorb remaining space.
- Collapsible slots with explicit labels and priorities for responsive degradation.
- `auto_axis(...)` when a shell should flip horizontal/vertical by aspect ratio.
- `SolvedNode::debug_report_with_source(...)` while iterating on complex layouts.

Keep solved layout snapshots stable when behavior is shared across apps.

## Charts And Data Viz

Start with `gpui-px` builders for app charts. They validate data and keep visual API high-level.

Drop to `gpui-d3rs` for custom marks, scales, axes, geometry, interaction primitives, color interpolation, or GPU-specific work. Do not manually implement chart scales, tick generation, color ramps, or Delaunay/spatial logic without checking `gpui-d3rs` first.

For chart UI changes, add or update component-lab stories and PX conformance inventory when the chart surface is first-party.

## Validation

Use the narrowest useful command first:

```bash
cargo check -p <crate>
cargo test -p <crate>
cargo build --examples -p <crate>
```

For broader GPUI confidence:

```bash
just examples
just qa-gpui-conformance
just qa-gpui-obvious
```

For demos and visual inspection:

```bash
just demo-ui-kit
just demo-builder
just demo-component-lab
just demo-px
just demo-d3rs
just demo-audio-kit
just demo-themes
```

For mobile surfaces, use the iOS/tvOS recipes in `Justfile` and read `crates/gpui-ios/AGENTS.md` or `crates/gpui-showcase/ios/TUTORIAL.md` first.

## Avoid Reinventing

Do not create one-off replacements for:

- Theme or design globals.
- Basic controls, forms, menus, modals, tabs, toasts, toolbars, sidebars, tables, or command palettes.
- Keyboard shortcut registries and shortcut display labels.
- Responsive split layout solving.
- Charting scales, axes, color ramps, or common plot types.
- Audio knobs, sliders, meters, spectrum displays, and tick marks.
- Component preview/conformance tooling.
- Text measurement and multiline layout.

If a toolkit API almost fits, extend it in the owning crate with tests and stories rather than duplicating it in an app.
