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

## Host FFI

The canonical C declarations are in [`include/gpui_au.h`](include/gpui_au.h).
Import that header in the AU target's bridging header instead of duplicating
Rust ABI declarations in Swift.

Forward `NSEvent` key events and `NSTextInputClient` callbacks separately:
key events drive shortcuts and navigation, while text callbacks carry committed
or marked Unicode text.

```swift
override func keyDown(with event: NSEvent) {
    let utf8 = (event.characters ?? "").utf8CString
    utf8.withUnsafeBufferPointer { characters in
        gpui_au_key_down(
            context,
            event.keyCode,
            characters.baseAddress,
            UInt32(truncatingIfNeeded: event.modifierFlags.rawValue),
            event.isARepeat
        )
    }
}

func insertText(_ value: Any, replacementRange: NSRange) {
    let text = (value as? NSAttributedString)?.string ?? String(describing: value)
    text.withCString { gpui_au_insert_text(context, $0) }
}

func setMarkedText(
    _ value: Any,
    selectedRange: NSRange,
    replacementRange: NSRange
) {
    let text = (value as? NSAttributedString)?.string ?? String(describing: value)
    text.withCString {
        gpui_au_set_marked_text(
            context,
            $0,
            selectedRange.location,
            selectedRange.length
        )
    }
}

func unmarkText() {
    gpui_au_unmark_text(context)
}
```

## Testing

```bash
cargo test -p gpui-au
clang -fsyntax-only -x c include/gpui_au.h
```

## License

Part of the SOTF (Sound of the Future) project.
