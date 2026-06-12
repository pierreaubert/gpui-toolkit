# gpui-au

macOS Audio Unit platform backend for GPUI — embeds GPUI rendering inside AUv3 ViewControllers via Metal/wgpu.

## What It Does

If you're building an Audio Unit (AUv3) plugin with a custom UI, this crate lets you use GPUI as the rendering engine inside your AUViewController. Instead of creating its own window, it renders into an NSView provided by the AU host (Logic Pro, GarageBand, etc.) using a Metal-backed CAMetalLayer.

## Features

- **NSView embedding**: Renders into an external NSView from AUViewController
- **Metal/wgpu rendering**: Hardware-accelerated UI via CAMetalLayer
- **Event forwarding**: Mouse and keyboard events forwarded from NSView to GPUI
- **CoreText integration**: Native text rendering via CoreText
- **Display-driven rendering**: Frame updates driven by Swift CVDisplayLink/timer

## How It Works

```
Swift AUViewController
  └── NSView (host-provided)
       └── CAMetalLayer
            └── wgpu (Metal backend)
                 └── GPUI rendering
```

1. Swift creates an AUViewController with an NSView
2. Rust-side `AuPlatform` initializes GPUI with the external NSView
3. Swift calls `gpui_au_request_frame()` on each display refresh
4. Mouse/keyboard events forwarded from NSView → FFI → GPUI

## Architecture

```
src/
├── lib.rs          # Module exports
├── platform.rs     # AuPlatform — GPUI Platform trait impl
├── window.rs       # AuWindow — NSView wrapper + Metal rendering
├── display.rs      # AuDisplay — display identification
├── dispatcher.rs   # AuDispatcher — thread dispatch
├── text_system.rs  # AuTextSystem — CoreText text rendering
├── ffi.rs          # FFI entry points for Swift
└── helpers.rs      # Objective-C runtime helpers
```

## Requirements

- macOS only
- Requires GPUI framework
- Metal-capable GPU

## Testing

```bash
cargo check -p gpui-au
```

## License

Part of the SOTF (Sound of the Future) project.
