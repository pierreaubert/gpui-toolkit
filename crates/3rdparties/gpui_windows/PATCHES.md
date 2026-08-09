# Local patches on top of zed v1.9.0

## 1. `PrimitiveBatch::Custom` no-op arm (MeshPlot, 2026-08-09)

Adds the minimal Windows renderer match arm required by GPUI's vendored
`PrimitiveBatch::Custom` scene extension. Windows does not implement the WGPU
custom-draw callback, so the batch is skipped safely until a native backend is
added.

- **`src/directx_renderer.rs`**: ignores `PrimitiveBatch::Custom` with a
  successful no-op result.
