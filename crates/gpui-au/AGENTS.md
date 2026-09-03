# gpui-au

macOS Audio Unit platform backend — embeds GPUI rendering inside AUv3 ViewControllers via Metal/wgpu.

## Architecture

macOS-only (`#[cfg(target_os = "macos")]`). Follows the same pattern as `gpui-ios` but adapted for NSView embedding.

- `platform.rs` — `AuPlatform`: implements GPUI `Platform` trait for AU context
- `window.rs` — `AuWindow`: wraps an external NSView (from AUViewController), renders via CAMetalLayer + wgpu. Exports `PENDING_VIEW` and `PendingViewInfo`
- `display.rs` — `AuDisplay`: display identification for the AU host
- `dispatcher.rs` — `AuDispatcher`: thread dispatch for GPUI event loop
- `text_system.rs` — `AuTextSystem`: CoreText-based text rendering
- `ffi.rs` — FFI functions called from Swift: `gpui_au_request_frame()`, mouse/keyboard event forwarding, `gpui_au_parameter_*` / state save-load
- `params.rs` — minimal `AUParameterTree` + `fullState` bridge (atomic values, observer tokens, versioned byte encoding)
- `helpers.rs` — Objective-C runtime helpers (`nslog` for failures, `nslog_verbose` for progress; `verbose-logging` feature forces verbose in release)

## Key Public API

- `AuPlatform` — GPUI Platform implementation for AU context (`platform.rs`)
- `PENDING_VIEW: Mutex<Option<PendingViewInfo>>` — shared state for Swift ↔ Rust view handoff (`window.rs`)
- `ffi::gpui_au_request_frame()` — called from Swift CVDisplayLink/timer to drive rendering

## Testing

```bash
cargo check -p gpui-au
```

## Important Notes

- macOS-only crate — will not compile on other platforms
- Frame rendering is driven externally by Swift via `gpui_au_request_frame()`
- Takes an external NSView instead of creating its own window
- Mouse/keyboard events forwarded from NSView → FFI → GPUI window
- Logging goes to stderr (visible in Console.app for AU extensions)
- Uses `#![allow(clippy::not_unsafe_ptr_arg_deref)]` for FFI pointer handling
