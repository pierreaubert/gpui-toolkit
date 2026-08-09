# Local patches on top of zed v1.9.0

This vendored `gpui_wgpu` crate carries the following local delta. Keep this
file current when refreshing the upstream renderer.

## 1. `PrimitiveBatch::Custom` zero-copy dispatch (MeshPlot, 2026-08-09)

Adds the WGPU platform side of GPUI's custom-draw registry. Registered
`WgpuCustomDrawAdapter` values are resolved in scene order and record directly
into the renderer's command encoder and current frame texture. The renderer
ends its normal pass before each custom batch and resumes it afterward; custom
draws must not submit the encoder themselves.

- **`src/custom.rs` (new)**: `WgpuCustomDraw` and
  `WgpuCustomDrawAdapter`.
- **`src/wgpu_renderer.rs`**: dispatches `PrimitiveBatch::Custom`, skips
  missing or non-WGPU registrations, and hands the shared `WgpuContext` to
  matching draws.
- **`src/gpui_wgpu.rs`**: exports the custom-draw API from the crate root.
