# Local patches on top of zed v1.9.0

This vendored `gpui` fork carries the following local deltas. Keep this file
current: every change relative to upstream zed v1.9.0 must be listed here.

## 1. `Primitive::Custom` scene hook + custom draw registry (MeshPlot, 2026-08-09)

Adds an extension point that lets embedders render their own GPU content
inside the GPUI scene graph, in correct paint order, clipped like any other
primitive. Used by the MeshPlot feature; platform renderer crates
(`gpui_wgpu`, `gpui_macos`) dispatch these batches in follow-up patches.

- **`src/custom_draw.rs` (new)**: `CustomDraw` base trait (`as_any` downcast
  hook), `CustomDrawId` (`u64`), and a main-thread (`thread_local!` + `Rc`)
  registry: `register_custom_draw`, `unregister_custom_draw`,
  `lookup_custom_draw`. `lookup_custom_draw` is `pub` (not `pub(crate)`) so the
  separate renderer crates can resolve ids.
- **`src/scene.rs`**: new `Primitive::Custom(CustomPrimitive)` variant with
  `CustomPrimitive { order, id, bounds, content_mask }` (bounds in
  `ScaledPixels`, mirroring the other scene primitives), threaded through
  `Scene` (`custom_primitives` vec, `clear`, `insert_primitive`, `finish`),
  `PrimitiveKind`, `BatchIterator`, and a new `PrimitiveBatch::Custom(range)`
  batch kind so renderers can dispatch at the right point in scene order.
  Includes a `#[cfg(test)]` scene retention/ordering test.
- **`src/window.rs`**: `Window::paint_custom(id, bounds)`, mirroring
  `paint_image` / `paint_surface` (snaps bounds, applies current content mask,
  inserts into the next frame's scene).
- **`src/gpui.rs`** (crate root): `mod custom_draw;` and re-export of
  `CustomDraw`, `CustomDrawId`, `register_custom_draw`,
  `unregister_custom_draw`, `lookup_custom_draw`.

**Known downstream impact**: `gpui_macos::metal_renderer` still matches on
`PrimitiveBatch` exhaustively without a custom dispatch arm until MeshPlot Task
10 lands. The WGPU arm is provided by Task 9, and the Windows renderer has a
minimal no-op arm because it has no WGPU custom-draw backend. `cargo check -p
gpui` itself is clean.

## 2. `WgpuCustomDraw` capability probe (vello 2D charts, 2026-08-16)

Adds a process-wide flag so chart elements can probe whether the active
renderer actually dispatches `WgpuCustomDraw` primitives, and pick GPU vs
CPU rasterization accordingly. Set by `gpui_wgpu::WgpuRenderer` on init
(native only — wasm canvas renderers store no `GpuContext` and silently
skip custom batches, so they leave the flag false).

- **`src/custom_draw.rs`**: `static WGPU_CUSTOM_DRAW_AVAILABLE: AtomicBool`,
  `wgpu_custom_draw_available() -> bool`, and
  `#[doc(hidden)] set_wgpu_custom_draw_available(bool)` (renderer-only, not
  app API). Includes a `#[cfg(test)]` flag roundtrip test.
- **`src/gpui.rs`** (crate root): re-export of `wgpu_custom_draw_available`
  and `set_wgpu_custom_draw_available` alongside the registry functions.
