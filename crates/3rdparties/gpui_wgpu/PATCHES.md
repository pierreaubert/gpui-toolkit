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

## 2. `draw_wgpu` `target_format` param + capability probe init (vello 2D charts, 2026-08-16)

Extends the custom-draw seam for the vello 2D chart backend: implementations
need the frame texture's format to build compositing pipelines, and elements
need to know whether custom draws dispatch at all before choosing GPU vs CPU
rasterization.

- **`src/custom.rs`**: `WgpuCustomDraw::draw_wgpu` gains a
  `target_format: wgpu::TextureFormat` parameter after `target`.
- **`src/wgpu_renderer.rs`**: the `PrimitiveBatch::Custom` dispatch site passes
  `self.surface_config.format` as `target_format`; `new_internal` calls
  `gpui::set_wgpu_custom_draw_available(has_context)` gated on
  `gpu_context.is_some()` — native `new()` passes `Some(..)` and advertises
  custom draw, wasm `new_from_canvas` passes `None` and leaves the probe false
  (its dispatch arm skips custom batches anyway when `self.context` is `None`).

## 3. `draw_wgpu` `full_bounds` param + premultiplied sprite shader fix (vello 2D charts, 2026-08-17)

Custom draws receive scene bounds in device pixels at unit scale factor.
Offscreen-buffer draws (vello 2D charts) need the unclipped element extent to
size that buffer and locate the visible sub-region, and polychrome sprites
must honor GPUI's premultiplied-atlas contract.

- **`src/custom.rs`**: `WgpuCustomDraw::draw_wgpu` gains a
  `full_bounds: Bounds<Pixels>` parameter after `bounds` — the unclipped
  element bounds, while `bounds` stays the content-mask-clipped visible
  region.
- **`src/wgpu_renderer.rs`**: the dispatch site passes
  `custom.bounds` (unclipped) as `full_bounds` alongside the clipped bounds.
- **`src/shaders.wgsl`**: `fs_poly_sprite` no longer routes the atlas sample
  through `blend_color` (which multiplied rgb by alpha a second time on
  premultiplied surfaces, darkening translucent sprite content). It now
  matches the Metal renderer's `polychrome_sprite_fragment`: premultiplied
  sample passes through, only `color.a` is scaled by opacity/edge coverage.
