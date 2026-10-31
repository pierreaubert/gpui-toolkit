impl SpinoramaApp {
fn render_cea2034_plot(&mut self, cx: &mut Context<Self>) -> Div {
    let theme = cx.theme();
    let ds = cx.design();
    let s = self.font_scale();
    let mut colors = cea2034_colors();
    // Static rendering: prefer the trace colors from the source payload
    // so generated images match the site palette (lines and legend).
    for (name, color) in &self.trace_colors {
        if let Some(key) = colors.keys().find(|key| **key == name.as_str()).copied() {
            colors.insert(key, *color);
        }
    }

    let chart_width = self.content_width;
    let height_fraction = if self.chart_only { 0.78 } else { 0.6 };
    let chart_height = (chart_width * 0.5).min(self.content_height * height_fraction);

    // Separate DI curves from SPL curves
    let spl_curve_names = [
        "On Axis",
        "Listening Window",
        "Early Reflections",
        "Sound Power",
    ];
    let di_curve_names = ["Early Reflections DI", "Sound Power DI"];

    // Build PlotCurve list for SPL curves (primary axis)
    let mut plot_curves: Vec<PlotCurve> = spl_curve_names
        .iter()
        .filter(|&&name| !self.hidden_cea2034_curves.contains(name))
        .filter_map(|&name| {
            let curve = self.cea2034_curves.get(name)?;
            let color = colors
                .get(name)
                .copied()
                .unwrap_or(D3Color::rgb(128, 128, 128));
            let points: Vec<LinePoint> = curve
                .freq
                .iter()
                .zip(curve.spl.iter())
                .filter(|&(&f, _)| (20.0..=20000.0).contains(&f))
                .map(|(&f, &spl)| LinePoint::new(f, spl))
                .collect();
            if points.is_empty() {
                return None;
            }
            Some(PlotCurve::new(points, color))
        })
        .collect();

    // Add DI curves (secondary axis)
    let di_curves: Vec<PlotCurve> = di_curve_names
        .iter()
        .filter(|&&name| !self.hidden_cea2034_curves.contains(name))
        .filter_map(|&name| {
            let curve = self.cea2034_curves.get(name)?;
            let color = colors
                .get(name)
                .copied()
                .unwrap_or(D3Color::rgb(128, 128, 128));
            let points: Vec<LinePoint> = curve
                .freq
                .iter()
                .zip(curve.spl.iter())
                .filter(|&(&f, _)| (20.0..=20000.0).contains(&f))
                .map(|(&f, &spl)| LinePoint::new(f, spl))
                .collect();
            if points.is_empty() {
                return None;
            }
            Some(PlotCurve::new(points, color).secondary_axis())
        })
        .collect();
    plot_curves.extend(di_curves);

    // Configure secondary axis for DI curves
    // Note: Only include tick values up to 20 for labels (full domain is -5 to 45)
    let secondary_axis = Some(SecondaryAxisConfig {
        domain: self.di_domain,
        title: "DI (dB)",
        tick_values: vec![-5.0, 0.0, 5.0, 10.0, 15.0, 20.0], // Only show labels up to 20
    });

    // Create the chart
    let chart = render_freq_spl_plot(
        plot_curves,
        self.freq_spl_zoom.x_domain(), // Frequency domain (zoomed)
        self.freq_spl_zoom.y_domain(), // SPL domain (zoomed)
        secondary_axis,
        chart_width,
        chart_height,
        self.freq_spl_brush
            .current_selection()
            .map(|sel| BrushOverlay { selection: sel }),
        &theme,
    );

    // Wrap with interactive handlers
    let interactive_chart = self.wrap_freq_spl_chart_interactive(
        chart,
        ChartId::FreqSpl,
        chart_width,
        chart_height,
        cx,
    );

    div()
        .flex()
        .flex_col()
        .gap(px(ds.spacing.section_gap * 1.5))
        .child(
            div()
                .text_size(px(ds.typography.large_size * s))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.text_primary)
                .child(self.title_override.clone().unwrap_or_else(|| {
                    format!(
                        "CEA2034 - {}",
                        self.selected_speaker.as_deref().unwrap_or("Unknown")
                    )
                })),
        )
        .child(interactive_chart)
    // Zoom status indicator
        .when(self.freq_spl_zoom.is_zoomed(), |el| {
            el.child(
                div()
                    .text_size(px(ds.typography.small_size * s))
                    .text_color(theme.text_secondary)
                    .child("Zoomed (double-click to reset)"),
            )
        })
        .child(self.render_legend(&colors, &ds, &theme, cx))
}
}
