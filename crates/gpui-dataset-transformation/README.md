# gpui-dataset-transformation

Dataset-to-dataset transformations for the GPUI toolkit: pure filters that
consume a `gpui_mesh::DataSet` and produce a new `DataSet`.

Filters: plane `slice` (cut), `clip_halfspace`, scalar `threshold`,
marching-tetrahedra `isosurface`, derived-field calculators (vector and
complex magnitude/phase), `warp_by_vector` / `warp_by_scalar`, vector
`glyphs`, and RK2 `streamlines`. Visualisation geometry: `solid_arrows`
(closed 3D shaft+cone arrows), `tensor_glyphs` (ellipsoids from the
eigendecomposition), `streamtubes` / `stream_ribbons` (parallel-transport
extrusions), `dashed_lines` (animated marching-ants dashes), and
`contour_lines` (marching-triangles isolines). Flow diagnostics
(`vorticity`, `divergence`, `q_criterion`, `lambda2`,
`swirling_strength`) use per-tetrahedron gradients averaged to points;
vortex cores are `isosurface(lambda2, 0)`. `lic` appends a
line-integral-convolution streak field, and `particle_frames` advects
seeded particles through a `TimeSeries` with fading trails;
`streak_frames` releases particles continuously (dye lines), `ftle`
computes finite-time Lyapunov exponents, `streamlines_adaptive` traces
with step-doubling error control, and `hyperstreamlines` follows a
tensor field's major axis. Data plumbing: `point_data_to_cell_data` /
`cell_data_to_point_data`, `surface_normals`, `feature_edges`,
`decimate` (vertex clustering), `smooth` (Laplacian), `clip_sphere`,
`append_datasets`, `transform_dataset`, an expression `calculator`,
`resample_to_grid`, `field_statistics` / `histogram`, and point-set
`delaunay_2d` / `convex_hull`. Heavy filters (isosurface, flow
diagnostics, LIC, contours, glyphs) run index-parallel on native
targets with deterministic, order-preserving output; wasm stays
sequential; the `gpu` feature runs the same diagnostics through wgpu compute
(Metal on Apple Silicon, Vulkan/DX12 adapters elsewhere), with a
prepared point-reduction kernel that is about 5x faster than the CPU at
110k points in local benchmarks. I/O covers legacy VTK text, VTU XML
(inline binary, appended raw, and zlib-compressed arrays), XDMF with
binary sidecars, PVD time-series collections, and STL/OBJ/PLY surface
files. Mesh generation itself (tetrahedralization, refinement, meshlang)
belongs to Sonium's mesher; Delaunay triangulation here delegates to the
same `delaunator` kernel it builds on.

No GPUI, wgpu, file-I/O, threading, or clock dependency: every filter runs
identically on native targets and `wasm32-unknown-unknown` and is testable
headlessly. Filters never mutate their input and process large inputs cell
by cell.

```rust
use gpui_dataset_transformation::{slice, Plane};
use gpui_mesh::DataSet;

let room = DataSet::structured_grid("room", [6, 5, 4], [0.0; 3], [0.6, 0.56, 0.6]).unwrap();
let cut = slice(&room, &Plane::axis(2, 0.9).unwrap()).unwrap();
assert_eq!(cut.validate(), Ok(()));
```
