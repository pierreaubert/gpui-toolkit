# vello2d scatter benchmark — 2026-08-17

Criterion run of `benches/vello2d_bench.rs` (gpui-d3rs, `--no-default-features --features vello`):

```bash
cargo bench -p gpui-d3rs --no-default-features --features vello --bench vello2d_bench
```

**Machine:** Mac mini, Apple M4 Pro (14 cores), 64 GB RAM, macOS (arm64), rustc 1.97.1, criterion 0.8.2, 100 samples per benchmark.

## Results

| Benchmark                    | Points   | Mean      | 95% CI            |
|------------------------------|----------|-----------|-------------------|
| `vello2d_scatter/scene_build`| 100,000  | 55.92 ms  | 51.37 – 60.86 ms  |
| `vello2d_scatter/cpu_raster` | 100,000  | 515.70 ms | 490.49 – 541.61 ms |
| `vello2d_scatter/scene_build`| 1,000,000| 231.26 ms | 227.33 – 236.51 ms |
| `vello2d_scatter/cpu_raster` | 1,000,000| 2.529 s   | 2.480 – 2.607 s   |

Outliers: 5/100 high (scene_build 100k), 1/100 high mild (cpu_raster 100k),
3/100 high (scene_build 1M), 4/100 high severe (cpu_raster 1M).

## Context

`scene_build` measures `d3rs::shape::scatter_chart_scene` — scale-mapping the
points and building the backend-neutral `ChartScene`. Since the Task 8
batching change, this produces **one fill command per series** (all point
circles accumulated into a single batched `BezPath`), plus one batched stroke
command when a stroke is configured. So scene build is a pure CPU path-build,
linear in point count (~0.55 µs/point at 100k, ~0.23 µs/point at 1M).

`cpu_raster` measures `d3rs::vello2d::CpuRasterizer::rasterize` (vello_cpu)
rendering that scene to an 800×600 RGBA pixmap — the CPU fallback backend.
Note this is dominated by actually shading a million tiny circles and is
**not** the interactive path: the GPU backend (`WgpuVelloDraw`, zero-copy via
`WgpuCustomDraw`) is the production route, and its frame time is validated
via the px showcase (Task 9). A direct comparison against the legacy
`paint_path` scatter pipeline lands with the follow-up chart-port plan.

The bench config sets only `fill_color` + `point_radius` on
`ScatterConfig::new()`, so it measures the real default: that default
includes a white stroke at 0.7 opacity, meaning both the batched fill path
**and** the batched stroke path are built and rasterized in the numbers
above.
