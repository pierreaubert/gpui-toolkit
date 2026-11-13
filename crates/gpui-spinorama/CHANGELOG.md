# Unreleased

# 0.11.3

## Changed

- Contour cache hits no longer build the contour generator or the
  log-frequency vector before discovering the hit.

## Fixed

- Preserve null samples as line breaks in scatter plots, including phase wraps.
- Label phase secondary axes in degrees.

# 0.11.1

## Changed

- Published Globe pictures are now the polar disc (angle around,
  log-frequency as radius, fixed -30..+3 dB scale, bilinearly
  sampled) instead of the per-frequency 3D sphere; the interactive
  sphere view is unchanged.
- Contour pictures keep a 1.6 width/height ratio on the contour
  area itself.
- 3D surface pictures sit further back so the surface is no longer
  truncated at the bottom of the frame.
- Every SPL-vs-frequency graph reserves the DI secondary-axis
  width, so all plot areas have the same width and align in
  comparisons.
- CEA2034 DI curves share their parent curve colour (ERDI green
  like Early Reflections, SPDI red like Sound Power).

# 0.11.0

## Fixed

- Contour charts: the frequency axis below the field is offset
  by the angle axis' own `total_size()` instead of a hard-coded
  80 * font_scale spacer, which drifted every frequency tick
  sideways against the field and grid.
- Headless `draw_capture` refreshes the window before drawing:
  with the gpui-fast element cache, state injected by direct
  field mutation was never rendered and every captured image was
  the empty first frame.

# 0.10.53

## Changed

- Contour charts: the resampled field is bucketed on the Python
  3 dB scale (11 bands from -30 to +3 dB, one flat colorscale
  colour per band, smooth band edges). Angle axis is titled
  "Angle (deg)" with ticks every 30 degrees including the
  extremes; frequency ticks run every 100 Hz to 1 kHz, every
  1 kHz to 10 kHz, then 20 kHz. A grid aligned with the axis
  ticks is drawn over the filled field. The colour legend is a
  centered continuous 11-segment bar with 3 dB boundary labels
  and a "dB (SPL)" title.

# 0.10.52

## Changed

- Headless contour pictures (`spinorama-render-images`) now render
  the 2D contour as a smooth top-down raster of the SPL surface:
  the field is bilinearly resampled in (angle x log-frequency) onto
  a fine regular grid and drawn as merged heatmap quads with the
  fixed -30..+3 dB Spinorama scale, replacing the marching-squares
  band polygons and their visible cell texture.

# 0.10.51

## Changed

- Radar/spider charts: category and angle labels are much larger
  (scaled with the chart size, centred on their spokes, primary text
  colour), and the radial grid circles now carry scale labels along
  the top spoke (grid fractions for spiders, nice steps for numeric
  radars). Headless polar pictures also fill the frame instead of
  being capped at the interactive 620 px size.

# 0.10.50

## Changed

- The generic chart legend reuses the CEA2034 legend's look and
  feel: entries centred on a muted rounded band, same 12 px primary
  text, row height, and hover treatment. Applies to the frequency,
  polar, bar, and subplot chart legends.

# 0.10.49

## Fixed

- Headless image rendering resolved graph files by measurement name
  first, so every "Contour 3D" file was drawn with the 2D contour
  renderer and Globe files had no renderer at all. Classification now
  prefers an exact catalog label match, sphere graphs receive their
  contour data like surface graphs do, and the sphere view is fed
  both directivity planes from the sibling SPL payloads (site
  pictures show the globe at 1 kHz).
- 3D surface and sphere pictures in headless (chart-only) mode no
  longer include the interactive colormap/opacity/frequency controls;
  they carry the payload title instead.

# 0.10.48

## Changed

- Reduced the padding between the vertical axis titles and the tick
  values on the left and right axes of SPL/frequency charts
  (`AXIS_TITLE_PADDING`, 2 px).
- Payloads can suppress the black 0 dB reference line with
  `layout.hide_zero_line`; the On Axis, Estimated In-Room, and
  non-normalized SPL Horizontal/Vertical figures now set it.
- Contour plots use the Python spinorama colormap by default
  (`Colormap::Spinorama`, the 11-stop `CONTOUR_COLORSCALE`), fixed
  -30..+3 dB bands every 3 dB, no grid over the contour field, and a
  4x bilinear upsample of the SPL grid for smoother band edges.
- Polar (radar) charts show their title again in chart-only
  (headless image) mode.

# 0.10.47

## Changed

- SPL versus frequency charts now follow the Python spinorama layout:
  Y labels every 5 dB, thin grid lines at every log tick (2–9 per
  decade) with no dots at intersections, minor ticks on the frequency
  axis, and axis ticks drawn inside the plot area.
- Added a black 0 dB reference line, slightly thicker than the grid,
  to the CEA2034/directivity and generic frequency charts.
- Graph catalog tick steps updated: SPL charts label every 5 dB and
  horizontal/vertical directivity charts use 90° steps over
  −180…180°.
