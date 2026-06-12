#[derive(Clone)]
struct DirectivityLegendItem {
    angle: f64,
    key: String,
    color: D3Color,
    dashed: bool,
}

#[derive(Clone, Copy)]
struct DirectivityLineStyle {
    color: D3Color,
    dashed: bool,
    stroke_width: f32,
}

fn directivity_line_style(angle: f64, max_abs_angle: f64) -> DirectivityLineStyle {
    let palette = [
        D3Color::from_hex(0x4cc9f0), // cyan
        D3Color::from_hex(0x7bd88f), // green
        D3Color::from_hex(0xf4d35e), // yellow
        D3Color::from_hex(0xf28f3b), // orange
        D3Color::from_hex(0xee6352), // red
        D3Color::from_hex(0xc77dff), // violet
        D3Color::from_hex(0xff70a6), // pink
    ];
    let t = (angle.abs() / max_abs_angle.max(1.0)).clamp(0.0, 1.0) as f32;

    DirectivityLineStyle {
        color: d3rs::color::interpolate_colors(&palette, t),
        dashed: angle < -0.05,
        stroke_width: if angle.abs() < 0.05 { 2.25 } else { 1.8 },
    }
}

impl SpinoramaApp {
    fn render_directivity_plot(&mut self, plane: &str, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let ds = cx.design();
        let s = self.font_scale();
        let Some(ref directivity) = self.directivity_data else {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_size(px(ds.typography.base_size * s))
                    .text_color(theme.text_secondary)
                    .child("No directivity data available for this speaker."),
            );
        };

        let curves = if plane == "horizontal" {
            &directivity.horizontal
        } else {
            &directivity.vertical
        };

        let mut display_curves = curves
            .iter()
            .map(|curve| (curve.angle, &curve.freq, &curve.spl))
            .collect::<Vec<_>>();
        let has_on_axis = display_curves
            .iter()
            .any(|(angle, _, _)| angle.abs() < 0.5);
        if !has_on_axis && let Some(on_axis) = self.cea2034_curves.get("On Axis") {
            display_curves.push((0.0, &on_axis.freq, &on_axis.spl));
        }
        display_curves.sort_by(|(a, _, _), (b, _, _)| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        });

        if display_curves.is_empty() {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_size(px(ds.typography.base_size * s))
                    .text_color(theme.text_secondary)
                    .child(format!("No {} directivity data available.", plane)),
            );
        }

        let chart_width = self.content_width;
        let chart_height = (chart_width * 0.5).min(self.content_height * 0.6);

        // Generate colors for different angles and build PlotCurve list
        let max_abs_angle = display_curves
            .iter()
            .map(|(angle, _, _)| angle.abs())
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let mut visible_curves = Vec::with_capacity(display_curves.len());
        let mut legend_items = Vec::with_capacity(display_curves.len());
        for (angle, freq, spl) in display_curves {
            let style = directivity_line_style(angle, max_abs_angle);

            let points: Vec<LinePoint> = freq
                .iter()
                .zip(spl.iter())
                .filter(|&(&f, _)| (20.0..=20000.0).contains(&f))
                .map(|(&f, &spl)| LinePoint::new(f, spl))
                .collect();

            if points.is_empty() {
                continue;
            }

            let key = directivity_curve_key(plane, angle);
            legend_items.push(DirectivityLegendItem {
                angle,
                key: key.clone(),
                color: style.color,
                dashed: style.dashed,
            });
            if !self.hidden_directivity_curves.contains(&key) {
                let mut plot_curve =
                    PlotCurve::new(points, style.color).stroke_width(style.stroke_width);
                if style.dashed {
                    plot_curve = plot_curve.dash_array(StrokeDashArray::Dashed);
                }
                visible_curves.push((angle, plot_curve));
            }
        }
        visible_curves.sort_by(|(a, _), (b, _)| {
            let a_group = if *a < -0.05 { 1 } else { 0 };
            let b_group = if *b < -0.05 { 1 } else { 0 };
            a_group
                .cmp(&b_group)
                .then_with(|| a.abs().partial_cmp(&b.abs()).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        });
        let plot_curves: Vec<PlotCurve> = visible_curves
            .into_iter()
            .map(|(_, curve)| curve)
            .collect();

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
                        .text_size(px(ds.typography.small_size * s))
                        .text_color(theme.text_secondary)
                        .child("Zoomed (double-click to reset)"),
                )
            })
            .child(self.render_directivity_legend(&legend_items, &ds, &theme, cx))
    }

    fn render_directivity_legend(
        &self,
        legend_items: &[DirectivityLegendItem],
        ds: &DesignSystem,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Div {
        let font_size = (10.0 * self.font_scale()).round();
        let row_height = (font_size * 1.25).ceil().max(16.0);
        let marker_width = 18.0;
        let marker_height = 3.0;
        let marker_top = ((row_height - marker_height) * 0.5).round();

        div()
            .flex()
            .flex_wrap()
            .justify_center()
            .items_center()
            .gap(px(ds.spacing.section_gap))
            .p(px(ds.spacing.card_padding))
            .bg(theme.muted)
            .rounded(px(ds.corners.md))
            .children(legend_items.iter().map(|item| {
                let key = item.key.clone();
                let is_hidden = self.hidden_directivity_curves.contains(&key);
                let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
                let (r, g, b) = (
                    channel(item.color.r),
                    channel(item.color.g),
                    channel(item.color.b),
                );
                let marker_color = rgb((r << 16) | (g << 8) | b);
                let marker = div().relative().flex_none().w(px(marker_width)).h(px(row_height));
                let marker = if item.dashed {
                    marker.children((0..3).map(|index| {
                        div()
                            .absolute()
                            .left(px(index as f32 * 7.0))
                            .top(px(marker_top))
                            .w(px(4.0))
                            .h(px(marker_height))
                            .bg(marker_color)
                    }))
                } else {
                    marker.child(
                        div()
                            .absolute()
                            .left_0()
                            .top(px(marker_top))
                            .w(px(marker_width))
                            .h(px(marker_height))
                            .bg(marker_color),
                    )
                };

                div()
                    .flex()
                    .h(px(row_height))
                    .gap(px(ds.spacing.control_gap))
                    .rounded(px(ds.corners.sm))
                    .cursor_pointer()
                    .opacity(if is_hidden { 0.35 } else { 1.0 })
                    .hover(|el| el.bg(theme.surface_hover))
                    .child(marker)
                    .child(
                        div()
                            .h(px(row_height))
                            .line_height(px(row_height))
                            .text_size(px(font_size))
                            .text_color(theme.text_primary)
                            .child(format_directivity_angle(item.angle)),
                    )
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                        if !this.hidden_directivity_curves.insert(key.clone()) {
                            this.hidden_directivity_curves.remove(&key);
                        }
                        cx.notify();
                    }))
            }))
    }
}

fn directivity_curve_key(plane: &str, angle: f64) -> String {
    format!("{plane}:{angle:.3}")
}

fn format_directivity_angle(angle: f64) -> String {
    let rounded = angle.round();
    if (angle - rounded).abs() < 0.05 {
        format!("{rounded:.0}°")
    } else {
        format!("{angle:.1}°")
    }
}
