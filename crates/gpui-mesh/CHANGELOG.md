# Unreleased

# 0.11.0

## Added

- Data-model wave: `CellType::Polygon` with fan triangulation across
  render, slice, and VTK/XDMF/VTU code paths; `DataSet::rectilinear_grid`
  and `DataSet::curvilinear_grid` constructors; `CompositeDataSet`
  (named multiblock blocks with AMR levels); and `TimeSeries::sample`
  for time-interpolated snapshots.

## Added

- `Field::tensor` / `Field::complex` constructors and `as_tensor` /
  `as_complex` views, completing the tensor and complex field API.

# 0.10.44

## Added

- Initial release. Mesh data model for 2D and 3D visualization: VTK-style
  cells (vertex, line, triangle, quad, tetrahedron, hexahedron, wedge,
  pyramid) with `u64` offsets and `u32` connectivity, scalar/vector/
  tensor/complex point and cell fields with validity masks, structured
  hexahedral grid construction, and full dataset validation.
- Boundary-surface extraction with interior-face culling and bounded
  `surface_chunks` iteration, so multi-million-cell datasets are consumed
  in pieces instead of as one monolithic allocation.
- `TimeSeries` for deforming datasets with shared topology.
- No GPUI, wgpu, file-I/O, threading, or clock dependency: identical
  behaviour on native targets and `wasm32-unknown-unknown`.
