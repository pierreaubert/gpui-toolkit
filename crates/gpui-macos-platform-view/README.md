# gpui-macos-platform-view

Embed AppKit `NSView`s (WebKit, AVPlayer, vendor SDK views) next to GPUI
content on macOS:

- `ViewHandle` — an opaque, non-owning wrapper around an `NSView*`.
- `MacPlatformView` — a GPUI placeholder element that records its laid-out
  frame (and window scale factor) each paint.
- `register_view` / `unregister_view` — pair placeholder ids with native
  views; unmounting forgets the recorded frame. `prune_stale_frames`
  expires placeholders that stopped painting.
- `insert_below_gpui` / `insert_above_gpui` — put the native view under
  GPUI content (video/3D revealed through a transparent GPUI region) or
  over it (interactive views that take clicks directly).
- `apply_snapshot(window)` — the follows-the-view loop in one call: move
  every registered view to its placeholder's frame. `snapshot_frames()`
  plus a JSON C ABI (`include/gpui_macos_platform_view.h`) remains for
  Swift/ObjC hosts driving frames themselves.
- `ns_window(window)` / `window_number(window)` — reach the hosting
  `NSWindow`; `make_first_responder` hands it the keyboard focus.
- `VerticalStack` / `vertical_stack_offsets` — fixed-height section
  stacking for settings/editor-style UIs.

```rust
// Once, when the window opens:
register_view("video", handle);
insert_below_gpui(window, handle);
// Once per host frame tick:
apply_snapshot(window);
```

Below-GPUI views need a hole to show through: create the window with
`WindowBackgroundAppearance::Transparent` and paint no GPUI background
over the placeholder. Pointer events land on the topmost sibling, so
GPUI receives them over below-GPUI views — below hosting is for display
plus programmatic focus (`make_first_responder`); interactive native
views belong above. See `examples/below_gpui.rs` for a runnable,
self-verifying demo (red `NSView` under transparent GPUI chrome).
