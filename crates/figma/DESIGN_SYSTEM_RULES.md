# SOTF Design System Rules — Figma to GPUI

## 1. Token Definitions

### Colors
Defined by `gpui_design::DesignSystem` in
`crates/gpui-design/src/lib.rs`. UI components resolve design
defaults through `DesignExt` and may accept an explicit `.design(...)`
override.

Export/import/validation lives in
`crates/gpui-design-tools`:

```bash
cargo run -p gpui-design-tools --bin gpui-export-design-tokens
cargo run -p gpui-design-tools --bin gpui-validate-design-tokens
```

Token JSON is backed by
`gpui_design::DesignSystem::style_dictionary_tokens` and grouped into names
such as `color.background`, `color.surface`, `color.text.primary`,
`color.accent`, `space.sm`, `radius.md`, `typography.text.md`, and
`motion.duration.fast`.

### Typography
- Use `DesignSystem` typography tokens for family, scale, line height, and
  weights.
- Legacy `gpui-ui-kit::theme::Theme` remains an app/theme bridge, not the
  source of Figma token truth.

### Spacing
Use `DesignSystem` spacing tokens (`space.xs`, `space.sm`, `space.md`,
`space.lg`, `space.xl`, `space.xxl`) through component builders.

### Border Radius
Use `DesignSystem` radius tokens (`radius.sm`, `radius.md`, `radius.lg`,
`radius.xl`, `radius.full`).

### Component Sizes
`ComponentSize` enum: Xs (0.5x), Sm (0.75x), Md (1.0x default), Lg (1.5x), Xl (2.0x)

## 2. Component Library

Located at `crates/gpui-ui-kit/src/`. Each component is a single `.rs` file.

### Core Components
| File | Component | Key Variants |
|------|-----------|-------------|
| `button.rs` | `Button` | ButtonVariant: Primary/Secondary/Destructive/Ghost/Outline, ButtonSize: Xs/Sm/Md/Lg |
| `icon_button.rs` | `IconButton` | IconButtonVariant: Ghost/Filled/Outline, IconButtonSize: Xs/Sm/Md/Lg/Xl |
| `input.rs` | `Input` | InputVariant: Default/Filled/Flushed, InputSize: Xs/Sm/Md/Lg |
| `number_input.rs` | `NumberInput` | NumberInputSize: Xs/Sm/Md/Lg |
| `select.rs` | `Select` | SelectSize: Xs/Sm/Md/Lg |
| `checkbox.rs` | `Checkbox` | CheckboxSize: Sm/Md/Lg |
| `toggle.rs` | `Toggle` | ToggleStyle: Sliding/Segmented, ToggleSize: Sm/Md/Lg |
| `slider.rs` | `Slider` | SliderSize: Sm/Md/Lg |
| `badge.rs` | `Badge` | BadgeVariant: Default/Primary/Success/Warning/Error/Info, BadgeSize: Sm/Md/Lg |
| `alert.rs` | `Alert` | AlertVariant: Info/Success/Warning/Error |
| `toast.rs` | `Toast` | ToastVariant: Info/Success/Warning/Error |
| `progress.rs` | `Progress` | ProgressVariant: Default/Success/Warning/Error, ProgressSize: Xs/Sm/Md/Lg |
| `spinner.rs` | `Spinner` | SpinnerSize: Xs/Sm/Md/Lg/Xl |
| `tabs.rs` | `Tabs` | TabVariant: Underline/Enclosed/Pills/VerticalCard |
| `button_set.rs` | `ButtonSet` | ButtonSetSize: Xs/Sm/Md/Lg |
| `breadcrumbs.rs` | `Breadcrumbs` | BreadcrumbSeparator |
| `menu.rs` | `Menu` | MenuItem types |
| `accordion.rs` | `Accordion` | AccordionMode: Single/Multiple, AccordionOrientation: Vertical/Horizontal/Side |
| `tooltip.rs` | `Tooltip` | TooltipPlacement |
| `card.rs` | `Card` | Slot-based (header, body, footer) |
| `dialog.rs` | `Dialog` | DialogSize: Sm/Md/Lg |
| `table.rs` | `Table` | SelectionMode, SortDirection |
| `wizard.rs` | `Wizard` | StepStatus: Complete/Current/Pending/Error |
| `avatar.rs` | `Avatar` | AvatarSize, AvatarShape: Circle/Square, AvatarStatus |

### Layout Components
| File | Component |
|------|-----------|
| `stack.rs` | `VStack`, `HStack` — StackSpacing: None/Xs/Sm/Md/Lg/Xl/Xxl, StackAlign, StackJustify |
| `divider.rs` | `Divider` |
| `pane_divider.rs` | `PaneDivider` |

### Chart Components
| File | Component |
|------|-----------|
| `../gpui-px/src/line.rs` | `LineChart` |
| `../gpui-px/src/bar.rs` | `BarChart` |
| `../gpui-px/src/scatter.rs` | `ScatterChart` |
| `../gpui-px/src/area.rs` | `AreaChart` |
| `../gpui-px/src/heatmap.rs` | `HeatmapChart` |
| `../gpui-px/src/contour.rs` | `ContourChart` |
| `../gpui-px/src/isoline.rs` | `IsolineChart` |
| `../gpui-px/src/pie.rs` | `PieChart` |
| `../gpui-px/src/boxplot.rs` | `BoxPlotChart` |
| `../gpui-px/src/treemap.rs` | `Treemap` |

Charts should use `.fill().min_size(...).aspect_ratio(...)` by default in lab
and product surfaces. Use `.size(width, height)` only when a fixed pixel chart
is intentional.

### Audio Components
| File | Component |
|------|-----------|
| `../gpui-audio-kit/src/audio/potentiometer.rs` | `Potentiometer` — PotentiometerSize: Xs/Sm/Md/Lg |
| `../gpui-audio-kit/src/audio/vertical_slider.rs` | `VerticalSlider` |
| `../gpui-audio-kit/src/audio/volume_knob.rs` | `VolumeKnob` |
| `../gpui-audio-kit/src/meter.rs` | `LevelMeterElement`, `MeterColors`, `HorizontalMeterTheme`, horizontal meter bar helpers |
| `../gpui-audio-kit/src/spectrum.rs` | `SpectrumElement`, `SpectrumColors`, `MeterData`, `SpectrumAxisTheme`, spectrum frequency/dB axis helpers |

Audio APIs are not re-exported by `gpui-ui-kit`. Design review coverage for
audio lives in component-lab stories `audio-kit.potentiometer`,
`audio-kit.vertical-slider`, `audio-kit.volume-knob`, `audio-kit.meter`,
`audio-kit.horizontal-meter`, `audio-kit.spectrum`, and
`audio-kit.spectrum-axis`.

## 3. Framework

- **UI Framework**: GPUI (Zed's GPU-accelerated UI framework for Rust)
- **Styling**: Inline via GPUI's fluent API (`div().bg(color).px(px(16)).rounded(px(6))`)
- **No CSS** — all styling is Rust code
- **Build system**: Cargo (Rust)

## 4. Component Architecture Pattern

Builder pattern with fluent API:

```rust
// Creating a button
Button::new("save-btn", "Save")
    .variant(ButtonVariant::Primary)
    .size(ButtonSize::Md)
    .on_click(|_, cx| { /* handler */ })

// Creating a stack layout
VStack::new()
    .spacing(StackSpacing::Md)
    .align(StackAlign::Stretch)
    .child(Button::new("btn1", "Click"))
    .child(Input::new("input1").placeholder("Type..."))

// Creating a toggle
Toggle::new("enable-toggle")
    .checked(is_checked)
    .style(ToggleStyle::Sliding)
    .size(ToggleSize::Md)
    .label("Enable")
    .on_change(|_checked, _window, _cx| { /* handler */ })
```

All components implement GPUI's `RenderOnce` or `Render` trait and are
composed via `.child()` calls. Component story metadata is registered in
`crates/gpui-component-lab` for prop panels, responsive matrices,
and conformance checks. First-party `gpui-ui-kit`, `gpui-px`, and
`gpui-audio-kit` stories must have matching interactive lab renderers.

## 5. Figma-to-GPUI Translation Rules

When translating from Figma designs:

1. **Auto Layout → VStack/HStack**: Vertical auto layout = `VStack`, horizontal = `HStack`. Map gap to `StackSpacing`.
2. **Fill container → `.flex_grow()`**: Figma "Fill" = `flex_grow()` in GPUI.
3. **Fixed size → `.w(px(N)).h(px(N))`**: Direct pixel mapping.
4. **Corner radius → `.rounded(px(N))`**: Map to border radius tokens.
5. **Colors → Design tokens**: Map Figma fills to `DesignSystem` color tokens, never hardcode hex in components.
6. **Component instances → Builder calls**: Map Figma component variants to enum values in the builder pattern.
7. **Text → `Text::new("content").size(TextSize::Md).weight(TextWeight::Normal)`**
8. **Padding → `.px(px(N)).py(px(N))`**: Map Figma padding to GPUI padding calls.

## 6. Icon System

There is no standalone `gpui-icons` crate. Use `gpui_ui_kit::IconButton` for icon actions and keep project-specific SVG or raster assets with their owning component.

## 7. Asset Management

No CDN. Assets are embedded at compile time or loaded from local filesystem. Images use GPUI's image rendering APIs.

## 8. Project Structure

```
crates/
  gpui-design/         # DesignSystem tokens, presets, conformance checks
  gpui-design-tools/   # Token export/import/validation CLIs
  gpui-component-lab/  # Story registry, responsive previews, designer JSON
  gpui-ui-kit/         # General-purpose UI components
  gpui-audio-kit/      # Audio controls, meters, spectrum, tick helpers
  gpui-d3rs/           # D3-style visualizations for GPUI
  gpui-px/             # High-level charting API
  gpui-toolkit/        # Optional facade crate
```
