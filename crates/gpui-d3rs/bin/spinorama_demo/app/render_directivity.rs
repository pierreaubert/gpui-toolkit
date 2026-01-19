impl SpinoramaApp {
    fn render_directivity_plot(&mut self, plane: &str, _cx: &mut Context<Self>) -> Div {
        // Create a viridis-like color palette for directivity
        let viridis_colors = vec![
            D3Color::from_hex(0x440154), // Dark purple
            D3Color::from_hex(0x414487), // Purple-blue
            D3Color::from_hex(0x2a788e), // Teal
            D3Color::from_hex(0x22a884), // Green-teal
            D3Color::from_hex(0x7ad151), // Light green
            D3Color::from_hex(0xfde725), // Yellow
        ];

        let Some(ref directivity) = self.directivity_data else {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No directivity data available for this speaker."),
            );
        };

        let curves = if plane == "horizontal" {
            &directivity.horizontal
        } else {
            &directivity.vertical
        };

        if curves.is_empty() {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child(format!("No {} directivity data available.", plane)),
            );
        }

        let chart_width = 800.0;
        let chart_height = 400.0;

        // Generate colors for different angles and build PlotCurve list
        let num_curves = curves.len();
        let plot_curves: Vec<PlotCurve> = curves
            .iter()
            .enumerate()
            .map(|(i, curve)| {
                let t = i as f32 / (num_curves.max(1) - 1).max(1) as f32;
                let color = d3rs::color::interpolate_colors(&viridis_colors, t);

                let points: Vec<LinePoint> = curve
                    .freq
                    .iter()
                    .zip(curve.spl.iter())
                    .filter(|&(&f, _)| (20.0..=20000.0).contains(&f))
                    .map(|(&f, &spl)| LinePoint::new(f, spl))
                    .collect();

                PlotCurve::new(points, color).stroke_width(1.5)
            })
            .collect();

        // Get angle range for legend
        let angle_min = curves.first().map(|c| c.angle).unwrap_or(-60.0);
        let angle_max = curves.last().map(|c| c.angle).unwrap_or(60.0);

        // Create the chart
        let chart = render_freq_spl_plot(
            plot_curves,
            self.freq_spl_zoom.x_domain(),
            self.freq_spl_zoom.y_domain(),
            None, // No secondary axis for directivity plots
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
            _cx,
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
                        "{} SPL - {}",
                        if plane == "horizontal" {
                            "Horizontal"
                        } else {
                            "Vertical"
                        },
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
            // Angle legend
            .child({
                let font_config = VectorFontConfig::horizontal(10.0, hsla(0.0, 0.0, 0.4, 1.0));

                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_4()
                    .bg(rgb(0xf5f5f5))
                    .rounded_md()
                    .child(render_vector_text(
                        &format!("{:.0}°", angle_min),
                        &font_config,
                    ))
                    // Simplified gradient legend (using color strip segments)
                    .children((0..6).map(|i| {
                        let color =
                            d3rs::color::interpolate_colors(&viridis_colors, i as f32 / 5.0);
                        let (r, g, b) = (
                            (color.r * 255.0) as u32,
                            (color.g * 255.0) as u32,
                            (color.b * 255.0) as u32,
                        );
                        div().flex_1().h(px(16.0)).bg(rgb((r << 16) | (g << 8) | b))
                    }))
                    .child(render_vector_text(
                        &format!("{:.0}°", angle_max),
                        &font_config,
                    ))
            })
    }
}

