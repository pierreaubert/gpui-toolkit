# Unreleased

## Performance

- Replaced one-thread-per-timer scheduling with a shared D3 timer scheduler;
  applications can register a UI-thread dispatcher for timer callbacks.
- Added a pixel-area budget to GPU 3D surface readback before creating the
  CPU-backed `RenderImage`.
- Reused per-size GPU resolve/readback buffers for surface frames, and added
  bounded/cancellable CSV, TSV, DSV, and TopoJSON parsing entry points.

# 0.9.9

## Features

- Added Gregorian month and year interval arithmetic and calendar-aware time
  ticks.
- Added SVG endpoint-parameterized elliptical arc flattening with exact path
  bounds.
- Added radial curve interpolation.
- Added polygon-level antimeridian preclip buffering and reconnection.

## Fixes

- Implemented contour-apportioned tidy-tree layout and recursive depth updates
  for attached subtrees.
- Updated the 3D surface example for Rust 1.97 Clippy compatibility.

# 0.8.5

## Breaking Changes

- Corrected geographic projection `center()` semantics to match D3.js:
  `.center(lon, lat)` is now applied as a **post-projection planar offset**,
  not as a pre-projection angular offset. Mercator, Orthographic,
  Stereographic, and Transverse Mercator projections with non-zero centers now
  produce different (correct) results.

## Features

- Added JS golden generator `golden/geo/generate_projections_angles.js` and
  generated `golden/geo/projections_angles.json`.
- Added Rust golden test `test_geo_projections_angles_golden` covering all five
  geographic projections at varied center and rotation angles.

## Fixes

- Fixed Mercator, Orthographic, Stereographic, and Transverse Mercator
  projections for non-zero centers and rotations.
- Fixed Conic Equal-Area inverse projection to use proper rotation inversion
  and the planar center offset.
- Removed the static geographic clip rectangle from Mercator and
  Equirectangular projections. Mercator now clamps input latitude to the Web
  Mercator limit to keep projected y finite. This eliminates diagonal closing
  chords and blank strips when cylindrical projections are rotated or centered.
- `GeoPath` no longer draws a redundant closing edge for polygon rings whose
  last vertex duplicates the first.
- Improved `GeoPath` antimeridian handling for cylindrical projections: rings
  are cut at periodic boundary crossings, full-width polar caps keep their
  boundary edges, and each sub-piece is normalized to the visible world copy.
  This removes horizontal closing chords across rotated or centered maps while
  preserving fills down to the Mercator pole clamp.

# 0.7.4

## Features

- Fixed Versor Mercator horizontal-streak regression: cylindrical projections
  now clip geometries to a rectangular geographic extent before projecting,
  preventing degenerate closing chords across the map.

## Tests

- Added `golden/geo/path_cylindrical.json` with Mercator/Equirectangular path
  cases (including the south-pole clip regression) and a corresponding golden
  test `test_geo_path_cylindrical_golden`.

# 0.7.3

## Features

- Spinorama demo now solves its shell layout with `gpui-builder` and uses the
  design system for spacing and typography.
- Added design-aware GPUI config helpers for axes, grids, legends, glyph text,
  GPU 2D/3D surfaces, and common shape defaults.
- Spinorama CEA2034 and horizontal/vertical SPL legends can hide or show their
  corresponding curves. Horizontal and vertical SPL plots now use discrete line
  legend entries by angle instead of a contour-style color scale.

## Fixes

- Fixed GPU 3D contour surface, isoline, and grid-line rendering artifacts so
  clipped segments do not connect across gaps and grid lines stay behind the
  surface.
- Made 3D surface isolines depth-aware so foreground surface geometry occludes
  far-side contour strokes.
- Fixed spinorama frequency plot X-axis alignment when a secondary DI axis is
  present, and centered line legends within their legend panels.
- Made spinorama horizontal/vertical SPL signed-angle traces easier to inspect:
  negative angles are rendered as dashed high-contrast lines with matching
  legend markers so overlapped `-60°` and `60°` traces can be toggled
  independently.
- Spinorama horizontal/vertical SPL plots now include a `0°` on-axis fallback
  from CEA2034 data when the directivity trace set does not expose one.
- Aligned spinorama legend line markers with the middle of their labels.
- Updated the d3rs showcase to inherit theme and design tokens for text and UI
  chrome outside the color demo.

## Performance

- Contour and contour-band elements now build their fill/stroke paths in
  `prepaint` and cache them for `paint`, avoiding repeated path construction
  each frame.
- Line types precompute segment geometry so render loops do not re-derive it.

# 0.7.0

## Breaking Changes

- `d3rs::fetch` parsing is now `Result`-first: `parse_csv`, `parse_tsv`,
  `parse_dsv`, `DsvParser::parse`, and `DsvParser::parse_rows` return
  structured `DsvParseError` values instead of silently returning empty data on
  malformed input.

## Features

- Added explicit lossy helpers (`parse_csv_lossy`, `parse_tsv_lossy`,
  `parse_dsv_lossy`, `DsvParser::parse_lossy`, and
  `DsvParser::parse_rows_lossy`) for D3-compatible demo paths.
- Added `ColumnPolicy::Strict` for header/row width validation plus empty and
  duplicate header rejection.
- DSV parsing now handles quoted newlines and CRLF input while reporting line,
  column, byte offset, and structured error kinds.

## Fixes

- `CsvOptions::default()` now matches `CsvOptions::new()` instead of disabling
  empty-line skipping and value trimming.

# 0.6.9

## Features

- `gpu3d::Lines3DElement`: new GPUI element rendering line / polygon scenes via CPU projection (`Camera3D::project_to_screen`) + `gpui::PathBuilder`. Same orbit / pan / zoom semantics as `Surface3DElement` through a shared `Lines3DState` (`Rc<RefCell<_>>`); parents wire mouse handlers to drive the embedded `OrbitControls`. Designed for sparse 3D scenes (~50 vertices) where a full wgpu pipeline would be overkill.

## Fixes

- **voronoi_airports example**: track `math-delaunay` API change — `triangles` and `halfedges` are now methods, not public fields. Updated callsites to use `.triangles()` / `.halfedges()`. Unblocks workspace compile.

# 0.6.8

## Features

- Added 3d lines with wpgu support

# 0.6.7

## Features

### Stroke dash array support for line rendering

- Added `StrokeDashArray` enum with predefined patterns (`Dotted`, `Dashed`,
  `DashDot`) and `Custom(Vec<f32>)` for arbitrary dash/gap sequences.
- Added `dash_array` field to `LineConfig` and a `.dash_array()` builder method.
- `render_line` now walks along line segments and splits them into dash/gap
  sub-segments when a pattern is set. The pattern state carries continuously
  across segments for seamless dashing.
- Re-exported `StrokeDashArray` from `shape::mod` and `lib.rs` prelude.

# 0.6.6

## Features

- Sphere gallery: GPU-rendered 3D sphere gallery with Metal shaders
- Legend rendering module (`legend/`)
- Voronoi stippling Observable example

## Fixes

- Fixed geo path clipping
- Fixed segfaults when data contains NaN
- iOS rendering support

# 0.6.5

## Features

- Sankey diagram layout engine
- 13 new Observable examples (ridgeline, sunburst, parallel sets, star map, etc.)
- Versor dragging for geo projections

## Fixes

- Voronoi rendering fixes
- NaN/error tolerance in plot rendering
- Dead code cleanup and clippy lints

# 0.6.4

## Features

- Observable examples framework (hexbin, pie, donut, line, stacked bar/area, streamgraph, chord, force-directed, box plot)
- Chord diagram layout
- Hexbin aggregation module

## Fixes

- Force simulation clamping during interpolation
- Log scale improvements
- Stack layout fixes

# 0.6.1

## Features

- Upgraded wgpu to latest version
- Split autoeq UI from UI Kit into standalone crate

## Fixes

- Clippy lints and formatting cleanup

# 0.6.0

- Initial release after crate reorganization (renamed from internal paths to `gpui-d3rs`)
- D3.js-inspired scales (linear, log, band, time, color)
- Shape rendering (line, bar, scatter, arc, pie, area, contour, heatmap)
- GPU-accelerated 2D and 3D rendering
- Axis and grid rendering
- Force-directed graph layout
- Delaunay triangulation
- Golden test infrastructure for D3.js compatibility
