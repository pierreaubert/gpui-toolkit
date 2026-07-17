# Vendored: gpui_macos

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/gpui_macos
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

### Remove private CGS symbols (Mac App Store static-analysis rejection risk)

Upstream `src/window.rs` declares the private Core Graphics Services symbols
`CGSMainConnectionID` and `CGSSetWindowBackgroundBlurRadius` and calls them for
legacy pre-Monterey window blur. SotF requires macOS 13+, so that path is
unreachable (guarded by `NSAppKitVersionNumber < NSAppKitVersionNumber12_0`),
but Apple's static analyzer can reject binaries that merely reference the
symbols. Removed regions (line numbers in the pristine v1.9.0 file, pre-patch):

- `src/window.rs:12` — dropped `NSAppKitVersionNumber, NSAppKitVersionNumber12_0,`
  from the `cocoa::appkit` import list (only used by the legacy branch).
- `src/window.rs:118-127` — dropped the entire
  `#[link(name = "CoreGraphics", kind = "framework")] unsafe extern "C"` block
  declaring `CGSMainConnectionID` and `CGSSetWindowBackgroundBlurRadius`.
- `src/window.rs:1502-1538` — dropped the
  `if NSAppKitVersionNumber < NSAppKitVersionNumber12_0 { … CGS call … } else { … }`
  branch in `set_background_appearance`; the `NSVisualEffectView` path (former
  `else` arm) is now unconditional and carries an explanatory comment.

History note: an earlier hand-refactored snapshot of this crate (which carried
the same patch but no longer compiles against gpui v1.9.0) is fully committed
in git history — recover it at commit e1beddc's parent
(`git checkout e1beddc~1 -- crates/3rdparties/gpui_macos`).

### Crate-root lint allows (clippy default lints, upstream code unchanged)

Added at the top of `src/gpui_macos.rs` (Task 6, `just lint-host` gate with `-D warnings`):

- `#![allow(unused_imports)]` — `NSEvent` in the `cocoa::appkit` import list at
  `src/window.rs:13` (upstream-identical; known since Task 4).
- `#![allow(clippy::collapsible_if)]` — `src/keyboard.rs:38`.
- `#![allow(clippy::single_match)]` — `src/pasteboard.rs:193`.
- `#![allow(clippy::needless_borrow)]` — `src/pasteboard.rs:195`.
- `#![allow(clippy::type_complexity)]` — boxed `FnMut` fields in
  `src/platform.rs:173,174` and `src/window.rs:487`.
- `#![allow(clippy::new_without_default)]` — `MetalHeadlessRenderer::new()` in
  `src/metal_renderer.rs:1775`.
