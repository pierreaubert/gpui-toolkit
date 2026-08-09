# Local patches on top of zed v1.9.0

This vendored `gpui_macos` fork carries the following local delta. Keep this
file current when adding further renderer changes relative to upstream zed
v1.9.0.

## 1. `Primitive::Custom` Metal zero-copy dispatch (MeshPlot, 2026-08-09)

Adds the macOS Metal half of GPUI's custom GPU primitive extension. The
renderer resolves the Task 8 `gpui::CustomDrawId` registry at the exact
`PrimitiveBatch::Custom` paint position, ends GPUI's active render encoder,
invokes a registered Metal callback against the same command buffer and
drawable texture, and resumes GPUI rendering with a `Load` pass.

- **`src/custom_metal.rs` (new)**: `MetalCustomDraw` and
  `MetalCustomDrawAdapter`, using the adapter downcast pattern so the shared
  GPUI registry remains platform-agnostic.
- **`src/metal_renderer.rs`**: dispatches registered Metal adapters without
  submitting the command buffer, converts scene `ScaledPixels` bounds to
  logical `Pixels` plus the drawable scale factor, and preserves scene order
  across the custom pass.
- **`src/gpui_macos.rs`**: exports the Metal callback trait and adapter.

Metal callbacks own their render encoder: they must use a `Load` color
attachment action, scissor to the supplied bounds after applying
`scale_factor`, and leave command-buffer submission to the GPUI renderer.
