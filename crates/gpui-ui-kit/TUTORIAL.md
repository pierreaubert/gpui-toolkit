# gpui-ui-kit Tutorial

`gpui-ui-kit` is the reusable component library for GPUI applications.

## 1. Add the crate

```toml
[dependencies]
gpui = { workspace = true }
gpui-ui-kit = { workspace = true }
```

Use `gpui-miniapp` for examples and demos.

## 2. Start with a component

```rust
use gpui::*;
use gpui_ui_kit::{Button, ButtonVariant, Card, Text};

div()
    .child(
        Card::new()
            .child(Text::new("Ready"))
            .child(Button::new("save", "Save").variant(ButtonVariant::Primary)),
    )
```

## 3. Use forms

Common form components include:

- `Input`
- `NumberInput`
- `Checkbox`
- `Toggle`
- `Select`
- `Slider`
- `ColorPickerView`

Keep form state in your entity, update it from callbacks, and validate before
committing changes to your application model.

## 4. Use navigation and surfaces

For application UI, combine:

- `Menu`, `ContextMenu`, `CommandPalette`
- `Tabs`, `Sidebar`, `Toolbar`, `StatusBar`
- `Dialog`, `ConfirmDialog`, `Popover`, `Toast`, `Notification`
- `Table`, `TreeView`, `Wizard`, `WorkflowCanvas`

## 5. Use theme and design globals

```rust
use gpui_ui_kit::theme::ThemeExt;

let theme = cx.theme();
div().bg(theme.surface).text_color(theme.text)
```

## 6. Run examples

```bash
cargo run -p gpui-ui-kit --example showcase
cargo run -p gpui-ui-kit --example input_debug
cargo run -p gpui-ui-kit --example workflow_debug
```

## 7. Verify

```bash
cargo test -p gpui-ui-kit
cargo build --examples -p gpui-ui-kit
```
