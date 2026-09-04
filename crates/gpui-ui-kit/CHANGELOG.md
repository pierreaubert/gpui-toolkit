# Unreleased

## Added

- Added `date_picker` and `scroll_area` components; extended `dialog`,
  `drag_list`, `input`, `i18n`, and `split_pane`.
- Added a `validation` module (`ValidationError` for form validation).
- Added accessibility node diffing (`accessibility_node_key`,
  `diff_accessibility_nodes`, `AccessibilityTree::apply_snapshot`) so
  unchanged trees skip snapshot rebuilds and native bridge exports.

## Performance

- Added a bounded shared QR raster cache: one native-resolution raster per
  matrix/colors combination, scaled on the GPU at paint time.

## Refactored

- Split `TabsEntity::render` into a `tabs::tab_render` module (per-variant
  builders), `Table::build` into `table::table_build` section builders,
  `InputEntity::handle_key_down` into per-group handlers, and the workflow
  canvas `render` into snapshot/layer/menu helpers — no behavior change
  (disabled card-tab hover registration is now consistently skipped).

## Fixed

- Fixed the radio-group integration test `SharedString` selection type.

## 0.9.40 - 2026-08-24

### Fixes

- fixed #34 : Checkbox gets squeezed below its natural width under flex-column pressure, i.e. when box + label width > bounding box

## 0.9.39 - 2026-08-24

### Accessibility

- Added stable element-to-focus mapping with per-window focused-element queries, rerender cleanup, nested-focus handling, and duplicate-ID diagnostics.
- Added focused state to accessibility bridge snapshots and enabled the Button, Input, and Tabs components to report their rendered focus elements.

## 0.9.38 - 2026-08-23

### Security

- Added `Input::password(bool)`: password text and native accessibility values render as fixed masks while callbacks retain the raw value.
- Redacted password-mode diagnostics and edit-state history; password copy and cut are disabled while paste, IME, selection, undo, and redo remain supported.

### Performance

- Retained workflow-canvas snapshots and added allocation contracts for interactive UI-kit paths.

## 0.9.37

### Maintenance

- Removed the stale, unexported audio-control implementation and its orphaned
  examples/tests; those controls are owned by `gpui-audio-kit`.
- Removed unused `gpu-2d` and `gpu-3d` feature declarations.

## Features

- Added bounded QR constructors with input/matrix limits and cooperative
  cancellation checks.

- Added GPUI/AccessKit accessibility bridging for UI-kit metadata, including
  native roles, labels, states, values, and actions on core controls. Platform
  screen-reader validation remains pending in release QA.
- `Spinner` now renders an animated progress arc, `CircularProgress` paints
  track and value arcs, and `Avatar` renders its configured image source with
  initials as a fallback.

## Fixes

- `WithTooltip` now uses GPUI's native hoverable tooltip handling and delay,
  so tooltip timing no longer depends on parent-managed hover state.
- TreeView virtual rendering now counts expanded rows without flattening the
  entire tree and materializes only the requested visible window.

# 0.9.34

## Features

- Added real image rendering to `ImageView` for filesystem paths, remote URIs,
  and data URIs, with `ObjectFit` mapping and ARIA alternative-text labels.
- added Multi touch support on iOS and Android

## Fixes

- Added an image load-failure placeholder.
- Replaced fixed-width text estimates in the audio potentiometer and vertical
  slider with platform text measurement.
- **Sidebar**: made sidebar content participate in GPUI scroll handling with a
  tracked `ScrollHandle` and wheel callback support, so showcase menus can
  scroll consistently with touch drags and mouse-wheel input.

# 0.7.26

## Features

- Re-exported the `ComponentBuilder` derive plus the documented `FormField`
  compatibility alias, and exposed `gpui-builder` layout primitives through
  `layout_builder`.
- Added default `DesignSystem` resolution helpers and explicit `.design(...)`
  overrides for core UI-kit builders including buttons, stacks, split panes,
  toolbars, tables, sliders, checkboxes, icon buttons, and sidebar.
- Moved audio controls out to `gpui-audio-kit` as a hard API split. UI-kit no
  longer exports `audio`, `audio_design_tokens`, `scale`, `Potentiometer`,
  `VerticalSlider`, or `VolumeKnob`.

## Fixes

- Direct `build_with_theme(...)` paths now preserve explicit design overrides
  instead of always falling back to the neutral design.

# 0.6.24

## Improvements

- Made isolines less pixelized both in 2d and 3d, same with 3d axis and grid

# 0.6.23

## Fixes

- Fixed text alignement (regression) in d3rs and px

# 0.6.22

## Improvements

- **Wizard**: make horizontal step indicators responsive. At medium widths only
  the current step keeps its label; at very narrow widths the header collapses
  to the current step icon.

# 0.6.21

## Improvements

- **PaneDivider**: make separators more visible and professional with wider
  gutters, theme-controlled foreground/background/border colors, and accent
  tint rails for hover and active states.
- **PaneDivider**: rotate collapsed vertical labels instead of rendering one
  character per line.
- **Accordion**: add active tint and accent rails to headers, plus accent
  indicators for expanded sections.
- **Accordion**: make horizontal content span the full accordion width below
  the tab row.
- **Accordion**: rework side orientation into split vertical bars around the
  active content while preserving rotated labels.

# 0.6.20

## Fixes

- **VolumeKnob**: drag value is now computed from a delta against the
  position captured at `mouse_down` (via `store_drag_state` /
  `handle_drag` / `clear_drag_state`), instead of interpreting the raw
  window-space Y coordinate as knob-local progress. Drag now responds
  to mouse movement regardless of where the knob is on screen.

# 0.6.19

- Added more options to configure audio components

# 0.6.18

## Fixes

- **Workflow**: debug Bulk-connect

# 0.6.17

## Features

- **Workflow**: Bulk-connect gesture — Alt+drag from one node's body to another to wire all matching output→input ports in one go. The target node's input count grows automatically if allowed by `max_input_count`. Single undo reverts the entire operation.
- **Workflow**: `WorkflowNodeData` gains `max_input_count` / `max_output_count` (both `Option<usize>`, backward-compatible). Nodes can declare their maximum port capacity; `grow_inputs_to()` grows ports up to the limit and adjusts node height.
- **Workflow**: Port index validation in `add_connection()` — rejects out-of-bounds port indices.

## Fixes

- **Workflow**: Connection lines now route around intermediate nodes instead of passing behind them. The routing algorithm detects obstacle nodes in each connection's path and detours above or below with a 15px margin. Updates live during node drag.

# 0.6.16

## Features

### Accessibility (ARIA) support

GPUI has no native accessibility support. This release adds a UI-kit-level
accessibility layer so components carry semantic metadata (roles, labels,
states) that can be queried by tests, dev tools, and future screen reader
bridges.

**New module: `accessibility.rs`**

- `AriaRole` enum (30+ WAI-ARIA roles: Button, Checkbox, Switch, Slider, Combobox, Dialog, Alert, etc.)
- `AriaState` enum (Checked, Mixed, Pressed, Expanded, Selected, Disabled, Hidden)
- `AriaLive` enum (Off, Polite, Assertive)
- `AriaProps` struct with builder pattern for role, description, states, live regions, value ranges
- `AccessibilityNode` — element ID + label + props
- `AccessibilityTree` — GPUI Global that collects registrations per render frame
- `AccessibilityExt` trait on `App` — `register_accessible()` / `accessibility_tree()`

**Component integration (22 components)**

Every component gets `.aria_label()` and `.aria_role()` builder methods.
Components auto-register in the `AccessibilityTree` during `render()` with
sensible defaults:

| Component | Default Role | States |
|-----------|-------------|--------|
| Button, IconButton | Button | Disabled, Pressed |
| Checkbox | Checkbox | Checked/Mixed, Disabled |
| Toggle | Switch | Checked, Disabled |
| Input | Textbox | Disabled |
| NumberInput | Spinbutton | Disabled, value_range |
| Slider, Potentiometer, VerticalSlider, VolumeKnob | Slider | Disabled, value_range |
| Select | Combobox | Expanded, Disabled |
| Dialog | Dialog | — |
| ConfirmDialog | Alertdialog | — |
| Alert | Alert | — |
| Toast | Status/Alert | AriaLive (Polite/Assertive by variant) |
| Tabs | Tablist | — |
| Menu | Menu | — |
| SearchBar | Search | — |
| Progress | Progressbar | value_range |
| Table | Table | — |
| TreeView | Tree | — |
| Accordion | Group | — |
| Toolbar | Toolbar | — |
| Breadcrumbs | Navigation | — |
| Tooltip | Tooltip | — |

**Showcase section**

New "Accessibility" section in the component showcase demonstrating
`.aria_label()`, `.aria_role()`, default roles, and custom role overrides.

**MiniApp initialization**

`MiniApp` automatically initializes the `AccessibilityTree` global.

### Usage

```rust
// Icon-only button with accessible name
Button::new("save-btn", "💾")
    .aria_label("Save document")

// Override default role
Button::new("link-btn", "Visit website")
    .aria_role(AriaRole::Link)

// Query the tree in tests
let tree = cx.global::<AccessibilityTree>();
let node = tree.get(&ElementId::Name("save-btn".into()));
assert_eq!(node.unwrap().props.role, AriaRole::Button);
```
