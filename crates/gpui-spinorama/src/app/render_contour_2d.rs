impl SpinoramaApp {
    /// Render contour plot from SPL Horizontal Contour data (new format with full -180 to +180 range)
    fn render_contour_from_contour_data(
        &self,
        title: &str,
        render_mode: ContourRenderMode,
        colormap: Colormap,
        ds: &DesignSystem,
        theme: &Theme,
    ) -> Option<Div> {
        let axis_theme = ChartTheme::from_theme(theme);

        let contour_data = self.contour_data.as_ref()?;

        let freq_count = contour_data.freq_count;
        let angle_count = contour_data.angle_count;

        if freq_count == 0 || angle_count == 0 {
            return None;
        }

        // Get actual angle range from data
        let angle_min = contour_data
            .angles
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let angle_max = contour_data
            .angles
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        // Get frequency range from data
        let freq_min = contour_data
            .freq
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let freq_max = contour_data
            .freq
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        // Python contour scale: fixed -30..+3 dB, one band every 3 dB
        // (`contour_start`/`contour_end` in plot/contour.py), so the
        // colours are comparable across speakers.
        const SPL_MIN: f64 = -30.0;
        const SPL_MAX: f64 = 3.0;
        let spl_min = SPL_MIN;
        let spl_max = SPL_MAX;
        let thresholds: Vec<f64> = (-30..=3).step_by(3).map(f64::from).collect();

        // For the contour generator, we pass the log-transformed frequencies
        let log_freq_values: Vec<f64> = contour_data.freq.iter().map(|f| f.ln()).collect();

        // Fixed axis ranges based on data or reasonable defaults
        let log_freq_min = freq_min.max(20.0).ln();
        let log_freq_max = freq_max.min(20000.0).ln();

        // Create contour generator with explicit log-transformed x
        // values. Upsampling the coarse (angle x frequency) grid
        // bilinearly smooths the marching-squares band edges, as the
        // Plotly contour rendering does.
        let generator = ContourGenerator::new(freq_count, angle_count)
            .x_values(log_freq_values.clone())
            .y_values(contour_data.angles.clone())
            .upsample_factor(4);

        // Generate contours and heatmap data based on render mode
        let is_isoline = render_mode == ContourRenderMode::Isoline;
        let is_surface = render_mode == ContourRenderMode::Surface;
        let is_heatmap = render_mode == ContourRenderMode::Heatmap;
        let cache_hit = self.contour_geometry_cache.borrow().as_ref().is_some_and(|cache| {
            cache.data_revision == self.contour_data_revision && cache.mode == render_mode
        });
        // Generate contours for Isoline mode
        let contours = if cache_hit {
            self.contour_geometry_cache
                .borrow()
                .as_ref()
                .and_then(|cache| cache.contours.clone())
        } else if is_isoline {
            Some(generator.contours(&contour_data.spl, &thresholds))
        } else {
            None
        };
        // Generate contour bands for Surface mode (filled polygons between levels)
        let contour_bands = if cache_hit {
            self.contour_geometry_cache
                .borrow()
                .as_ref()
                .and_then(|cache| cache.bands.clone())
        } else if is_surface {
            Some(generator.contour_bands(&contour_data.spl, &thresholds))
        } else {
            None
        };
        // For heatmap mode, use HeatmapData (renders using quads without anti-aliasing gaps)
        let heatmap_data = if cache_hit {
            self.contour_geometry_cache
                .borrow()
                .as_ref()
                .and_then(|cache| cache.heatmap.clone())
        } else if is_heatmap {
            // Top-down raster of the SPL surface: bilinearly resample
            // the coarse (angle x log-frequency) field onto a fine
            // regular grid (~2 px cells), like the 3D surface seen
            // from above, then bucket values on the Python 3 dB scale
            // (3, 0, -3, ..., -30) so each band takes one flat colour
            // from the colorscale while band edges stay smooth.
            let nx = ((self.content_width / 2.0) as usize).clamp(freq_count, 900);
            let ny = ((self.content_height / 2.0) as usize).clamp(angle_count, 450);
            let (xs, ys, values) = resample_spl_field(
                &log_freq_values,
                &contour_data.angles,
                &contour_data.spl,
                nx,
                ny,
            );
            Some(HeatmapData::new(xs, ys, values))
        } else {
            None
        };

        if !cache_hit {
            *self.contour_geometry_cache.borrow_mut() = Some(ContourGeometryCache {
                data_revision: self.contour_data_revision,
                mode: render_mode,
                contours: contours.clone(),
                bands: contour_bands.clone(),
                heatmap: heatmap_data.clone(),
            });
        }

        let chart_width = self.content_width;
        let chart_height = if self.chart_only {
            // Leave room for the title, x axis, and the three-row
            // colorbar (bar, boundary labels, dB (SPL) title).
            (chart_width * 0.5).min(self.content_height * 0.64)
        } else {
            (chart_width * 0.375).min(self.content_height * 0.5)
        };
        let font_scale = (chart_width / 800.0).clamp(0.7, 1.2);
        let label_sz = (10.0 * font_scale).round();
        let title_sz = (12.0 * font_scale).round();

        // Create scales with data-driven ranges
        let freq_scale = LinearScale::new()
            .domain(log_freq_min, log_freq_max)
            .range(0.0, f64::from(chart_width));

        let angle_scale = LinearScale::new()
            .domain(angle_min, angle_max)
            .range(0.0, f64::from(chart_height));

        // Configure rendering based on mode
        let color_scale = colormap.color_scale();
        // Surface bands are opaque fills without outlines, like the
        // Plotly contour: stroking every band polygon leaves a dotted
        // mesh of seams over the field.
        let contour_config = ContourConfig::new()
            .stroke_width(if is_isoline { 1.5 } else { 0.0 })
            .fill(is_surface)
            .fill_opacity(if is_surface || is_heatmap { 1.0 } else { 0.0 })
            .stroke_opacity(if is_isoline { 1.0 } else { 0.0 })
            .value_range(spl_min, spl_max)
            .color_scale(color_scale);

        // Frequency ticks in log space: every 100 Hz up to 1 kHz,
        // every 1 kHz up to 10 kHz, then 20 kHz.
        let freq_ticks: Vec<f64> = (1..=9)
            .map(|i| f64::from(i) * 100.0)
            .chain((1..=9).map(|i| f64::from(i) * 1000.0))
            .chain([10_000.0, 20_000.0])
            // Small tolerance so the 100 Hz and 20 kHz edge labels
            // survive data whose first/last samples sit just inside.
            .filter(|&f| f >= freq_min / 1.05 && f <= freq_max * 1.05)
            .map(f64::ln)
            .collect();

        // Angle ticks: every 30 degrees across the full domain,
        // extremes included.
        let angle_ticks: Vec<f64> = (-180..=180)
            .step_by(30)
            .map(f64::from)
            .filter(|&a| a >= angle_min && a <= angle_max)
            .collect();

        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(ds.spacing.section_gap))
                .child(
                    div()
                        .text_size(px(ds.typography.large_size * font_scale))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .child(render_axis(
                                    &angle_scale,
                                    &AxisConfig::left()
                                        .with_tick_values(angle_ticks.clone())
                                        .with_formatter(|v| format!("{v:.0}°"))
                                        .with_title("Angle (deg)")
                                        .with_label_font_size(label_sz)
                                        .with_title_font_size(title_sz),
                                    chart_height,
                                    &axis_theme,
                                ))
                                .child(
                                    div()
                                        .w(px(chart_width))
                                        .h(px(chart_height))
                                        .relative()
                                        .bg(theme.surface)
                                        // Render contour bands (for Surface mode) - filled polygons
                                        .when_some(contour_bands.clone(), |el, bands| {
                                            el.child(render_contour_bands(
                                                bands,
                                                &freq_scale,
                                                &angle_scale,
                                                &contour_config,
                                            ))
                                        })
                                        // Render heatmap (for Heatmap mode) - uses quads, no anti-aliasing gaps
                                        .when_some(heatmap_data.clone(), |el, data| {
                                            el.child(render_heatmap(
                                                data,
                                                &freq_scale,
                                                &angle_scale,
                                                &contour_config,
                                            ))
                                        })
                                        // Grid over the filled field, aligned
                                        // with the axis ticks.
                                        .when(is_surface || is_heatmap, |el| {
                                            el.child(render_grid(
                                                &freq_scale,
                                                &angle_scale,
                                                &GridConfig::lines_only()
                                                    .with_vertical_values(freq_ticks.clone())
                                                    .with_horizontal_values(angle_ticks.clone()),
                                                chart_width,
                                                chart_height,
                                                &axis_theme,
                                            ))
                                        })
                                        // Render contour lines (for Isoline mode)
                                        .when_some(contours.clone(), |el, c| {
                                            el.child(render_contour(
                                                c,
                                                &freq_scale,
                                                &angle_scale,
                                                &contour_config,
                                            ))
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .child(div().w(px((80.0 * font_scale).round())))
                                .child(render_axis(
                                    &freq_scale,
                                    &AxisConfig::bottom()
                                        .with_tick_values(freq_ticks)
                                        .with_formatter(|log_f| {
                                            let f = log_f.exp();
                                            if f >= 1000.0 {
                                                format!("{:.0}k", f / 1000.0)
                                            } else {
                                                format!("{f:.0}")
                                            }
                                        })
                                        .with_title("Frequency (Hz)")
                                        .with_label_font_size(label_sz)
                                        .with_title_font_size(title_sz),
                                    chart_width,
                                    &axis_theme,
                                )),
                        ),
                )
                // Centered continuous colorbar: 11 adjacent 3 dB
                // segments with boundary labels, titled dB (SPL).
                .child({
                    let font_sz = (10.0 * font_scale).round();
                    let font_config =
                        GlyphTextConfig::horizontal(font_sz, Hsla::from(theme.text_primary));
                    let bar_w = chart_width * 0.75;
                    let scale_fn = colormap.color_scale();
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(2.0))
                        .child(
                            div()
                                .w(px(bar_w))
                                .flex()
                                .children((0..11).map(|i| {
                                    let color = scale_fn(f64::from(i) / 10.0);
                                    let (r, g, b) = (
                                        (color.r * 255.0) as u32,
                                        (color.g * 255.0) as u32,
                                        (color.b * 255.0) as u32,
                                    );
                                    div()
                                        .flex_1()
                                        .h(px(14.0 * font_scale))
                                        .bg(rgb((r << 16) | (g << 8) | b))
                                })),
                        )
                        .child(
                            div()
                                .relative()
                                .w(px(bar_w))
                                .h(px(font_sz * 1.4))
                                .children((0..12).map(|i| {
                                    let value = -30 + i * 3;
                                    let label = format!("{value}");
                                    let half_w =
                                        label.len() as f32 * font_sz * 0.28;
                                    let left = (bar_w * i as f32 / 11.0 - half_w)
                                        .clamp(0.0, bar_w - half_w * 2.0);
                                    div()
                                        .absolute()
                                        .left(px(left))
                                        .child(render_glyph_text(&label, &font_config))
                                })),
                        )
                        .child(render_glyph_text("dB (SPL)", &font_config))
                }),
        )
    }
}

/// Bracket `v` in a monotonic axis, returning the lower index and the
/// fraction toward the next sample. Works for ascending and
/// descending axes; positions outside the axis clamp to the ends.
fn axis_bracket(axis: &[f64], v: f64) -> (usize, f64) {
    if axis.len() < 2 {
        return (0, 0.0);
    }
    let ascending = axis[axis.len() - 1] >= axis[0];
    let idx = if ascending {
        axis.partition_point(|a| *a < v)
    } else {
        axis.partition_point(|a| *a > v)
    };
    let lo = idx.saturating_sub(1).min(axis.len() - 2);
    let span = axis[lo + 1] - axis[lo];
    let t = if span == 0.0 {
        0.0
    } else {
        ((v - axis[lo]) / span).clamp(0.0, 1.0)
    };
    (lo, t)
}

/// Bilinearly resample a row-major SPL field (`values[y * xs.len() +
/// x]`) onto a regular `nx` by `ny` grid spanning the same axis
/// extents. Output values are bucketed on the 3 dB scale over
/// -30..+3 dB (11 bands); each band is mapped to `-30 + band * 3.3`
/// so its normalized position lands exactly on one of the 11
/// colorscale stops.
fn resample_spl_field(
    xs: &[f64],
    ys: &[f64],
    values: &[f64],
    nx: usize,
    ny: usize,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let width = xs.len();
    if width == 0 || ys.is_empty() || values.len() < width * ys.len() || nx == 0 || ny == 0 {
        return (xs.to_vec(), ys.to_vec(), values.to_vec());
    }
    let linspace = |a: f64, b: f64, n: usize| -> Vec<f64> {
        if n == 1 {
            return vec![a];
        }
        (0..n)
            .map(|i| a + (b - a) * (i as f64) / ((n - 1) as f64))
            .collect()
    };
    let out_xs = linspace(xs[0], xs[width - 1], nx);
    let out_ys = linspace(ys[0], ys[ys.len() - 1], ny);
    let x_brackets: Vec<(usize, f64)> = out_xs.iter().map(|x| axis_bracket(xs, *x)).collect();
    let mut out = Vec::with_capacity(nx * ny);
    for y in &out_ys {
        let (ylo, ty) = axis_bracket(ys, *y);
        let row0 = ylo * width;
        let row1 = (ylo + 1).min(ys.len() - 1) * width;
        for (xlo, tx) in &x_brackets {
            let xhi = (*xlo + 1).min(width - 1);
            let top = values[row0 + xlo] + (values[row0 + xhi] - values[row0 + xlo]) * tx;
            let bottom = values[row1 + xlo] + (values[row1 + xhi] - values[row1 + xlo]) * tx;
            let v = top + (bottom - top) * ty;
            let bucket = ((v + 30.0) / 3.0).floor().clamp(0.0, 10.0);
            out.push(-30.0 + bucket * 3.3);
        }
    }
    (out_xs, out_ys, out)
}
