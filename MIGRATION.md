# GPUI Toolkit Migration Guide

Rules for migrating GPUI binaries to the gpui-toolkit design system. Every binary in gpui-toolkit must follow these rules. No exceptions.

## Core Rules

### 1. No Colors in Application Code

All colors come from the theme via `cx.theme()`. Application code must not contain `rgb()`, `rgba()`, `hsla()`, or any color literal.

```rust
// WRONG
div().bg(rgb(0x1e1e1e)).text_color(rgb(0xffffff))

// RIGHT
let theme = cx.theme();
div().bg(theme.surface).text_color(theme.text_primary)
```

**Color mapping reference:**

| Hardcoded | Theme field |
|-----------|------------|
| Dark background (`0x1e1e1e`, `0x1a1a1a`, `0x2a2a2a`) | `theme.surface` |
| Light background (`0xffffff`, `0xf8f8f8`, `0xf5f5f5`) | `theme.background` or `theme.surface` |
| Muted background (`0xe5e7eb`, `0xe0e0e0`, `0x0a0a0a`) | `theme.muted` |
| Hover background (`0x2d2d2d`, `0x3a3a3a`, `0xd0d0d0`) | `theme.surface_hover` |
| Primary text (`0xffffff`, `0x333333`) | `theme.text_primary` |
| Secondary text (`0xcccccc`, `0x666666`) | `theme.text_secondary` |
| Muted text (`0x888888`, `0x555555`) | `theme.text_muted` |
| Text on accent (`0xffffff` on colored bg) | `theme.text_on_accent` |
| Accent/selection (`0x007acc`, `0x3b82f6`) | `theme.accent` |
| Accent hover | `theme.accent_hover` |
| Accent muted (semi-transparent accent) | `theme.accent_muted` |
| Border (`0x3c3c3c`, `0x3a3a3a`, `0xdddddd`) | `theme.border` |
| Error text (`0xd32f2f`) | `theme.error` |
| Overlay backdrop | `theme.overlay_bg` |

**Domain-specific colors** (data visualization palettes like viridis, CEA2034 curve colors) are allowed as `D3Color` values since they represent data, not UI chrome.

**GPU 3D backgrounds** must also use the theme:
```rust
let bg = theme.surface;
config.background_color(bg.r, bg.g, bg.b)
```

**Glyph chart text** uses `Hsla::from(theme.text_primary)`:
```rust
GlyphTextConfig::horizontal((10.0 * s).round(), Hsla::from(theme.text_primary))
```

### 2. All Spacing from the Design System

All spacing, padding, gaps, corner radii, and text sizes come from `cx.design()`. Application code must not use GPUI's built-in spacing methods (`.px_3()`, `.gap_4()`, `.p_8()`, `.rounded_md()`, `.text_sm()`, etc.).

```rust
// WRONG
div().px_3().py_2().gap_4().rounded_md().text_sm()

// RIGHT
let ds = cx.design();
div()
    .px(px(ds.spacing.control_padding_x))
    .py(px(ds.spacing.control_padding_y))
    .gap(px(ds.spacing.section_gap))
    .rounded(px(ds.corners.md))
    .text_size(px(ds.typography.small_size))
```

**Spacing mapping reference:**

| GPUI method | Design system field |
|-------------|-------------------|
| `.px_3()` (12px) | `px(ds.spacing.control_padding_x)` |
| `.py_2()` (8px) | `px(ds.spacing.control_padding_y)` |
| `.py_1()` (4px) | `px(ds.spacing.control_padding_y * 0.5)` |
| `.px_4()` / `.p_4()` (16px) | `px(ds.spacing.card_padding)` |
| `.p_8()` (32px) | `px(ds.spacing.section_gap * 2.0)` |
| `.gap_2()` (8px) | `px(ds.spacing.control_gap)` |
| `.gap_4()` (16px) | `px(ds.spacing.section_gap)` |
| `.gap_6()` (24px) | `px(ds.spacing.section_gap * 1.5)` |
| `.mb_2()` (8px) | `.mb(px(ds.spacing.control_gap))` |
| `.mb_4()` (16px) | `.mb(px(ds.spacing.section_gap))` |
| `.mt_1()` (4px) | `.mt(px(ds.spacing.grid_unit))` |
| `.rounded_sm()` | `px(ds.corners.sm)` |
| `.rounded_md()` | `px(ds.corners.md)` |
| `.rounded_lg()` | `px(ds.corners.lg)` |
| `.rounded(px(4.0))` | `px(ds.corners.sm)` |

**Typography mapping reference:**

| GPUI method | Design system field |
|-------------|-------------------|
| `.text_xs()` | `px(ds.typography.small_size * 0.85)` |
| `.text_sm()` | `px(ds.typography.small_size)` |
| `.text_base()` | `px(ds.typography.base_size)` |
| `.text_lg()` | `px(ds.typography.large_size)` |
| `.text_xl()` / `.text_2xl()` | `px(ds.typography.large_size)` |

### 3. Charts Scale with Window Size

Charts must not have hardcoded pixel dimensions. Store content dimensions on your app state, update them from `window.bounds()` every render, and compute chart sizes as fractions of available space.

```rust
// App state
pub struct MyApp {
    pub content_width: f32,
    pub content_height: f32,
}

// In Render impl — compute available space
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let bounds = window.bounds();
    let win_w: f32 = bounds.size.width.into();
    let win_h: f32 = bounds.size.height.into();
    // Subtract sidebar, header, padding
    self.content_width = (win_w - sidebar_w - padding).max(400.0);
    self.content_height = (win_h - header_h - padding).max(300.0);
    // ...
}

// In chart render functions — use fractions, not absolutes
let chart_width = app.content_width;
let chart_height = (chart_width * 0.5).min(app.content_height * 0.6);
```

**Aspect ratio guidelines by chart type:**

| Chart type | Width | Height |
|-----------|-------|--------|
| Frequency/SPL (2:1) | `content_width` | `width * 0.5` capped at `content_height * 0.6` |
| Bar/line/scatter | `content_width * 0.7` | `width * 0.5` capped at `content_height * 0.4` |
| Square (quadtree, contour) | `min(content_width * 0.5, content_height * 0.6)` | same as width |
| Force/hierarchy | `content_width` | `width * 0.75` capped at `content_height * 0.8` |
| 3D surface (16:9) | `content_width` | `width * 0.56` capped at `content_height * 0.6` |
| Sankey/flow | `content_width` | `width * 0.71` capped at `content_height * 0.8` |
| Horizon (fixed bands) | `content_width` | fixed 80px per band |
| Geo/map | `content_width` | `width * 0.625` capped at `content_height * 0.8` |

For d3_examples that use `f64` math:
```rust
let width = app.content_width as f64;
let height = (width * 0.5).min(app.content_height as f64 * 0.6);
```

### 4. Font Sizes Scale with Content Width

Chart-related text (titles, axis labels, legends) must scale with the content area width. Define a `font_scale()` method on your app:

```rust
impl MyApp {
    fn font_scale(&self) -> f32 {
        (self.content_width / 800.0).clamp(0.7, 1.2)
    }
}
```

Apply to all chart text:
```rust
let s = self.font_scale();

// UI text in chart views
div().text_size(px(ds.typography.large_size * s))   // titles
div().text_size(px(ds.typography.small_size * s))    // labels

// Glyph chart labels
GlyphTextConfig::horizontal((10.0 * s).round(), Hsla::from(theme.text_primary))

// d3rs axis configs
AxisConfig::bottom()
    .with_label_font_size((10.0 * scale).round())
    .with_title_font_size((12.0 * scale).round())
    .with_tick_size((6.0 * scale).round().max(4.0))
```

UI chrome text (headers, dropdowns, menus) does NOT scale — only chart content text.

### 5. Use MiniApp for Application Shell

All binaries must use `MiniApp` with theme enabled:

```rust
fn main() {
    MiniApp::run(
        MiniAppConfig::new("My App")
            .size(1200.0, 800.0)
            .with_theme(true)
            .scrollable(false), // false for apps with their own layout
        |cx| cx.new(MyApp::new),
    );
}
```

This gives you:
- Theme switching via View > Theme menu (Cmd+T to toggle)
- Design system switching via View > Design System menu
- `cx.theme()` and `cx.design()` available everywhere
- Quit via Cmd+Q

### 6. Use ChartTheme Bridge for d3rs Charts

The d3rs axis/grid system has its own `AxisTheme` trait. Bridge it from the UI theme:

```rust
pub struct ChartTheme {
    pub line_color: Rgba,
    pub label_color: Rgba,
    pub bg: Option<Rgba>,
}

impl ChartTheme {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            line_color: theme.text_secondary,  // axis lines
            label_color: theme.text_primary,    // high contrast labels
            bg: Some(theme.surface),            // chart background
        }
    }
}

impl AxisTheme for ChartTheme { ... }
```

### 7. Use `render_legend()` for Chart Legends

Use the `d3rs::legend::render_legend()` function instead of manual flex-wrap layouts. It automatically computes optimal column count to minimize height without overlap.

```rust
use d3rs::legend::{LegendConfig, LegendItem, render_legend};

let legend_config = LegendConfig::new()
    .font_size(11.0)
    .symbol_size(10.0)
    .item_spacing(4.0)
    .padding(6.0)
    .items(items);

let legend = render_legend(&legend_config, chart_width, theme.text_primary, Some(theme.muted));
```

### 8. 3D Views: Overlay Color Adapts to Background

The 3D surface renderer draws axis labels as overlays. The overlay color must contrast with the GPU background:

```rust
let bg = self.config.background_color;
let luminance = 0.299 * bg[0] + 0.587 * bg[1] + 0.114 * bg[2];
let overlay_color = if luminance > 0.5 {
    gpui::rgba(0x000000ff)
} else {
    gpui::rgba(0xffffffff)
};
```

### 9. Geo Projections Clip Correctly

Azimuthal projections (Stereographic, Orthographic) must clip points beyond their `clip_angle`. The `project()` method returns `(NaN, NaN)` for clipped points, and `GeoPath` skips them.

```rust
// Already implemented in d3rs — no action needed in app code.
// Just be aware: the back hemisphere is correctly hidden.
```

## Migration Checklist

For each binary:

- [ ] Uses `MiniApp::run()` with `.with_theme(true)`
- [ ] Zero `rgb()`, `rgba()`, `hsla()` in application code (except data visualization colors)
- [ ] All spacing uses `cx.design()` values, no `.px_N()` / `.gap_N()` / `.rounded_md()` etc.
- [ ] All text sizes use `cx.design()` typography values, no `.text_sm()` / `.text_lg()` etc.
- [ ] App state has `content_width` / `content_height`, updated from `window.bounds()` each render
- [ ] Chart dimensions computed as fractions of content area, not hardcoded pixels
- [ ] Chart text scales with `font_scale()` method
- [ ] d3rs axes use `ChartTheme::from_theme()` instead of `DefaultAxisTheme`
- [ ] Legends use `render_legend()` with automatic column layout
- [ ] Theme changes via View menu visually update all elements
- [ ] Design system changes via View menu visually update spacing/corners/typography
- [ ] Window resize causes all charts to scale proportionally

## Verification

```bash
# Check compilation
cargo check -p gpui-d3rs

# Run tests
cargo test -p gpui-d3rs --lib --no-default-features

# Visual verification — resize window, switch themes and design systems
cargo run --bin d3rs-showcase --features="gpui,gpu-2d"

# Audit for violations
grep -rc "rgb(0x\|rgba(0x\|hsla(" path/to/binary.rs  # should be 0
grep -rc "\.px_[0-9]\|\.gap_[0-9]\|\.rounded_md\|\.text_sm()" path/to/binary.rs  # should be 0
grep -rc "let width = [0-9]" path/to/chart.rs  # should be 0
```

## Imports

```rust
use gpui_design::DesignExt;              // cx.design()
use gpui_ui_kit::theme::ThemeExt;        // cx.theme()
use gpui_ui_kit::{MiniApp, MiniAppConfig};
```

For d3rs chart rendering:
```rust
use d3rs::axis::{AxisConfig, render_axis};
use d3rs::legend::{LegendConfig, LegendItem, render_legend};
use super::utils::ChartTheme;  // your theme bridge
```
