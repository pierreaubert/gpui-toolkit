# gpui-miniapp

`gpui-miniapp` is the workspace-internal application shell used by GPUI toolkit
examples, demos, and showcase crates. It creates the correct GPUI platform
backend for the current target, installs common design-system globals, opens one
window, and wraps the caller's root view in a small shell.

Use it when you want a demo app with consistent menus, theme toggles, language
switching, and scroll handling without repeating platform setup in every
example.

## Platform Selection

`MiniApp::run` calls `current_platform()` before starting GPUI. The helper
selects the backend at compile time:

| Target OS | Backend |
| --- | --- |
| `macos` | `gpui_macos::MacPlatform` |
| `linux` | `gpui_linux::current_platform` |
| `windows` | `gpui_windows::WindowsPlatform` |
| `ios` or `tvos` | `gpui_ios::current_platform` |
| `android` | `gpui_android::current_platform` |

Unsupported targets fail at compile time. Examples that need custom platform
construction should call `current_platform()` directly instead of duplicating
the `cfg` cascade.

## Usage

```rust,ignore
use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};

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

## Configuration

`MiniAppConfig` controls:

- window title, app name, width, and height
- whether the content area scrolls vertically
- theme state and the `cmd-t` theme toggle
- language state and menu-driven language switching
- the initial theme variant and language

The default shell uses `gpui-builder` through the default `builder` feature to
size the single content slot. Disable default features if a demo wants the
plain fill behavior without the layout-solver dependency:

```toml
gpui-miniapp = { workspace = true, default-features = false }
```

## Packaging Boundary

This crate is intentionally `publish = false`. It depends on Zed GPUI platform
backend crates such as `gpui_macos`, `gpui_linux`, and `gpui_windows`, which are
available from the Zed git workspace but are not published on crates.io.

Release guidance:

- Use `gpui-miniapp` for workspace examples, demos, component labs, and internal
  showcase apps.
- Do not expose it as part of the public toolkit API contract until the
  platform backend crates have a crates.io-compatible distribution story.
- Keep publishable library crates independent of `gpui-miniapp`; examples can
  use it as a dev dependency inside this workspace.
- For packaged desktop or mobile products, treat `gpui-miniapp` as a starting
  shell and replace it when the app needs custom menus, signing, entitlements,
  lifecycle hooks, or installer behavior.

## Verification

For host-side changes:

```bash
cargo check -p gpui-miniapp --all-targets
cargo test -p gpui-miniapp
```

For platform release work, pair those checks with the relevant showcase or
scaffolder target gate, such as iOS/tvOS simulator builds, Android NDK builds,
or native Windows checks.
