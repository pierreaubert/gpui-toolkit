# GPUI Toolkit Tutorial

Build **Toolkit Counter**, a native window with a count and an **Increment**
button. You will create an app, change its UI and behavior, run it, and produce
an artifact you can share. Choose [Rust](#1-scaffold-an-app) or
[Python](#12-python-runtime-demo); both implement the same interaction.

- [Set up your tools](#before-you-start)
- [Understand the CLI and scaffolder](#which-command-does-what)
- [Create the Rust project](#1-scaffold-an-app) and [add the counter](#2-extend-the-scaffolded-miniapp)
- [Create the Python app](#12-python-runtime-demo)
- [Build and distribute](#build-and-distribute)
- [Continue to mobile](#13-mobile-extension)
- [Give an agent a reproducible workflow](#agent-workflow)

Sections 3–11 are optional next steps for themes, layouts, charts, and design
handoff. Finish the counter first; you do not need every toolkit crate.

## Before you start

The Rust path uses a source checkout: most GPUI-dependent crates are source
beta rather than crates.io packages. Keep the checkout beside your generated
app because its toolkit dependencies are local paths. See [release lanes](RELEASE.md).

For Rust, install Git, current stable Rust through rustup, and your platform's
native build tools. `just` is optional for desktop but used by the generated
mobile recipes. Building the CLI or Python host from source needs the same
native tools as building a Rust app.

| Platform | Native prerequisites |
| --- | --- |
| macOS | Full Xcode, its license accepted, and the Metal compiler available via `xcrun -sdk macosx metal --version`. Command Line Tools alone may lack Metal. Select your installed Xcode in `xcode-select` if necessary. |
| Linux | A C/C++ toolchain, pkg-config, and development libraries for the GPUI Linux backend (Wayland/X11, xkbcommon, fontconfig, Vulkan); a working graphical session and graphics drivers. See the [GPUI Linux backend sources](https://github.com/pierreaubert/sotf-3rdparties/tree/main/gpui/crates/gpui_linux). |
| Windows | Visual Studio C++ build tools, the Windows SDK, and the Rust MSVC toolchain. |

For the Python wheel path, start with Python 3.10+ in a virtual environment.
A compatible wheel with a bundled host avoids compiling Rust. If your install
has no host, use the source-build fallback in the Python section.

The commands below use a POSIX shell (macOS/Linux). In PowerShell use
`.venv\Scripts\Activate.ps1` to activate the environment and set environment
variables with `$env:NAME = "value"`. Run the Windows `.exe` equivalents.

Clone once, starting in the directory where you keep projects:

```bash
git clone https://github.com/pierreaubert/gpui-toolkit.git
cd gpui-toolkit
rustup show
```

Unless a step says otherwise, commands run from this checkout root. The first
Cargo build downloads dependencies and compiles the native renderer; allow
more time than for later incremental builds.

## Which command does what?

| Tool | Purpose | Creates a runnable project? |
| --- | --- | --- |
| `gpui-toolkit` | Discover components and their props, inspect templates, export themes, check layouts, and run diagnostics. | `init` writes a component catalog into an existing project's agent documentation. Use the scaffolder to create the app. |
| `gpui-scaffolder` | Generate Cargo files, an application view, a desktop launcher, and optional mobile hosts. | Yes. |
| `cargo` | Compile, check, and run the Rust project or native Python host. | Builds the generated project. |
| `gpui_toolkit` | Python API for UI declarations, state, and native-host events. | Write a Python script; the native host renders it. |

Try the development CLI from the checkout root:

```bash
cargo run -p gpui-toolkit-cli -- --help
cargo run -p gpui-toolkit-cli -- manifest --json
cargo run -p gpui-toolkit-cli -- search button --limit 5
cargo run -p gpui-toolkit-cli -- component ui-kit.button --props
```

`cargo run -p gpui-toolkit-cli -- ...` runs the binary named `gpui-toolkit`.
Arguments after `--` go to that binary. To use the shorter command from any
directory, install it with `cargo install --path crates/gpui-toolkit-cli`, then
run `gpui-toolkit --help`. The CLI is optional for running the counter; it is
useful when you start adding components. More commands are in
[section 15](#15-drive-it-from-the-cli).

## 1. Scaffold an App

From the checkout root, preview the files and generate a sibling project:

```bash
cargo run -p gpui-scaffolder -- toolkit-counter --output-dir .. --dry-run
cargo run -p gpui-scaffolder -- toolkit-counter --output-dir ..
cd ../toolkit-counter
```

The preview lists files without writing them. Generation creates this structure:

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

The generator also writes a project README. Only the `default` scaffold template
exists today; the development CLI's component/page templates are separate.
`--no-ios --no-android` omits mobile hosts when you only need desktop.
The generator refuses to overwrite a populated directory; choose a new name
when repeating this exercise.

### Align the GPUI dependency

This checkout's generator currently emits an upstream Zed GPUI dependency,
while the toolkit crates use the renamed GPUI fork. Before building, replace
**only the `gpui = ...` line** under `[dependencies]` in the generated
`Cargo.toml` with the value from `[workspace.dependencies]` in the checkout's
`Cargo.toml`. For this revision it is:

```toml
gpui = { package = "sotf-gpui-toolkit-gpui", git = "https://github.com/pierreaubert/sotf-3rdparties", version = "0.2.3", default-features = false }
```

Keep the generated local toolkit paths, platform dependencies, and compatibility
pins. This makes your view and the toolkit use the same GPUI types. After an
upgrade, use the current checkout's dependency value instead of an older snippet.

Now, from `toolkit-counter/`:

```bash
cargo check
cargo run
```

A window titled **Toolkit Counter** opens with a heading and a **Refresh**
button. That generated button has no action yet. Close the window before the
next step. `just run` is an optional alias for `cargo run`.

## 2. Extend the Scaffolded MiniApp

Replace the **whole contents of `src/app.rs`** with the following. Keep
`src/lib.rs` and `src/main.rs`: they already refer to `ToolkitCounterView`,
`run_desktop`, and `open_app_window`, which this file preserves.

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

From `toolkit-counter/`:

```bash
cargo check
cargo run
```

You should see **Count: 0**. Click **Increment** three times: the text becomes
**Count: 3**. Close and reopen the app: it starts at zero again. State lives in
the view; persistence is a separate feature you can add later.

The event handler updates the model and calls `cx.notify()` to request another
render. `on_click_event` accepts `cx.listener`, which reconnects the click to
this view's state. `MiniApp` selects the platform backend and installs theme
and design-system globals. The UI consists of GPUI elements, not HTML.

You now have a desktop app. Jump to [distribution](#build-and-distribute), try
[the Python version](#12-python-runtime-demo), or continue with optional toolkit
features below. For an optional crate, add a path dependency under
`[dependencies]`, for example `gpui-px = { path = "../gpui-toolkit/crates/gpui-px", features = ["gpui"] }`.

## 3. Add Theme Controls

`gpui-miniapp` installs theme state when `.with_theme(true)` is enabled.
Components can read it through `ThemeExt`.

```rust
use gpui_ui_kit::theme::ThemeExt;

let theme = cx.theme();
div()
    .bg(theme.surface)
    .border_color(theme.border)
    .text_color(theme.text_primary)
```

For a full theme editor:

```bash
cargo run -p gpui-themes --bin theme-editor
cargo run -p gpui-themes --bin theme-showcase
```

## 4. Use the Design System

`gpui-design` stores platform-adaptive rules for spacing, typography, corners,
elevation, interaction, and animation. `MiniApp` installs
`DesignSystemState::new()` automatically.

Use design values when writing shared UI surfaces:

```rust
use gpui_design::DesignExt;

let ds = cx.design();
div()
    .gap(ds.spacing.md)
    .rounded(ds.corners.card)
```

Validate design tokens:

```bash
cargo run -p gpui-design-tools --bin gpui-validate-design-tokens
```

Export tokens for external tools:

```bash
cargo run -p gpui-design-tools --bin gpui-export-design-tokens -- \
  --format style-dictionary-json \
  --output target/design-tokens.json
```

## 5. Bring in Figma Handoff

The [figma](./crates/figma/) directory contains design-system rules and Code
Connect mappings. A practical handoff loop is:

1. Keep Figma component names aligned with `gpui-ui-kit` component names.
2. Export or review tokens with `gpui-design-tools`.
3. Use [crates/figma/DESIGN_SYSTEM_RULES.md](./crates/figma/DESIGN_SYSTEM_RULES.md)
   to check naming, density, and token expectations.
4. Use [crates/figma/CODE_CONNECT_MAPPINGS.md](./crates/figma/CODE_CONNECT_MAPPINGS.md)
   to map Figma components to GPUI component constructors.
5. Run `just qa-gpui-conformance` before accepting a design-system change.

## 6. Designer Workflow: Figma to Component Lab

Use this workflow when a designer changes a component, token, or responsive
behavior:

1. Start in Figma and name the component, variants, and slots after the closest
   `gpui-ui-kit` component.
2. Check the mapping in
   [CODE_CONNECT_MAPPINGS.md](./crates/figma/CODE_CONNECT_MAPPINGS.md).
3. Export or validate design tokens with `gpui-design-tools`.
4. Implement the GPUI component or prop change.
5. Add a story in `gpui-component-lab` for every important state: default,
   hover, disabled, selected, dense, narrow viewport, and high-contrast theme.
6. Run `just qa-gpui-conformance`.
7. Attach the generated markdown report to the design review.

## 7. Add Keybindings

`gpui-keybinding` lets each feature provide a set of bindings per preset, plus
human-readable documentation for help or command-palette UI.

```rust
use gpui::{actions, KeyBinding};
use gpui_keybinding::{
    DocumentedKeybinding, KeybindingCategory, KeybindingProvider,
    KeybindingRegistry, KeymapPreset,
};

actions!(dashboard, [RefreshDashboard]);

struct DashboardBindings;

impl KeybindingProvider for DashboardBindings {
    fn bindings(&self, preset: KeymapPreset) -> Vec<KeyBinding> {
        match preset {
            KeymapPreset::Default | KeymapPreset::VSCode => {
                vec![KeyBinding::new("secondary-r", RefreshDashboard, None)]
            }
            KeymapPreset::Vim => vec![KeyBinding::new("r", RefreshDashboard, None)],
            KeymapPreset::Emacs => vec![KeyBinding::new("ctrl-r", RefreshDashboard, None)],
        }
    }

    fn documented_bindings(&self, _: KeymapPreset) -> Vec<DocumentedKeybinding> {
        vec![
            DocumentedKeybinding::new(
                "Refresh",
                "Refresh dashboard data",
                KeybindingCategory::View,
            )
            .with_raw_key_spec("refresh"),
        ]
    }
}

let mut registry = KeybindingRegistry::new();
registry.register(DashboardBindings);
let conflicts = registry.detect_conflicts(KeymapPreset::Default);
```

Bind the returned `KeyBinding` values during app initialization, then handle the
action with `cx.on_action::<RefreshDashboard>(...)`.

## 8. Solve Responsive Layouts

Use `gpui-builder` when you need a layout contract that can be tested outside
the visual component.

```rust
use gpui_builder::{Axis, ContainerNode, LayoutPreferences, Sizing, SlotNode, solve};

let children = [
    SlotNode::new("sidebar", Sizing::Fixed(260.0)).into_node(),
    SlotNode::new("content", Sizing::flex(480.0)).into_node(),
];

let root = ContainerNode::new(
    "root",
    Axis::Horizontal,
    Sizing::flex(0.0),
    &children,
)
.into_node();
let solved = solve(&root, 1200.0, 780.0, &LayoutPreferences::default());
```

Explore the solver:

```bash
cargo run -p gpui-builder --bin layout-showcase --features showcase
cargo run -p gpui-builder --example responsive_dashboard
```

## 9. Add Graphs with gpui-px

Use `gpui-px` for high-level charts.

```rust
use gpui_px::{bar, line, scatter, ScaleType};

let x = vec![1.0, 10.0, 100.0, 1000.0];
let y = vec![2.0, 8.0, 32.0, 128.0];

let scatter_chart = scatter(&x, &y)
    .title("Latency by load")
    .x_scale(ScaleType::Log)
    .build()?;

let line_chart = line(&x, &y)
    .title("Trend")
    .build()?;

let bars = bar(&["A", "B", "C"], &[4.0, 7.0, 3.0])
    .title("Buckets")
    .build()?;
```

Run graph demos:

```bash
cargo run -p gpui-px --bin px-showcase --features gpui
cargo run -p gpui-px --example logscale_demo --features gpui
```

## 10. Drop Down to d3rs for Custom Visuals

Use `gpui-d3rs` for D3-style primitives such as scales, shapes, color,
hierarchy, force layout, geo projections, Delaunay/Voronoi, and custom GPU
rendering.

```rust
use d3rs::delaunay::Delaunay;
use d3rs::scale::LinearScale;

let scale = LinearScale::new()
    .domain([0.0, 100.0])
    .range([0.0, 600.0]);

let points = vec![(0.0, 0.0), (1.0, 0.0), (0.4, 0.8)];
let mesh = Delaunay::new(&points);
let nearest = mesh.find(0.5, 0.5, None);
```

Run:

```bash
cargo run -p gpui-d3rs --example d3rs-showcase --features "gpui,gpu-2d"
cargo run -p gpui-d3rs --example delaunay_demo
```

## 11. Register Stories in Component Lab

`gpui-component-lab` is the review surface for component states, responsive
viewports, theme presets, and design-token conformance.

Start with the built-in stories:

```bash
cargo run -p gpui-component-lab --bin gpui-component-lab
```

Generate conformance reports:

```bash
mkdir -p target/gpui-conformance
cargo run -p gpui-component-lab --bin gpui-component-lab -- \
  --conformance \
  --report-json target/gpui-conformance/component-lab.json \
  --report-markdown target/gpui-conformance/component-lab.md
```

When adding a component:

1. Add the component to `gpui-ui-kit`.
2. Add focused examples under `crates/gpui-ui-kit/examples/`.
3. Add or extend component-lab stories.
4. Run `just qa-gpui-conformance`.
5. Check the markdown report before merging visual changes.

## 12. Python Runtime Demo

This path builds the same counter with Python declarations. Python owns state
and handles events; a Rust/GPUI host draws the native desktop window. You do
not need to write Rust when using a compatible wheel with a bundled host.

### Install and create the app

Use the matching checkout's Python sources for this tutorial. The public
package index currently lists 0.9.30 as its latest release, while this checkout
is 0.10.44; do not silently mix those APIs. From the **checkout root**, create a
sibling directory, separate from the Rust project:

```bash
cd ..
mkdir toolkit-counter-python
cd toolkit-counter-python
python3 -m venv .venv
source .venv/bin/activate
export PYTHONPATH="$(cd ../gpui-toolkit && pwd)/crates/gpui-python-runtime/python"
python -c "import gpui_toolkit; print(gpui_toolkit.__version__)"
```

This makes the declarations importable without a package build. The final
command should print `0.10.44`. IR inspection works immediately; to open a
window, build the native host using [the source steps](#python-source-fallback)
after creating `counter.py` below.

When a matching wheel is available for your platform, you can instead install
`gpui-toolkit==0.10.44` in the environment and omit `PYTHONPATH`. Check host
availability with `python -c "from gpui_toolkit.app import _host_binary; print(_host_binary())"`.
This private helper is a diagnostic, not an API for application code. A path
means a host was found; `None` means you need the source-built host. Installing
declarations alone does not supply a renderer.

Create **`counter.py`** in this directory:

```python
from dataclasses import dataclass

from gpui_toolkit import App, Event, SessionContext, section, ui
from gpui_toolkit.miniapp import MiniAppConfig


@dataclass
class CounterApp(App):
    count: int = 0

    def on_action(self, event: Event, context: SessionContext) -> None:
        if event.action == "counter.increment":
            self.count += 1
            context.acknowledge(event)
            context.patch(
                [{"op": "set", "id": "count", "property": "text",
                  "value": f"Count: {self.count}"}],
                request_id=event.id,
            )


def build_app() -> CounterApp:
    return CounterApp(
        title="Toolkit Counter",
        miniapp=MiniAppConfig(
            title="Toolkit Counter", width=640.0, height=480.0,
            with_theme=True,
        ),
        sections=[section("counter", "Counter", ui.vstack([
            ui.text("Toolkit Counter"),
            ui.text("Count: 0", id="count"),
            ui.button("Increment", id="increment", action="counter.increment"),
        ], gap=16.0))],
    )


if __name__ == "__main__":
    build_app().run()
```

### Inspect, launch, and change state

From `toolkit-counter-python/`, with the virtual environment active:

```bash
GPUI_TOOLKIT_DUMP_IR=1 python counter.py
GPUI_PYTHON="$(command -v python)" python counter.py
```

The first command prints the UI's JSON description without opening a window.
Run the source-host build steps below before the second command unless you
already have a compatible host. The second opens a native window with a **Counter** sidebar section. Click
**Increment** three times and check **Count: 3**. Restart to reset to zero.
`GPUI_PYTHON` selects the interpreter the host uses for your script, so it can
find the same virtual environment's packages. On PowerShell, set it with
`$env:GPUI_PYTHON = (Get-Command python).Source`, then run `python counter.py`.

The button's action reaches `on_action`. Acknowledge the event, update Python
state, then patch the text node using its stable `id`. The patch's property
is `text`; `request_id=event.id` connects the update to the originating event.
To change the app, edit the heading or button label and restart the script.
For a larger dashboard with charts and a theme picker, continue with the
[Python dashboard tutorial](crates/gpui-python-runtime/python/tutorial/README.md).

### Python source fallback

For this source checkout, build the desktop host as follows. These steps also
apply when an installed wheel lacks a host. This requires Rust and your native platform tools.
From the **gpui-toolkit checkout root**, with your Python environment active:

```bash
export PYTHONPATH="$PWD/crates/gpui-python-runtime/python"
export GPUI_PYTHON="$(command -v python)"
cargo build -p gpui-python-runtime --features showcase --bin gpui-python-host
export GPUI_TOOLKIT_HOST="$PWD/target/debug/gpui-python-host"
python ../toolkit-counter-python/counter.py
```

Adjust the final script path if you created it elsewhere. On Windows the host
ends in `.exe`; on PowerShell use `$env:PYTHONPATH`, `$env:GPUI_PYTHON`, and
`$env:GPUI_TOOLKIT_HOST` assignments instead of `export`. `PYTHONPATH` deliberately
uses the matching checkout's declarations, so source-only UI validation can
also run before the host build succeeds.

## Build and distribute

A release build is the next step after the counter works. Build on each target
platform; a macOS executable is not a Windows or Linux executable.

### Rust desktop

From `toolkit-counter/`:

```bash
cargo build --release
./target/release/toolkit-counter
```

Windows output is `target/release/toolkit-counter.exe`. With `CARGO_TARGET_DIR`
or `--target`, use the corresponding output directory instead. Launch this
release version and repeat the three-click check before sharing it.

The generated project produces an executable, not an installer. For a local
handoff, copy that executable plus any app assets you later introduce and test
on a clean machine with the required graphics/runtime libraries. For end-user
distribution, create the platform package:

| Platform | Next packaging step |
| --- | --- |
| macOS | Put the executable in `Toolkit Counter.app/Contents/MacOS/`, add `Contents/Info.plist` with the matching executable name and bundle identifier, and include your resources. Sign the bundle and notarize it for public distribution; build a DMG/ZIP if desired. |
| Windows | Ship the executable and required runtime/assets, then use your installer tooling to create an MSI or MSIX and configure signing. |
| Linux | Ship the executable with documented runtime dependencies, or create an AppImage/deb/rpm with your packaging tooling; include a desktop entry and icon. |

The toolkit does not currently generate these desktop installers. Keep signing
identities, entitlements, icons, and store metadata in your application project.

### Python desktop

A working handoff consists of `counter.py`, matching `gpui_toolkit` declarations,
a compatible native host, and Python 3.10+. For the source route used here, build
the host with the fallback's Cargo command plus `--release`. Copy the script,
the checkout's `python/gpui_toolkit/` directory (under `crates/gpui-python-runtime/`),
and `target/release/gpui-python-host` (or `.exe`) to the destination. Set
`PYTHONPATH` to the parent of that copied package, `GPUI_TOOLKIT_HOST` to the
host's absolute installed path, and `GPUI_PYTHON` to the destination interpreter.
Then launch the script. Check native libraries are available on a clean machine.

Once a matching platform wheel is published, it provides an alternative to
copying the package and host separately. For an offline wheel handoff, download
it on the intended platform (this command requires that release to exist):

```bash
python -m pip download --only-binary=:all: --dest wheelhouse gpui-toolkit==0.10.44
```

Copy `counter.py` and `wheelhouse/` to the destination. There, in a new activated
virtual environment:

```bash
python -m pip install --no-index --find-links wheelhouse gpui-toolkit==0.10.44
GPUI_PYTHON="$(command -v python)" python counter.py
```

Verify the wheel includes a host before shipping it. If it does not, build the
host from the matching checkout using the fallback's Cargo command with
`--release`, include `target/release/gpui-python-host` (or `.exe`), and set
`GPUI_TOOLKIT_HOST` to its absolute installed path. Native libraries must also
be available on the destination. A wheel or Rust extension by itself is not a
standalone application installer. A one-click app additionally needs a bundled
Python runtime, a launcher, assets, and platform packaging/signing; no automatic
Python application packager is supplied by this tutorial.

## 13. Mobile Extension

The Rust counter shares `src/app.rs` with its mobile hosts. These builds need
platform SDKs and signing setup in addition to Rust. The desktop Python host
does **not** package a Python interpreter for iOS or Android; use the Rust path
for these generated mobile applications.

Run the commands below from `toolkit-counter/`. For iOS, install full Xcode,
XcodeGen, and `just`, and select your Xcode developer directory.

Scaffolded apps include an `ios/` XcodeGen host with `AppDelegate.swift`,
`BridgingHeader.h`, `Info.plist`, and `Entitlements.plist`. Build the iOS
simulator and device apps:

```bash
just ios-sim
just ios
```

`just ios-sim` compiles the Rust static library, stages it under `ios/lib/`,
generates the Xcode project, and runs `xcodebuild` for the simulator. `just ios`
does the same for a device build. Set signing values when building for hardware:

```bash
IOS_DEVELOPMENT_TEAM=ABCDE12345 IOS_SIGN_IDENTITY="Apple Development: Your Name" just ios
```

To sign an already-built device app with the scaffolded entitlements:

```bash
IOS_SIGN_IDENTITY="Apple Development: Your Name" just ios-sign
```

Build the tvOS simulator and device artifacts:

```bash
just tvos-sim
just tvos
```

The tvOS recipes compile `src/lib.rs` to a static library under
`target/mobile/...`. Link that library from your tvOS host and call the
generated `<app_name>_ios_start` symbol. See
[crates/gpui-ios/README.md](./crates/gpui-ios/README.md) and
[crates/gpui-showcase/ios/TUTORIAL.md](./crates/gpui-showcase/ios/TUTORIAL.md)
for full Swift/Xcode host steps.

### Launch the iOS simulator app

`just ios-sim` builds the app but does not open it. On an Apple Silicon Mac,
boot an iOS simulator in Xcode, then run from `toolkit-counter/`:

```bash
xcrun simctl install booted "ios/build/Release-iphonesimulator/ToolkitCounter.app"
xcrun simctl launch booted com.example.toolkit.counter
```

The scaffold's simulator recipe targets ARM64. For Intel Macs, adapt the Rust
target and Xcode architecture before using it. Device builds appear under
`ios/build/Release-iphoneos/`; an App Store/TestFlight release additionally needs
an Xcode archive, provisioning, and export. A built `.app` is not an App Store
submission.

### Build and launch Android

Install a JDK, Gradle, the Android SDK/NDK, and `just`. Configure the SDK location
and an NDK linker for Rust's `aarch64-linux-android` target (for example through
`.cargo/config.toml` and the installed NDK toolchain). The generated recipe uses
plain Cargo, so it needs that linker configuration before it can build.
From `toolkit-counter/`:

```bash
just android-apk
adb install -r android/gradle/app/build/outputs/apk/debug/app-debug.apk
adb shell monkey -p com.example.toolkit.counter -c android.intent.category.LAUNCHER 1
```

Use an ARM64 device/emulator for this scaffold. `just android-apk` builds and
stages `libtoolkit_counter.so`, then Gradle creates the debug APK. Install and
launch it to check the counter. Public distribution requires a release signing
configuration and an appropriately signed APK/AAB; the debug APK is for local
testing. Adjust bundle/application identifiers before publishing your own app.

## 14. Development Loop

Use this loop for most feature work:

1. Generate a focused app with `gpui-scaffolder`.
2. Prototype in the scaffolded MiniApp or a workspace example.
3. Extract stable UI into `gpui-ui-kit`, `gpui-audio-kit`, or `gpui-px`.
4. Add responsive layout constraints in `gpui-builder` if the layout is shared.
5. Add design tokens or conformance expectations in `gpui-design`.
6. Add component-lab coverage.
7. Run `just examples`, then `just qa-gpui-obvious`.
8. Use iOS/tvOS recipes only after desktop checks are green.

## 15. Drive It From the CLI

The `gpui-toolkit` binary wraps the catalog, templates, themes, health
checks, and layout tools in one place. All examples below run from the
workspace root; substitute the installed binary once you run
`cargo install --path crates/gpui-toolkit-cli`. Append `--json` to any
command for a typed `{apiVersion, type, data}` envelope, or `--dense`
for token-efficient output.

Bootstrap a project's agent docs with the component catalog:

```bash
cargo run -p gpui-toolkit-cli -- init --dir ../toolkit-counter
```

Look up components instead of guessing prop names:

```bash
cargo run -p gpui-toolkit-cli -- component --detail compact
cargo run -p gpui-toolkit-cli -- component ui-kit.button --props
cargo run -p gpui-toolkit-cli -- component ui-kit.button ui-kit.input --json
cargo run -p gpui-toolkit-cli -- search button --limit 5
```

Start pages from templates rather than from scratch:

```bash
cargo run -p gpui-toolkit-cli -- template
cargo run -p gpui-toolkit-cli -- template --all
cargo run -p gpui-toolkit-cli -- template settings-page --skeleton
cargo run -p gpui-toolkit-cli -- template settings-page --out ../toolkit-counter/src
cargo run -p gpui-toolkit-cli -- template theme-custom --out ../toolkit-counter/src
```

Export theme tokens and keep them fresh in CI:

```bash
cargo run -p gpui-toolkit-cli -- theme list
cargo run -p gpui-toolkit-cli -- theme build dark --out ../toolkit-counter/tokens
cargo run -p gpui-toolkit-cli -- theme build dark --out ../toolkit-counter/tokens --check
cargo run -p gpui-toolkit-cli -- theme targets dark --dense | head
```

Check the tree before trusting it, and report gaps when the toolkit is
missing something:

```bash
cargo run -p gpui-toolkit-cli -- doctor
cargo run -p gpui-toolkit-cli -- upgrade
cargo run -p gpui-toolkit-cli -- upgrade --detect --dir ../toolkit-counter/src
cargo run -p gpui-toolkit-cli -- gap-report --area component DateRangePicker
```

Sketch layouts as one-line expressions, then expand them to Rust:

```bash
cargo run -p gpui-toolkit-cli -- layout check 'V > (Tx"Hi" + B.primary"Go")'
cargo run -p gpui-toolkit-cli -- layout expand 'V > (Tx"Hi" + B.primary"Go")' --function hero
cargo run -p gpui-toolkit-cli -- layout grammar
```

The syntax follows Astryx XLE conventions: text size is a modifier
(`Tx.lg"$42k"`), heading level an attr (`Hd"Analytics"[level=2]`), and
stack gaps use fused `gN` attrs (`V[g6]`, N×4px, alongside the explicit
`spacing=` form). Text also takes `weight` and `muted`, so a styled
header reads:

```bash
cargo run -p gpui-toolkit-cli -- layout check 'V[g6] > (Hd"Analytics"[level=2] + H[g2] > Tx.lg"$42k")'
```

Teach the CLI your project's vocabulary with a `toolkit.toml` file.
Custom layout components behave like built-ins in `layout check` and
`layout expand`, `issues_url` retargets `gap-report`, and extra
`upgrade` rules extend `--detect`. Unknown tables and keys fail
loudly, and `doctor` reports config health:

```toml
[project]
issues_url = "https://github.com/acme/toolkit-counter/issues"

[[layout.components]]
name = "PrimaryButton"
base = "B"
modifier = "primary"

[[upgrade.rules]]
id = "no-legacy-color"
pattern = "legacy_color("
message = "Use theme tokens instead of legacy_color."
```

```bash
cargo run -p gpui-toolkit-cli -- layout check 'PrimaryButton"Save"'
cargo run -p gpui-toolkit-cli -- doctor --dir ../toolkit-counter
```

The same expressions work at compile time through the `layout!` macro
from `gpui-ui-kit-macros`, with unknown components failing the build
(the macro stays built-in-only: it cannot see `toolkit.toml`):

```rust
use gpui_ui_kit_macros::layout;

let _hero = layout! {
    V {
        Tx "Hi",
        B.primary("Go"),
    }
};

let _styled = layout! {
    V[g6] {
        Hd "Analytics" [level = "2"],
        H[g2] {
            Tx.lg "$42k",
            Tx.sm "Revenue" [weight = "bold", muted],
        },
    }
};
```

What the macro saves, side by side. With `layout!`:

```rust
let _hero = layout! {
    V {
        Tx "Hi",
        B.primary("Go"),
    }
};
```

Without (the same tree `layout expand` emits for
`V > (Tx"Hi" + B.primary"Go")`, wrapped for readability):

```rust
use gpui::div;
use gpui_ui_kit::{Button, ButtonVariant, VStack};

let _hero = VStack::new()
    .child(div().child("Hi"))
    .child(Button::new("layout-0", "Go").variant(ButtonVariant::Primary));
```

The macro form drops the imports, the constructor names, the auto id,
and the closing-paren accounting; on invalid input the build fails at
the offending token instead of mid-frame.

When a stock component almost fits, eject its source into your tree and
customize it there:

```bash
cargo run -p gpui-toolkit-cli -- eject Button --into ../toolkit-counter/src/vendor
```


## Agent workflow

When asked to build an app, deliver a runnable project and an exact launch
command. Use this sequence from the checkout root; replace the example
project path with the user's destination. Follow the repository's own
approval and contribution rules.

1. Establish the host OS, intended targets, Rust or Python preference, and
   native prerequisites. Start with a desktop interaction before mobile.
2. Discover the CLI contract and real component APIs:

   ```bash
   cargo run -p gpui-toolkit-cli -- manifest --json
   cargo run -p gpui-toolkit-cli -- search button --limit 5 --json
   cargo run -p gpui-toolkit-cli -- component ui-kit.button --props --json
   cargo run -p gpui-toolkit-cli -- template settings-page --skeleton --json
   ```

   Parse the `{apiVersion, type, data}` envelope and the manifest's response
   types. Branch on `type` and `ERR_*` codes, not prose. Success exits 0,
   operational failures exit 1, and CLI usage errors exit 2. A page template
   is a starting fragment; inspect it before integrating it into the app.
3. For Rust, preview and generate the project as in section 1. Align the
   GPUI dependency with this checkout and preserve the generated exports.
   For Python, create the script and confirm native-host availability as
   in section 12; the Rust scaffolder does not generate Python projects.
4. Once a project directory exists, initialize its agent catalog:

   ```bash
   cargo run -p gpui-toolkit-cli -- init --dir ../toolkit-counter --json
   ```

   This writes a managed catalog block in the project's `AGENTS.md`; it
   preserves text outside the block. It does not create the runnable app.
   For Python, pass the Python project directory instead.
5. Implement one observable interaction using the documented APIs. Check
   from the generated Rust directory with `cargo check`, or inspect Python
   with `GPUI_TOOLKIT_DUMP_IR=1 python counter.py`. Launch the native window
   and exercise the interaction; JSON output alone does not prove rendering.
6. From the toolkit checkout, run diagnostics when investigating toolkit
   health, and inspect the typed result and exit code:

   ```bash
   cargo run -p gpui-toolkit-cli -- doctor --json
   ```

   `doctor` diagnoses this toolkit checkout; use the generated application's
   own Cargo commands to verify the application.
7. Build the release artifact using the distribution section. Report the
   working directory, exact launch command, resulting artifact paths, and
   what was actually verified. Name any missing SDK or host and the next
   action needed. Distinguish a release executable, a debug APK, and a signed
   installer; do not report one as another.

A useful agent request is:

> Build Toolkit Counter using this checkout. Use the tutorial's Rust (or
> Python) path, implement the Increment button, verify three clicks change
> Count: 0 to Count: 3, and give me the command to run it and the release
> artifact location. Explain any native build prerequisite that prevents
> verification.

## Troubleshooting the first launch

| Symptom | Check and next action |
| --- | --- |
| `xcrun` cannot find `metal` | Install full Xcode and its Metal toolchain, select that Xcode, and rerun the failed Cargo command. |
| Rust reports incompatible GPUI types | Check that the generated app uses the same renamed GPUI dependency as the toolkit's workspace manifest. |
| Python reports no native host | Follow the source fallback or install a compatible wheel containing the host. Point `GPUI_TOOLKIT_HOST` to an executable, not a directory. |
| Host cannot import `gpui_toolkit` | Set `GPUI_PYTHON` to the environment's interpreter; for the source fallback also set `PYTHONPATH` to the checkout's Python directory. |
| Python produces JSON but no window | Remove `GPUI_TOOLKIT_DUMP_IR=1` for the launch command. |
| Desktop runs but mobile will not build | Check platform SDKs, target architecture, signing, and the Android NDK linker configuration before changing shared UI code. |
