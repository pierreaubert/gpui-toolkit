# Rust guide

The Rust path compiles your app directly against the toolkit crates: one
native binary per platform, desktop and mobile from the same view code.
Most GPUI-dependent crates are source beta rather than crates.io packages,
so you work from a checkout kept beside your generated app (its toolkit
dependencies are local paths). This page is the short version; the
[full tutorial](https://github.com/pierreaubert/gpui-toolkit/blob/main/TUTORIAL.md)
covers every step in depth.

## Prerequisites

Git, current stable Rust via `rustup`, and your platform's native build
tools. `just` is optional on desktop, used by the generated mobile recipes.

| Platform | Native prerequisites |
|---|---|
| macOS | Full Xcode with license accepted and the Metal compiler (`xcrun -sdk macosx metal --version`). Command Line Tools alone may lack Metal. |
| Linux | C/C++ toolchain, pkg-config, GPUI Linux backend dev libraries (Wayland/X11, xkbcommon, fontconfig, Vulkan), a graphical session. |
| Windows | Visual Studio C++ build tools, Windows SDK, Rust MSVC toolchain. |

Clone once, where you keep projects:

```bash
git clone https://github.com/pierreaubert/gpui-toolkit.git
cd gpui-toolkit
rustup show
```

The first Cargo build downloads dependencies and compiles the native
renderer — allow more time than for later incremental builds.

## Which command does what?

| Tool | Purpose |
|---|---|
| `gpui-scaffolder` | Generates a runnable app: Cargo files, application view, desktop launcher, optional mobile hosts. |
| `gpui-toolkit` (CLI) | Discovers components and props, inspects templates, exports themes, checks layouts, runs diagnostics. `init` writes a component catalog into an existing project. |
| `cargo` | Compiles, checks, and runs everything. |

Try the CLI from the checkout root — it is the fastest way to learn what
components exist and what props they take:

```bash
cargo run -p gpui-toolkit-cli -- search button --limit 5
cargo run -p gpui-toolkit-cli -- component ui-kit.button --props
cargo run -p gpui-toolkit-cli -- manifest --json   # full command surface
```

## 1. Scaffold an app

Preview, then generate a sibling project (add `--no-ios --no-android` for
desktop-only):

```bash
cargo run -p gpui-scaffolder -- toolkit-counter --output-dir .. --dry-run
cargo run -p gpui-scaffolder -- toolkit-counter --output-dir ..
cd ../toolkit-counter
```

```text
projects/
├── gpui-toolkit/          # keep this source checkout
└── toolkit-counter/
    ├── Cargo.toml
    ├── Justfile
    ├── gpui-scaffold.toml # generator and dependency metadata
    ├── src/app.rs         # shared view: edit this next
    ├── src/lib.rs         # exports and mobile entry points
    ├── src/main.rs        # desktop launcher
    ├── ios/               # Swift/XcodeGen host
    └── android/gradle/    # Android host
```

One alignment step before building: the generator emits an upstream Zed
GPUI dependency, while the toolkit uses its renamed fork. Replace **only
the `gpui = …` line** under `[dependencies]` in the generated `Cargo.toml`
with the value from `[workspace.dependencies]` in the checkout's
`Cargo.toml`; for this revision:

```toml
gpui = { package = "sotf-gpui-toolkit-gpui", git = "https://github.com/pierreaubert/sotf-3rdparties", version = "0.2.3", default-features = false }
```

Keep the generated local toolkit paths and pins. Then:

```bash
cargo check
cargo run
```

A **Toolkit Counter** window opens with a heading and a not-yet-wired
Refresh button. (`just run` is an alias.)

## 2. Make it interactive

Replace the whole of `src/app.rs` (the generated `lib.rs`/`main.rs` already
refer to these exact items):

```rust
use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::{Button, ButtonVariant, Heading, Text, ThemeExt};

pub struct ToolkitCounterView {
    count: u32,
}

impl ToolkitCounterView {
    pub fn new(_: &mut Context<Self>) -> Self {
        Self { count: 0 }
    }
}

impl Render for ToolkitCounterView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id("counter-root")
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .p_8()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .child(Heading::new("Toolkit Counter"))
            .child(Text::new(format!("Count: {}", self.count)))
            .child(
                Button::new("increment", "Increment")
                    .variant(ButtonVariant::Primary)
                    .on_click_event(cx.listener(|this, _: &ClickEvent, _window, cx| {
                        this.count += 1;
                        cx.notify();
                    })),
            )
    }
}

pub fn run_desktop() {
    MiniApp::run(
        MiniAppConfig::new("Toolkit Counter")
            .size(640.0, 480.0)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(ToolkitCounterView::new),
    );
}

pub fn open_app_window(cx: &mut App) {
    if let Err(error) = cx.open_window(WindowOptions::default(), |_, cx| {
        cx.new(ToolkitCounterView::new)
    }) {
        eprintln!("failed to open Toolkit Counter: {error}");
        return;
    }
    cx.activate(true);
}
```

`cargo run` again: **Count: 0**, click Increment three times → **Count:
3**. The handler updates the model and calls `cx.notify()` to re-render;
`cx.listener` reconnects the click to this view's state. `MiniApp` picks
the platform backend and installs theme and design-system globals.
Everything is GPUI elements — no HTML anywhere.

Add optional crates as path dependencies, e.g. charts:
`gpui-px = { path = "../gpui-toolkit/crates/gpui-px", features = ["gpui"] }`.

## The crate map

| Crate | Reach for it when |
|---|---|
| `gpui-ui-kit` | Components: buttons, forms, menus, workflow canvas, audio controls, focus/keyboard handling |
| `gpui-ui-kit-macros` | The `layout!` DSL and theme derivation |
| `gpui-px` | Plotly Express-style charts, incl. unstructured mesh plots |
| `gpui-d3rs` | Low-level D3-inspired primitives: scales, shapes, geo, GPU 2D/3D renderers |
| `gpui-scene3d` | 3D scene rendering: orbit cameras, transfer functions, volume raycasting |
| `gpui-design` | Platform-adaptive design tokens (spacing, corners, typography, animation) |
| `gpui-themes` | Theme state, editor, and showcase |
| `gpui-builder` | Constraint-based responsive layout solving |
| `gpui-miniapp` | The app shell used above (backends, theme/i18n globals) |
| `gpui-mesh` + `gpui-dataset-transformation` | Mesh data model and pure dataset filters (slice, threshold, isosurface, streamlines) |

Every demo on this site is one of these crates running — the
[UI kit](/showcase/), [charts](/px/), [d3rs](/d3rs/), and
[mesh](/mesh/) sources are the fastest way to see idiomatic usage.

## Build and distribute

```bash
cargo build --release
./target/release/toolkit-counter        # .exe on Windows
```

Build on each target platform; the output is an executable, not an
installer. For end users, wrap it per platform — a signed/notarized `.app`
bundle (optionally a DMG) on macOS, MSI/MSIX on Windows, AppImage/deb/rpm
with a desktop entry on Linux — and test on a clean machine. The toolkit
does not generate installers; signing identities and store metadata stay
in your app project. The same `src/app.rs` also drives the generated iOS
and Android hosts (full Xcode / Android SDK required).

## Going further

- [Tutorial §3–11](https://github.com/pierreaubert/gpui-toolkit/blob/main/TUTORIAL.md#3-add-theme-controls) — themes, design system, Figma handoff, keybindings, responsive layout, charts with `gpui-px`, dropping to `d3rs`, Component Lab stories.
- [Tutorial §15](https://github.com/pierreaubert/gpui-toolkit/blob/main/TUTORIAL.md#15-drive-it-from-the-cli) — driving components, templates, and layout checks from the `gpui-toolkit` CLI (scriptable `--json` output).
- [Mobile extension](https://github.com/pierreaubert/gpui-toolkit/blob/main/TUTORIAL.md#13-mobile-extension) — iOS simulator and Android builds from the scaffolded hosts.
- [Repository README](https://github.com/pierreaubert/gpui-toolkit#readme) and [release guide](https://github.com/pierreaubert/gpui-toolkit/blob/main/RELEASE.md) — workspace layout and what ships where.
- Prefer Python for the same app? [The Python guide](../python/) implements this exact counter against the wheel.
