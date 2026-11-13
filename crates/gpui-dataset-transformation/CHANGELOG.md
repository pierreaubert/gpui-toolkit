# Unreleased

# 0.11.1

## Added

- `threshold_gpu` / `GpuFlow::threshold_cells`: portable GPU
  classify → blocked exclusive scan → scatter threshold pipeline with
  exact CPU output parity (differential-tested).
- `GpuFlow::from_shared_device`, `with_power_preference`, and
  `new_async`; `PreparedFlow` resident diagnostics
  (`run_resident`, buffer accessors, readback counters) with
  selectable outputs via `FlowOutputs`.
- Structured `VtuError` for VTU reads (typed budget variant);
  fallible numeric-array expansion.

## Changed

- `contour_lines` honors validity masks (triangles with missing
  nodes emit nothing) and validates its input and levels.
- Mesh IO is validity-aware: VTK/VTU/XDMF export masked entries as
  NaN with a `<name>__valid` companion and readers restore masks;
  bare NaNs read back as missing data. Surface formats are documented
  geometry-only.
- GPU dispatches are partitioned across 2D grids with checked buffer
  sizes (typed `GpuError::TooLarge`); per-tet geometry is cached by a
  prepass instead of recomputed per adjacency entry; the default
  power policy is the system default.
- `read_vtu*` now returns `Result<_, VtuError>` instead of `String`.

# 0.11.0

## Added

- Data-model and I/O wave: VTU appended raw binary and zlib-compressed
  (`vtkZLibDataCompressor` layout) writers, a byte-oriented VTU reader,
  STL/OBJ/PLY surface readers and writers, and PVD collection loading
  (parallel parts merged per timestep). The base64 codec is now public
  for bridge consumers.
- Sonium scope boundary documented: Delaunay delegates to the
  `delaunator` kernel Sonium's mesher builds on; no meshing algorithms
  (tetrahedralization, refinement, meshlang) are implemented here.

## Added

- Missing-filter wave: point/cell data conversion, `surface_normals`,
  `feature_edges`, `decimate`, `smooth`, `clip_sphere`,
  `append_datasets`, `transform_dataset`, expression `calculator`,
  `resample_to_grid`, `field_statistics` / `histogram`, `delaunay_2d`,
  `convex_hull`, `streamlines_adaptive` (step-doubling RK2),
  `streak_frames`, `ftle`, and `hyperstreamlines`.
- Heavy filters (isosurface, flow diagnostics, LIC, contours, solid
  arrows, tensor glyphs) now evaluate index-parallel on native targets
  via scoped threads with order-preserving, bit-identical output
  (vorticity and isosurface benchmarks improve ~30-40% at 24^3); wasm
  builds run the same code sequentially.

## Added

- `solid_arrows`: closed 3D shaft+cone arrow glyphs coloured by a
  `magnitude` field; `tensor_glyphs`: ellipsoid glyphs from the
  eigendecomposition of a symmetric-tensor field.
- `particle_frames`: time-dependent particle advection (pathlines) with
  fading trails through a `TimeSeries`.
- `streamtubes` and `stream_ribbons` (parallel-transport frames),
  `dashed_lines` for animated marching-ants flow direction.
- `contour_lines` marching-triangles isolines with a per-vertex `value`
  field.
- `lic`: deterministic line-integral-convolution streak field.
- `lambda2` and `swirling_strength` vortex criteria (analytic against
  solid-body rotation and pure strain); vortex cores extract as
  `isosurface(lambda2, 0)`.

# 0.10.44

## Added

- Initial release. Pure `DataSet -> DataSet` filters with point-field
  interpolation: plane `slice` and `clip_halfspace`, scalar `threshold`,
  marching-tetrahedra `isosurface`, derived-field calculators (vector and
  complex magnitude/phase), `warp_by_vector` / `warp_by_scalar`, vector
  `glyphs`, RK2 `streamlines`, and barycentric `probe`.
- Flow diagnostics from per-tetrahedron gradients: `vorticity`,
  `divergence`, `q_criterion`, and `vorticity_magnitude`, verified against
  analytic solid-body rotation and pure strain.
- I/O: legacy VTK text, VTU XML with inline binary arrays, and XDMF with
  binary sidecars (host-resolved, no filesystem assumed).
- Optional `gpu` feature: wgpu compute diagnostics (Metal, Vulkan,
  Direct3D, WebGPU). A retained `PreparedFlow` point-reduction kernel
  uploads geometry once and streams only velocities per frame; about
  5x faster than the CPU at 110k points in local Metal benchmarks,
  matching the CPU reference to `f32` gradient precision.
