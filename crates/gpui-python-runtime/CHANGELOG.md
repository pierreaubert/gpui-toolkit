# Unreleased

# 0.11.0

## Features

- Python stays in sync with the Rust data-model and I/O wave:
  `gpui_toolkit.mesh.read_mesh` / `write_mesh` exchange VTU (raw or
  zlib-compressed), OBJ, PLY, and STL files, and `read_pvd` loads a PVD
  time series with parallel parts merged (backed by the new native
  `mesh_read` / `mesh_write` / `mesh_read_pvd` bridge functions).

## Features

- `gpui_toolkit.mesh` stays in sync with the new Rust filters: the
  `run`/`geometry` op chains accept `point_to_cell`, `cell_to_point`,
  `normals`, `feature_edges`, `decimate`, `smooth`, `clip_sphere`,
  `calculator`, `resample`, `hyperstreamlines`, `delaunay`,
  `convex_hull`, `transform`, and `append`, and new `mesh_statistics` /
  `mesh_ftle` native functions back `mesh.statistics()` and
  `mesh.ftle()`.

## Features

- Mesh visualization bridge to `gpui-mesh` and
  `gpui-dataset-transformation` (both now workspace dependencies): the
  native extension exports `mesh_filters` (structured-grid dataset spec
  plus slice/clip/threshold/isosurface/vorticity/divergence/Q-criterion/
  magnitude ops, returning a JSON summary), `mesh_streamlines` (seeded
  RK2 streamlines stitched into polylines), and `mesh_problem` (spec
  validation). The new `gpui_toolkit.mesh` Python module wraps them with
  `structured`, `slice_`, `clip`, `run`, `problem`, and `streamlines`.
- The bridge now also exposes the flow-visualisation wave in sync with
  the Rust filters: `mesh_geometry` (points/cells/fields for
  `solid_arrows`, `tensor_glyphs`, `streamtubes`, `stream_ribbons`,
  `dashed_lines`, `contour_lines`, `lic`) and `mesh_particles`
  (time-dependent particle advection), with matching `gpui_toolkit.mesh`
  helpers (`arrows`, `tensor_glyphs`, `tubes`, `ribbons`, `dashes`,
  `contours`, `lic`, `geometry`, `particles`, tensor fields in
  `structured`).

# 0.10.33

## Features

- Added retained IR nodes plus Python builders and showcase sections
  for the new UI-kit components, including `CalendarNode`,
  `PaginationNode`, and `ListNode`/`ListItemNode` with strict
  validation (YYYY-MM-DD dates, 1 ≤ page ≤ total, non-blank rows).
- Added `DatasetFrame::encode`/`decode` wire framing and
  `DatasetFrameStore::ingest_arrow_ipc` so numpy-backed Arrow IPC bytes
  ingest alongside JSON specs with header-first budget enforcement.
- Split `PxChartV2Node::validate` into per-chart-kind helpers with unchanged
  error precedence.
- Mirrored the new `AudioDesignTokens` spectrum geometry fields
  (`spectrum_bar_gap`, `spectrum_corner_radius`, `spectrum_peak_height`)
  in the Python dataclass, report decoder, and showcase evidence.

## Performance

- Added `MeshPlotSpec::validate_value`, which runs schema-version and
  structural gates on the borrow so rejected specs skip the clone.
- Added a `spec_validate` criterion bench (validate + frame ingest + decode
  hot/cold) with documented budgets.
- Fixed two `manual_contains` clippy lints in dataset-frame decoding.

## 0.9.10 - 2026-08-23

### Performance

- Completed retained mesh and field object caching with structural fingerprints and stable element identifiers for Python-rendered scenes.

# 0.9.7

## Features

- Completed typed Python declarations, commands, events, resources, platform services, tooling reports, and strict capability negotiation across the aggregate toolkit surface.
- Added native Rust-backed D3 algorithms and layouts for color, formatting, time, DSV, interpolation, easing, random distributions, selection, brush, drag, transitions, geometry, force, hierarchy, chord, Sankey, geo, tiles, and LOD.
- Added full native PX chart declarations and interaction wiring, including functional wheel zoom, drag pan, brushing, keyboard navigation, inspection, legend toggling, and static export.
- Added native audio, design, pretext, themes, UI conformance, builder, scaffolder, and platform bridges without additional runtime dependencies.
- Mesh scene specs now have a retained GPUI rendering path that normalizes
  validated triangles and paints them as 3D polygons instead of showing a
  validation-only summary.

## Fixes

- Rebuild chart host views when zoom, pan, brush, hover, or legend state changes, and use the actual painted chart bounds for pointer hit testing.

# 0.7.2

## Maintenance

- Version bump; no user-facing changes.
