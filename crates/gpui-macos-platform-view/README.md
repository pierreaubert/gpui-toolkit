# gpui-macos-platform-view

Embed AppKit `NSView`s (WebKit, AVPlayer, vendor SDK views) next to GPUI
content on macOS, M1 scope:

- `ViewHandle` — an opaque, non-owning wrapper around an `NSView*`.
- `MacPlatformView` — a GPUI placeholder element that records its laid-out
  frame each paint.
- `snapshot_frames()` — back-to-front frames for the host's
  follows-the-view loop (plus a JSON C ABI for Swift/ObjC hosts, see
  `include/gpui_macos_platform_view.h`).
- `VerticalStack` / `vertical_stack_offsets` — fixed-height section
  stacking for settings/editor-style UIs.

```swift
// Host side, once per display-link tick:
let json = String(cString: gpui_macos_platform_views_snapshot_json())
// decode, then for each entry set the matching NSView's frame,
// converting from GPUI's top-left origin with the host view's height.
```

The native view is an overlaid sibling in the same window: add it above
the GPUI view once, then keep its frame in sync from the snapshot. There
is deliberately no GPUI-owned host area, lifecycle, or input routing in
M1 — Cocoa first-responder rules already route the keyboard to a clicked
native view. Cutting a hole in GPUI's own rendering (so GPUI chrome can
overlap native content) is future work.
