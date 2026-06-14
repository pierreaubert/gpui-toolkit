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
