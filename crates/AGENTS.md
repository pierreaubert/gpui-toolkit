# GPUI Toolkit - Agent's Guide

Quick reference for AI agents working with the GPUI toolkit libraries.

## Overview

The `gpui-toolkit` workspace contains several related crates for building GPUI applications:

| Crate | Purpose | Documentation |
|-------|---------|---------------|
| `gpui-d3rs` | Low-level D3.js-inspired visualization primitives | [README](gpui-d3rs/README.md) |
| `gpui-px` | High-level Plotly Express-style charting API | [README](gpui-px/README.md) |
| `gpui-themes` | Theme editor and management infrastructure | [AGENTS.md](gpui-themes/AGENTS.md) |
| `gpui-ui-kit` | Reusable UI components (buttons, forms, layout) | [lib.rs](gpui-ui-kit/src/lib.rs) |
| `gpui-ui-kit-macros` | Procedural macros for theme derivation | [README](gpui-ui-kit-macros/README.md) |

**Key Principle**: All crates use GPUI's native `div()`-based rendering, not HTML/SVG. Components return `impl IntoElement`.

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
```

## Quick Links

- [GPUI Documentation](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [D3.js Reference](https://d3js.org/) (for understanding gpui-d3rs concepts)
- Main project [GPUI.md](../../GPUI.md) for project-wide GPUI conventions
