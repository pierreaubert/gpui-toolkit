impl SpinoramaApp {
fn render_cea2034_plot(&mut self, cx: &mut Context<Self>) -> Div {
    let colors = cea2034_colors();

    let chart_width = 800.0;
    let chart_height = 400.0;

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
        .filter_map(|&name| {
            let curve = self.cea2034_curves.get(name)?;
            let color = colors
                .get(name)
                .cloned()
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
        .filter_map(|&name| {
            let curve = self.cea2034_curves.get(name)?;
            let color = colors
                .get(name)
                .cloned()
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
        domain: (-5.0, 45.0),
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
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(0x333333))
                .child(format!(
                    "CEA2034 - {}",
                    self.selected_speaker.as_deref().unwrap_or("Unknown")
                )),
        )
        .child(interactive_chart)
    // Zoom status indicator
        .when(self.freq_spl_zoom.is_zoomed(), |el| {
            el.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child("Zoomed (double-click to reset)"),
            )
        })
        .child(self.render_legend(&colors))
}
}

