# Unreleased

# 0.11.1

## Added

- Streaming movie export: `write_png_sequence` sinks, `mux_mp4_to`
  (bounded feed queue, concurrently drained pipes — frames larger
  than a pipe buffer no longer deadlock), and
  `Player::stream_movie_frames` with cancellation. Collect-all
  `png_sequence`/`mux_mp4` keep an explicit small-export limit.
- Scalar-only updates: retained preparation and GPU sync rebuild only
  packed scalars (new scalar buffers, encoded-pass safe) when
  topology, transport, and the validity pattern are unchanged.
- `GpuStats` activity counters, per-chunk frustum culling,
  `sync_with_budget` incremental actor uploads (single-call
  completions stay atomic), and in-place `resize_target` reuse.
- Volume raycasting: field-revision grid reuse (`update_field`),
  `VolumeOptions::sample_budget`, and `render_cancellable`.
- Warm-frame allocation acceptance test (camera-only frames do not
  allocate in proportion to mesh size).

## Changed

- Face normals accumulate per global point across chunks, so shared
  vertices shade continuously across chunk boundaries (previously
  each chunk normalized independently); point-scalar preparation
  streams in two passes instead of materializing all chunks.
- Volume opacity uses exponential step-length correction
  (homogeneous slabs are step-size independent); the tet grid is CSR
  with cached inverse transforms and empty-space skipping.
- The interactive viewer example now requires the `gpui-gpu`
  feature, so the shipped configuration exercises the host-GPU
  element.

# 0.11.0

## Added

- `gpu` feature: `GpuRenderer` / `GpuScene` / `GpuTarget` — GPU-resident
  surface rendering. Geometry and colour LUTs upload once; per-frame
  draws rewrite only per-actor uniforms, so nothing returns to the CPU.
  Opaque and transparent (sorted, blended) pipelines, clip planes, and
  a QA readback for headless tests and movie export. `gpu_render`
  example renders a 1080p isosurface (about 2x faster than the software
  rasterizer in debug builds, before readback).

## Added

- `legend` module: colour-bar data (nice ticks, formatted labels),
  pixel layout, gradient-strip painting into software frames, axis
  triad overlay, and `Annotation` text anchors on `Scene`.
- `TransferFunction::banded` for filled contours.
- `ClipBox` clipping (six planes from bounds) via `Actor::clip_box`;
  the interactive viewer now drives three orthogonal cutting planes
  simultaneously plus a clip box with inset steps.
- `flow_showcase` example: Taylor-Green vortex exercising arrows,
  streamtubes/ribbons, dashed streamlines, LIC, contours, tensor
  glyphs, lambda-2 cores, legend, and a 48-frame particle movie.

# 0.10.44

## Added

- Initial release. Rendering for `gpui-mesh` datasets: orbit/pan/dolly
  camera with fitted projection, color transfer functions (named ramps,
  linear/log scaling, colour-blind-safe palettes), scenes with per-actor
  clip planes and opacity, and GPU preparation that rebases to the dataset
  centre so large real-world coordinates stay precise.
- Software rasterizer: per-fragment clip-plane testing, painter-sorted
  transparency, wireframe and points modes, double-sided lighting, and
  polylines rendered as shaded tubes. Maximum-intensity-projection volume
  raycasting with analytic ray/box clipping.
- WGSL surface, volume, and picking shaders ready for a wgpu host.
- Animation and movies: `Player` (explicit caller-supplied clock) and
  `png_sequence` at movie resolution; native-only `mp4` feature muxes the
  sequence to H.264 with a system `ffmpeg` and returns the bytes.
- Optional `gpui` feature: `SceneElement`, a ready-to-paint element for
  GPUI applications.
- Examples: `pcfan_airflow`, `speaker_port`, `necking_bar`, and the
  `interactive_viewer` (rotate/zoom/pan, live cutting plane), with
  1920x1080 / 120-frame movies under `docs/movies/`.
