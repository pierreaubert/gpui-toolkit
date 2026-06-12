# gpui-miniapp Tutorial

`gpui-miniapp` is a minimal application shell for toolkit examples and demos.

## 1. Add the crate

```toml
[dependencies]
gpui = { workspace = true }
gpui-miniapp = { workspace = true }
gpui-ui-kit = { workspace = true }
```

## 2. Create a renderable view

```rust
use gpui::*;

struct HelloApp;

impl HelloApp {
    fn new(_: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for HelloApp {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().p_4().child("Hello from GPUI")
    }
}
```

## 3. Run it

```rust
use gpui_miniapp::{MiniApp, MiniAppConfig};

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Hello App")
            .size(900.0, 700.0)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(HelloApp::new),
    );
}
```

## 4. Configure behavior

Use `MiniAppConfig` to set:

- title and app name
- width and height
- scrollable content
- theme support
- i18n support
- initial theme and language

## 5. Verify

Run any MiniApp-based example:

```bash
cargo run -p gpui-ui-kit --example showcase
cargo run -p gpui-px --bin px-showcase --features gpui
```
