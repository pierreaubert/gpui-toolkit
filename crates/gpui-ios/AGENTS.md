# gpui-ios - Agent's Guide

Quick reference for AI agents working with the iOS platform backend.

## Crate Structure

```
gpui-ios/
  src/
    lib.rs              # Public API: safe_area_insets(), show/hide_keyboard(), text input
    momentum.rs         # Momentum scrolling physics engine
    platform_view.rs    # Platform view factory
    ios/
      mod.rs            # IosPlatform + current_platform() (cfg target_os = "ios")
      ffi.rs            # C FFI exports: set_app_callback, run_app, request_frame, etc.
      platform.rs       # IosPlatform: Platform trait implementation
      window.rs         # IosWindow: Metal layer, touch, safe areas (~65KB, largest file)
      text_system.rs    # CoreText text shaping/rendering (~27KB)
      text_input.rs     # Software keyboard input handling
      display.rs        # Screen/display information
      dispatcher.rs     # Task dispatching
      events.rs         # Touch → GPUI event conversion
```

## Key Patterns

### The ios module is cfg-gated

```rust
#[cfg(target_os = "ios")]
pub mod ios;
```

This means `gpui_ios::ios::*` only compiles when targeting iOS. Code that references `gpui_ios::ios::ffi::set_app_callback()` will fail on macOS host. This is by design — the staticlib entry points only make sense on iOS.

### FFI entry points (ffi.rs)

All Swift-callable functions are `#[unsafe(no_mangle)] pub extern "C"`:

| Function | Called by | Purpose |
|----------|-----------|---------|
| `set_app_callback(Box<...>)` | Rust entry point | Register the GPUI app setup closure |
| `run_app()` | Rust entry point | Initialize platform and start GPUI |
| `gpui_ios_request_frame(ptr)` | CADisplayLink | Pump render loop each frame |
| `gpui_ios_get_window() -> ptr` | Swift | Get active GPUI window handle |
| `gpui_ios_handle_touch(...)` | UIKit | Forward touch events |
| `gpui_ios_will_enter_foreground(ptr)` | UIKit lifecycle | App entering foreground |
| `gpui_ios_did_become_active(ptr)` | UIKit lifecycle | App became active |
| `gpui_ios_will_resign_active(ptr)` | UIKit lifecycle | App resigning active |
| `gpui_ios_did_enter_background(ptr)` | UIKit lifecycle | App entering background |
| `gpui_ios_will_terminate(ptr)` | UIKit lifecycle | App terminating |

### Window management (window.rs)

`IosWindow` creates a `CAMetalLayer` and uses `gpui_wgpu` for Metal rendering. Touch events from UIKit are converted to GPUI `MouseDownEvent`/`MouseMoveEvent`/`MouseUpEvent` with synthesized scroll wheel events for panning.

### Text system (text_system.rs)

Uses CoreText directly (not UIKit text). Font enumeration, shaping, glyph rasterization all go through CoreText C API via `core-text` crate.

## Building

```bash
# Check (fast, no linking)
cargo check -p gpui-ios --target aarch64-apple-ios-sim

# Build rlib (used by other crates)
cargo build -p gpui-ios --target aarch64-apple-ios-sim --release

# This crate CANNOT be checked on macOS host (ios module is cfg-gated)
# cargo check -p gpui-ios  # Only checks the non-ios parts (lib.rs, momentum.rs)
```

## Dependencies

- `gpui` (workspace) — Core GPUI framework
- `gpui_wgpu` (Zed git) — Metal/wgpu renderer
- `objc` — Objective-C runtime FFI
- `core-foundation`, `core-graphics`, `core-text` — Apple framework bindings
- `font-kit` (Zed fork) — Font loading and enumeration

## Common Tasks

### Adding a new FFI function

1. Add `#[unsafe(no_mangle)] pub extern "C" fn gpui_ios_my_func(...)` in `ffi.rs`
2. Add declaration in the Swift bridging header: `void gpui_ios_my_func(...);`
3. Call from Swift in the appropriate lifecycle method

### Handling a new UIKit event

1. Create FFI function in `ffi.rs` to receive the event from Swift
2. Convert to a GPUI event type in `events.rs`
3. Dispatch through `IosWindow` to GPUI's event system

### Debugging on simulator

The crate uses `oslog` for logging. View logs with:
```bash
xcrun simctl spawn booted log stream --predicate 'subsystem == "org.spinorama.sotf"' --level debug
```

## Reference

- [gpui-mobile](https://github.com/itsbalamurali/gpui-mobile) — Original upstream
- [Zed GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) — Core framework
- Showcase app: `../gpui-ui-kit/ios/` — Working iOS GPUI app example
- SotF iOS app: `../../app-ios/` — Full music player iOS app
