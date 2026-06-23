# gpui-ui-kit Tutorial

`gpui-ui-kit` is the reusable component library for GPUI applications.

## 1. Scaffold a MiniApp

From the repository root, create a small standalone GPUI app:

```bash
cargo run -p gpui-scaffolder -- ui-kit-demo
cd ui-kit-demo
cargo run
```

The generated app uses `gpui-miniapp`, which installs the GPUI application
shell used by the UI-kit examples and demos. It also includes `gpui-ui-kit` so
you can immediately use the component imports below.

If you generated the app at the repository root, its `Cargo.toml` should include:

```toml
[dependencies]
gpui = { version = "0.2.2", git = "https://github.com/zed-industries/zed.git", tag = "v1.0.0" }
gpui-miniapp = { path = "../crates/gpui-miniapp" }
gpui-ui-kit = { path = "../crates/gpui-ui-kit" }
```

Adjust the `../crates/...` paths if you used `--output-dir`.

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
div().bg(theme.surface).text_color(theme.text_primary)
```

## 6. Run examples

```bash
cargo run
just run
cargo run -p gpui-ui-kit --example showcase
cargo run -p gpui-ui-kit --example input_debug
cargo run -p gpui-ui-kit --example workflow_debug
```

## 7. Verify

```bash
cargo test -p gpui-ui-kit
cargo build --examples -p gpui-ui-kit
```
