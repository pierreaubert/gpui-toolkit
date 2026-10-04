# Local patches on top of zed v1.9.0

## 1. `PrimitiveBatch::Custom` no-op arm (MeshPlot, 2026-08-09)

Adds the minimal Windows renderer match arm required by GPUI's vendored
`PrimitiveBatch::Custom` scene extension. Windows does not implement the WGPU
custom-draw callback, so the batch is skipped safely until a native backend is
added.

- **`src/directx_renderer.rs`**: ignores `PrimitiveBatch::Custom` with a
  successful no-op result.

## 2. `PaddedBool32` shader mask simplification (upstream port, 2026-10-04)

Ports the `gpui_windows` hunk of upstream zed PR #60482 (merged 2026-07-09):
`polychrome_sprite_fragment` compares `grayscale` directly instead of
masking with `& 0xFFu`. HLSL-only change; not compiled on this host
(same caveat as the upstream author: no Windows target available).

## 3. Atlas texture lookup returns `Option` (upstream port, 2026-10-04)

Ports the `gpui_windows` hunks of upstream zed PR #64623 (merged 2026-09-22):
`get_texture_view()` returns `Option` and the three sprite draws skip
batches whose texture was released. Rust changes; not compiled on this
host — needs Windows CI (same caveat as the upstream author: their
DirectX hunks were also uncompiled).
