impl SpinoramaApp {
    /// Render contour plot from SPL Horizontal Contour data (new format with full -180 to +180 range)
    fn render_contour_from_contour_data(
        &self,
        title: &str,
        render_mode: ContourRenderMode,
        colormap: Colormap,
    ) -> Option<Div> {
        let theme = DefaultAxisTheme;

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
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let angle_max = contour_data
            .angles
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        // Get frequency range from data
        let freq_min = contour_data
            .freq
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let freq_max = contour_data
            .freq
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        println!(
            "Contour (SPL Horizontal Contour): {} angles x {} freqs, angle range: {:.1}° to {:.1}°, freq range: {:.1}Hz to {:.1}Hz",
            angle_count, freq_count, angle_min, angle_max, freq_min, freq_max
        );

        // Calculate SPL range
        let spl_min = contour_data
            .spl
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let spl_max = contour_data
            .spl
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        // Generate contour thresholds (every 3 dB based on actual data range)
        let threshold_min = ((spl_min / 3.0).floor() * 3.0) as i32;
        let threshold_max = ((spl_max / 3.0).ceil() * 3.0) as i32;
        let thresholds: Vec<f64> = (threshold_min..=threshold_max)
            .step_by(3)
            .map(|v| v as f64)
            .collect();

        // For the contour generator, we pass the log-transformed frequencies
        let log_freq_values: Vec<f64> = contour_data.freq.iter().map(|f| f.ln()).collect();

        // Fixed axis ranges based on data or reasonable defaults
        let log_freq_min = freq_min.max(20.0).ln();
        let log_freq_max = freq_max.min(20000.0).ln();

        // Create contour generator with explicit log-transformed x values
        let generator = ContourGenerator::new(freq_count, angle_count)
            .x_values(log_freq_values)
            .y_values(contour_data.angles.clone());

        // Generate contours and heatmap data based on render mode
        let is_isoline = render_mode == ContourRenderMode::Isoline;
        let is_surface = render_mode == ContourRenderMode::Surface;
        let is_heatmap = render_mode == ContourRenderMode::Heatmap;
        // Generate contours for Isoline mode
        let contours = if is_isoline {
            Some(generator.contours(&contour_data.spl, &thresholds))
        } else {
            None
        };
        // Generate contour bands for Surface mode (filled polygons between levels)
        let contour_bands = if is_surface {
            Some(generator.contour_bands(&contour_data.spl, &thresholds))
        } else {
            None
        };
        // For heatmap mode, use HeatmapData (renders using quads without anti-aliasing gaps)
        let heatmap_data = if is_heatmap {
            // Use log-transformed frequencies for the heatmap x-values
            let log_freq_values: Vec<f64> = contour_data.freq.iter().map(|f| f.ln()).collect();
            Some(HeatmapData::new(
                log_freq_values,
                contour_data.angles.clone(),
                contour_data.spl.clone(),
            ))
        } else {
            None
        };

        let chart_width = 800.0;
        let chart_height = 300.0;

        // Create scales with data-driven ranges
        let freq_scale = LinearScale::new()
            .domain(log_freq_min, log_freq_max)
            .range(0.0, chart_width as f64);

        let angle_scale = LinearScale::new()
            .domain(angle_min, angle_max)
            .range(0.0, chart_height as f64);

        // Configure rendering based on mode
        let color_scale = colormap.color_scale();
        let contour_config = ContourConfig::new()
            .stroke_width(if is_isoline { 1.5 } else { 0.5 })
            .fill(is_surface)
            .fill_opacity(if is_surface {
                0.6
            } else if is_heatmap {
                1.0
            } else {
                0.0
            })
            .stroke_opacity(if is_isoline { 1.0 } else { 0.8 })
            .color_scale(color_scale);

        // Build frequency tick values in log space
        let freq_ticks: Vec<f64> = [
            20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0,
        ]
        .iter()
        .filter(|&&f| f >= freq_min && f <= freq_max)
        .map(|f| f.ln())
        .collect();

        Some(
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x333333))
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
                                        .with_ticks(13)
                                        .with_formatter(|v| format!("{:.0}°", v))
                                        .with_title("Angle"),
                                    chart_height,
                                    &theme,
                                ))
                                .child(
                                    div()
                                        .w(px(chart_width))
                                        .h(px(chart_height))
                                        .relative()
                                        .bg(rgb(0xf8f8f8))
                                        // In Isoline mode, render grid first (underneath lines)
                                        .when(is_isoline, |el| {
                                            el.child(render_grid(
                                                &freq_scale,
                                                &angle_scale,
                                                &GridConfig::with_lines()
                                                    .with_vertical_values(freq_ticks.clone()),
                                                chart_width,
                                                chart_height,
                                                &theme,
                                            ))
                                        })
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
                                        // In Surface/Heatmap mode, render grid on top
                                        .when(is_surface || is_heatmap, |el| {
                                            el.child(render_grid(
                                                &freq_scale,
                                                &angle_scale,
                                                &GridConfig::with_lines()
                                                    .with_vertical_values(freq_ticks.clone()),
                                                chart_width,
                                                chart_height,
                                                &theme,
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
                            div().flex().child(div().w(px(80.0))).child(render_axis(
                                &freq_scale,
                                &AxisConfig::bottom()
                                    .with_tick_values(freq_ticks)
                                    .with_formatter(|log_f| {
                                        let f = log_f.exp();
                                        if f >= 1000.0 {
                                            format!("{:.0}k", f / 1000.0)
                                        } else {
                                            format!("{:.0}", f)
                                        }
                                    })
                                    .with_title("Frequency (Hz)"),
                                chart_width,
                                &theme,
                            )),
                        ),
                )
                // Color legend
                .child({
                    let font_config = VectorFontConfig::horizontal(10.0, hsla(0.0, 0.0, 0.4, 1.0));
                    div()
                        .flex()
                        .items_center()
                        .gap_4()
                        .p_2()
                        .bg(rgb(0xf5f5f5))
                        .rounded_md()
                        .child(render_vector_text(
                            &format!("{:.0} dB", spl_min),
                            &font_config,
                        ))
                        .children((0..15).map(|i| {
                            let t = i as f64 / 14.0;
                            let color = colormap.color_scale()(t);
                            let (r, g, b) = (
                                (color.r * 255.0) as u32,
                                (color.g * 255.0) as u32,
                                (color.b * 255.0) as u32,
                            );
                            div()
                                .w(px(15.0))
                                .h(px(15.0))
                                .bg(rgb((r << 16) | (g << 8) | b))
                        }))
                        .child(render_vector_text(
                            &format!("{:.0} dB", spl_max),
                            &font_config,
                        ))
                }),
        )
    }
}

