# gpui-ios

iOS platform backend for GPUI. Enables any GPUI application to run natively on iOS with Metal rendering, touch input, momentum scrolling, and software keyboard support.

Vendored from [gpui-mobile](https://github.com/itsbalamurali/gpui-mobile) and adapted to work with Zed's GPUI revision `dd9efd9`.

## How It Works

GPUI apps on iOS follow a hybrid architecture:

```
Swift AppDelegate
  |-- sotf_ios_start()          # Rust FFI: registers app callback + run_app()
  |-- CADisplayLink             # Drives gpui_ios_request_frame() every frame
  |-- UIKit lifecycle events    # Forwarded to Rust via FFI

Rust (staticlib)
  |-- set_app_callback()        # Register GPUI app setup (theme, window, views)
  |-- run_app()                 # Initialize GPUI platform, start event loop
  |-- IosPlatform              # Implements gpui::Platform for iOS
  |-- IosWindow                # Metal rendering, touch dispatch, safe areas
```

The Rust code compiles to a static library (`.a`) that is force-loaded into the Swift iOS app. A `CADisplayLink` on the Swift side calls `gpui_ios_request_frame()` every frame, which pumps the GPUI render loop.

## Porting a GPUI App to iOS

### Step 1: Create a staticlib crate

```toml
# crates/my-app-ios/Cargo.toml
[package]
name = "my-app-ios"
edition = "2024"

[lib]
name = "my_app_ios"
crate-type = ["staticlib"]

[dependencies]
gpui = { workspace = true }
gpui-ios = { workspace = true }
log = "0.4"
oslog = "0.2"
```

### Step 2: Write the entry point

```rust
// crates/my-app-ios/src/lib.rs
use gpui::*;

#[unsafe(no_mangle)]
pub extern "C" fn my_app_ios_start() {
    oslog::OsLogger::new("com.example.myapp")
        .level_filter(log::LevelFilter::Info)
        .init()
        .ok();

    gpui_ios::ios::ffi::set_app_callback(Box::new(|cx: &mut App| {
        cx.open_window(
            WindowOptions { window_bounds: None, ..Default::default() },
            |_, cx| cx.new(|cx| MyView::new(cx)),
        ).expect("Failed to open window");
        cx.activate(true);
    }));

    gpui_ios::ios::ffi::run_app();
}
```

### Step 3: Create the Swift iOS app

Create `ios/SotFApp/AppDelegate.swift`:

```swift
import UIKit

@main
class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    private var displayLink: CADisplayLink?

    func application(_ application: UIApplication,
                     didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        my_app_ios_start()
        displayLink = CADisplayLink(target: self, selector: #selector(renderFrame))
        displayLink?.add(to: .main, forMode: .common)
        return true
    }

    @objc private func renderFrame() {
        let win = gpui_ios_get_window()
        if win != nil { gpui_ios_request_frame(win) }
    }

    func applicationWillEnterForeground(_ application: UIApplication) { gpui_ios_will_enter_foreground(nil) }
    func applicationDidBecomeActive(_ application: UIApplication) { gpui_ios_did_become_active(nil) }
    func applicationWillResignActive(_ application: UIApplication) { gpui_ios_will_resign_active(nil) }
    func applicationDidEnterBackground(_ application: UIApplication) { gpui_ios_did_enter_background(nil) }
    func applicationWillTerminate(_ application: UIApplication) { gpui_ios_will_terminate(nil) }
}
```

Create `ios/SotFApp/BridgingHeader.h`:

```c
#ifndef BridgingHeader_h
#define BridgingHeader_h
#include <stdbool.h>

void my_app_ios_start(void);
void gpui_ios_request_frame(void *window_ptr);
void *gpui_ios_get_window(void);
void gpui_ios_handle_touch(void *window_ptr, void *touch_ptr, void *event_ptr);
void gpui_ios_will_enter_foreground(void *app_ptr);
void gpui_ios_did_become_active(void *app_ptr);
void gpui_ios_will_resign_active(void *app_ptr);
void gpui_ios_did_enter_background(void *app_ptr);
void gpui_ios_will_terminate(void *app_ptr);

#endif
```

### Step 4: Build and run

```bash
# Install iOS targets
rustup target add aarch64-apple-ios-sim aarch64-apple-ios

# Build the static library for simulator
cargo build -p my-app-ios --target aarch64-apple-ios-sim --release

# Copy to Xcode project
cp target/aarch64-apple-ios-sim/release/libmy_app_ios.a ios/lib/

# Generate Xcode project (requires xcodegen: brew install xcodegen)
cd ios && xcodegen generate

# Build with Xcode
xcodebuild -project MyApp.xcodeproj -scheme MyApp \
  -sdk iphonesimulator -arch arm64 build

# For device builds, use aarch64-apple-ios target instead
cargo build -p my-app-ios --target aarch64-apple-ios --release
```

### Step 5: Xcode project configuration (project.yml)

```yaml
name: MyApp
options:
  bundleIdPrefix: com.example
  deploymentTarget:
    iOS: "15.0"

settings:
  base:
    SWIFT_OBJC_BRIDGING_HEADER: SotFApp/BridgingHeader.h
    EXCLUDED_ARCHS[sdk=iphonesimulator*]: x86_64

targets:
  MyApp:
    type: application
    platform: iOS
    sources: [SotFApp]
    settings:
      base:
        LIBRARY_SEARCH_PATHS: ["$(PROJECT_DIR)/lib"]
        DEAD_CODE_STRIPPING: false
        OTHER_LDFLAGS:
          - "-force_load"
          - "$(PROJECT_DIR)/lib/libmy_app_ios.a"
          - "-framework Metal"
          - "-framework MetalKit"
          - "-framework QuartzCore"
          - "-framework CoreText"
          - "-framework CoreGraphics"
          - "-framework CoreFoundation"
          - "-framework UIKit"
          - "-framework Foundation"
          - "-lc++"
```

## Platform APIs

### Safe area insets

```rust
let (top, left, bottom, right) = gpui_ios::safe_area_insets();
// Apply as padding to root view
```

### Scene metrics

```rust
if let Some(metrics) = gpui_ios::scene_metrics() {
    let (content_width, content_height) = metrics.content_size();
    let landscape_like = metrics.is_landscape_like();
    let split_view_like = metrics.is_split_view_like();
    let scene_class = metrics.scene_class();
}
```

Use scene metrics for iPad layout decisions. They are derived from the actual
UIKit view bounds, not physical device orientation, so they also work in Split
View and Stage Manager.

### Native bridge completion APIs

`gpui-ios` exposes platform bridge surfaces for production iOS shells:

- `gpui_ios::platform_view` registers Swift/UIKit factories and tracks native
  view bounds, visibility, z-order, hit testing, and disposal.
- `gpui_ios::accessibility::set_accessibility_snapshot(...)` publishes a GPUI
  accessibility snapshot for the UIKit bridge to mirror into VoiceOver.
- `gpui_ios::pencil::{set_pencil_event_callback, set_hover_event_callback}`
  exposes Apple Pencil pressure/tilt and hover side-channel data while the
  normal GPUI mouse/touch compatibility events continue to fire.
- `gpui_ios::widget::render_widget_snapshot(...)` writes WidgetKit/Live
  Activity snapshot image bytes plus timeline metadata to an App Group folder.
- `gpui_ios::{begin_metal_capture, end_metal_capture}` and
  `gpui_ios::instrumentation::emit_signpost(...)` expose debug hooks for
  Instruments-oriented tracing.
- `gpui_ios::hot_reload::HotReloadManifest` defines the simulator debug dylib
  reload manifest consumed by Swift shells.

Swift-facing C FFI is provided for host-view attachment, native platform-view
factory registration, Pencil hover forwarding, and Metal capture control.

### Software keyboard

```rust
gpui_ios::show_keyboard();
gpui_ios::hide_keyboard();
let height = gpui_ios::keyboard_height(); // in points
```

### Text input

```rust
gpui_ios::set_text_input_callback(Some(Box::new(|text: &str| {
    // Handle text input from software keyboard
})));
```

## Architecture

| Module | Purpose |
|--------|---------|
| `ios/platform.rs` | `IosPlatform` — implements `gpui::Platform` trait |
| `ios/window.rs` | `IosWindow` — Metal layer, touch dispatch, safe areas |
| `ios/ffi.rs` | C FFI exports called from Swift (frame rendering, lifecycle, touch) |
| `ios/text_system.rs` | CoreText-based text shaping and rendering |
| `ios/text_input.rs` | Software keyboard text input handling |
| `ios/display.rs` | Display/screen information |
| `ios/dispatcher.rs` | Task dispatching for async operations |
| `ios/events.rs` | Touch event conversion to GPUI events |
| `momentum.rs` | Momentum scrolling physics |
| `platform_view.rs` | Platform view factories |

## Known Limitations

- iOS only supports `aarch64` (arm64). No x86_64 simulator builds.
- Sample rate changes at runtime are not supported on iOS CoreAudio.
- File system access is sandboxed. Use `UIDocumentPickerViewController` for user file selection.
- No menu bar or title bar — iOS apps are fullscreen.
