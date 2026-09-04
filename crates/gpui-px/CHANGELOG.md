# Unreleased

## Added

- Added a `decimation` module for large-series downsampling and reworked
  mesh-plot state transitions.
- Fixed heatmap, line, and scatter chart edge cases with new chart tests.

## Refactored

- Extracted the revolve-preparation stage out of `build_frame` into
  `begin_revolve_preparation` with no behavior change.
- Split `build_frame` into staged helpers (`prepare_frame`,
  `prepare_frame_revolve`, `prepare_frame_contours`,
  `prepare_frame_series_2d`, plus interaction, style, navigation, and
  element builders) with no behavior change.

## 0.9.11 - 2026-08-23

### Performance

- Added stable Vello cache keys for common chart marks and allocation contracts for mesh-plot interaction paths.

# 0.9.10

## Features

- Routed ordinary PX chart marks through the d3rs renderer selection API,
  including deterministic Vello scene construction for area, bar, box plot,
  pie/donut, scatter, and treemap charts.

## Fixes

- Contour, heatmap, and isoline charts now honor their configured Vello
  renderer and raster backend consistently.

# 0.9.9

## Features

- Added smooth curve selection, stroke dashes, grouped bars, annotations, per-series opacity, secondary Y axes, nearest-point inspection, legend toggling, brushing, and SVG/PNG/CSV export to the Python chart surface.

## Fixes

- Fixed wheel zoom, drag pan, hover, and brush hit testing by retaining the actual painted bounds and converting window coordinates to chart-local coordinates.
- Added interaction invalidation callbacks so native state changes rebuild host chart geometry instead of repainting stale content.

# 0.9.8

## Features

- Made interactive chart wrappers focusable and added keyboard zoom, pan, and
  reset controls.
- Added hover-domain tracking and clearing plus brush lifecycle wiring while
  retaining pointer panning and wheel zoom.

# 0.7.6

## Features

- Added public `ChartSize` support plus `.fill()`, `.min_size(...)`,
  `.aspect_ratio(...)`, and `.design(...)` builder methods across PX charts.
- PX charts now default to responsive fill sizing while preserving
  `.size(width, height)` as the fixed-size opt-in.

## Fixes

- Chart plot geometry now resolves from `ChartSize` so responsive minimums and
  aspect ratios are reflected in scales, canvases, and plot bounds.

## Performance

- Chart series data (`x`, `y`, values, etc.) is now stored as `Arc<[f64]>` so
  large datasets can be shared between series and render passes.
- Area and pie chart paths are built once outside the paint closure and reused
  for rendering.

# 0.6.4

## Fixes

- **Boxplot**: `bins(0)` now returns `ChartError::InvalidData` instead of
  panicking via `num_bins - 1` underflow / division by zero in
  `calculate_boxes`.
- **Pie**: an empty user-supplied colors slice now falls back to the
  default palette instead of dividing by zero in `colors[i % colors.len()]`.

# 0.6.3

## Features

- Stroke dash array support (`.dash_array(StrokeDashArray::Dashed)`) for line charts
- Migrated showcase to design system / builder pattern
- Re-exported `StrokeDashArray` from `gpui_px`

## Fixes

- Clippy and metadata cleanup
- Showcase dash pattern demo (Solid, Dashed, Dotted, Dash-Dot, Custom)

# 0.6.2

## Features

- Interactive chart pan/zoom (`InteractiveChart` with drag and scroll)
- Heatmap rendering improvements and log scale support
- Bar chart negative value support

## Fixes

- Treemap and boxplot rendering fixes
- Animation crash fix

# 0.6.0

- Initial release after crate reorganization (renamed to `gpui-px`)
- Plotly Express-style API: `scatter()`, `line()`, `bar()`, `heatmap()`,
  `contour()`, `isoline()`, `treemap()`, `boxplot()`, `pie()`, `area()`
- Multi-series line charts with legend and secondary Y-axis
- Logarithmic scale support for all chart types
- Color scales (Viridis, Plasma, Inferno, Magma, Heat, Coolwarm, Greys)
- Showcase binary with interactive examples
