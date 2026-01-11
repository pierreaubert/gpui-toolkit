impl SpinoramaApp {
    /// Render polar contour plot - contour in polar coordinates (r=frequency, θ=angle)
    fn render_polar_contour_plot(&mut self, cx: &mut Context<Self>) -> Div {
        let Some(ref contour_data) = self.contour_data else {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No contour data available for this speaker."),
            );
        };

        let chart_size = 600.0_f32;
        let center_x = chart_size / 2.0;
        let center_y = chart_size / 2.0;
        let outer_radius = (chart_size / 2.0) - 80.0;

        // Frequency range (logarithmic radial axis)
        let freq_min = self.polar_contour_freq_range.0.max(20.0);
        let freq_max = self.polar_contour_freq_range.1.min(20000.0);
        let log_freq_min = freq_min.ln();
        let log_freq_max = freq_max.ln();

        // Color scale
        let color_scale = self.contour_colormap.color_scale();

        // Find SPL range for color normalization
        let spl_min = contour_data
            .spl
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let spl_max = contour_data
            .spl
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let spl_range = (spl_max - spl_min).max(1.0);

        // Generate polar heatmap data - store wedge geometry and colors
        let angular_resolution = 72_usize; // 5° steps
        let radial_resolution = 50_usize;

        // Build wedge data for canvas rendering
        #[derive(Clone)]
        struct WedgeData {
            r_inner: f32,
            r_outer: f32,
            theta1: f32,
            theta2: f32,
            color: Rgba,
        }

        let mut wedges: Vec<WedgeData> = Vec::with_capacity(angular_resolution * radial_resolution);

        let angle_min_data = contour_data.angles.first().copied().unwrap_or(-180.0);
        let angle_max_data = contour_data.angles.last().copied().unwrap_or(180.0);

        for r_idx in 0..radial_resolution {
            for a_idx in 0..angular_resolution {
                // Calculate frequency for this radial position (logarithmic)
                let r_t = r_idx as f64 / radial_resolution as f64;
                let r_t_next = (r_idx + 1) as f64 / radial_resolution as f64;

                let log_freq = log_freq_min + r_t * (log_freq_max - log_freq_min);
                let freq = log_freq.exp();

                // Calculate angle (0-360)
                let angle_t = a_idx as f64 / angular_resolution as f64;
                let angle_t_next = (a_idx + 1) as f64 / angular_resolution as f64;

                // Map to data angle range
                let angle = angle_min_data + angle_t * (angle_max_data - angle_min_data);

                // Find nearest indices in contour data
                let freq_idx = contour_data
                    .freq
                    .iter()
                    .position(|&f| f >= freq)
                    .unwrap_or(contour_data.freq_count - 1)
                    .min(contour_data.freq_count - 1);

                let angle_idx = contour_data
                    .angles
                    .iter()
                    .position(|&a| a >= angle)
                    .unwrap_or(contour_data.angle_count - 1)
                    .min(contour_data.angle_count - 1);

                let idx = angle_idx * contour_data.freq_count + freq_idx;
                let spl = contour_data.spl.get(idx).copied().unwrap_or(0.0);

                // Normalize SPL to color
                let color_t = ((spl - spl_min) / spl_range).clamp(0.0, 1.0);
                let color = color_scale(color_t);

                // Calculate radii (inner and outer)
                let r_inner = (r_t * outer_radius as f64) as f32;
                let r_outer = (r_t_next * outer_radius as f64) as f32;

                // Calculate angles (convert to radians, 0° at top)
                let theta1 = ((angle_t * 360.0 - 90.0).to_radians()) as f32;
                let theta2 = ((angle_t_next * 360.0 - 90.0).to_radians()) as f32;

                wedges.push(WedgeData {
                    r_inner,
                    r_outer,
                    theta1,
                    theta2,
                    color: color.to_rgba(),
                });
            }
        }

        // Grid frequencies for overlay
        let grid_frequencies = [100.0_f64, 1000.0, 10000.0];
        let grid_radii: Vec<f32> = grid_frequencies
            .iter()
            .filter(|&&f| f >= freq_min && f <= freq_max)
            .map(|&f| {
                let t = (f.ln() - log_freq_min) / (log_freq_max - log_freq_min);
                (t * outer_radius as f64) as f32
            })
            .collect();

        let grid_angles: Vec<f32> = (0..12)
            .map(|i| ((i as f64 * 30.0).to_radians()) as f32)
            .collect();

        // Clone data for canvas closure
        let wedges_clone = wedges.clone();
        let grid_radii_clone = grid_radii.clone();
        let grid_angles_clone = grid_angles.clone();

        // Canvas-based polar contour plot
        let polar_canvas = canvas(
            move |_bounds, _window, _cx| {
                (
                    wedges_clone.clone(),
                    grid_radii_clone.clone(),
                    grid_angles_clone.clone(),
                    center_x,
                    center_y,
                    outer_radius,
                )
            },
            move |bounds, (wedge_data, radii, angles, cx_f, cy_f, outer_r), window, _cx| {
                let origin_x: f32 = bounds.origin.x.into();
                let origin_y: f32 = bounds.origin.y.into();

                // Draw wedges (filled quads approximating arcs)
                for wedge in &wedge_data {
                    // For small angular segments, approximate with a quad
                    let x1_inner = origin_x + cx_f + wedge.r_inner * wedge.theta1.cos();
                    let y1_inner = origin_y + cy_f + wedge.r_inner * wedge.theta1.sin();
                    let x2_inner = origin_x + cx_f + wedge.r_inner * wedge.theta2.cos();
                    let y2_inner = origin_y + cy_f + wedge.r_inner * wedge.theta2.sin();
                    let x1_outer = origin_x + cx_f + wedge.r_outer * wedge.theta1.cos();
                    let y1_outer = origin_y + cy_f + wedge.r_outer * wedge.theta1.sin();
                    let x2_outer = origin_x + cx_f + wedge.r_outer * wedge.theta2.cos();
                    let y2_outer = origin_y + cy_f + wedge.r_outer * wedge.theta2.sin();

                    // Build a filled quad path
                    let mut builder = PathBuilder::fill();
                    builder.move_to(point(px(x1_inner), px(y1_inner)));
                    builder.line_to(point(px(x1_outer), px(y1_outer)));
                    builder.line_to(point(px(x2_outer), px(y2_outer)));
                    builder.line_to(point(px(x2_inner), px(y2_inner)));
                    builder.line_to(point(px(x1_inner), px(y1_inner))); // Close

                    if let Ok(path) = builder.build() {
                        window.paint_path(path, wedge.color);
                    }
                }

                // Draw grid circles
                for &r in &radii {
                    let num_segments = 72;
                    let mut builder = PathBuilder::stroke(px(1.0));
                    for i in 0..=num_segments {
                        let theta = (i as f32 / num_segments as f32) * std::f32::consts::TAU;
                        let x = origin_x + cx_f + r * theta.cos();
                        let y = origin_y + cy_f + r * theta.sin();
                        if i == 0 {
                            builder.move_to(point(px(x), px(y)));
                        } else {
                            builder.line_to(point(px(x), px(y)));
                        }
                    }
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, hsla(0.0, 0.0, 1.0, 0.4));
                    }
                }

                // Draw grid rays
                for &angle in &angles {
                    let mut builder = PathBuilder::stroke(px(1.0));
                    let x1 = origin_x + cx_f;
                    let y1 = origin_y + cy_f;
                    let x2 = origin_x + cx_f + outer_r * angle.cos();
                    let y2 = origin_y + cy_f + outer_r * angle.sin();
                    builder.move_to(point(px(x1), px(y1)));
                    builder.line_to(point(px(x2), px(y2)));
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, hsla(0.0, 0.0, 1.0, 0.25));
                    }
                }
            },
        )
        .w(px(chart_size))
        .h(px(chart_size));

        // Frequency labels using render_vector_text
        let font_config = VectorFontConfig::horizontal(10.0, hsla(0.0, 0.0, 0.2, 1.0));
        let freq_labels = div().absolute().inset_0().children(
            grid_frequencies
                .iter()
                .filter(|&&f| f >= freq_min && f <= freq_max)
                .map(|&freq| {
                    let t = (freq.ln() - log_freq_min) / (log_freq_max - log_freq_min);
                    let r = (t * outer_radius as f64) as f32;
                    let x = center_x;
                    let y = center_y - r - 8.0;

                    div()
                        .absolute()
                        .left(px(x - 15.0))
                        .top(px(y))
                        .child(render_vector_text(&format_frequency(freq), &font_config))
                }),
        );

        // Angle labels
        let angle_font_config = VectorFontConfig::horizontal(10.0, hsla(0.0, 0.0, 0.4, 1.0));
        let angle_labels = div().absolute().inset_0().children((0..12).map(|i| {
            let angle_deg = i as f64 * 30.0;
            let angle_rad = (angle_deg - 90.0).to_radians();
            let label_radius = outer_radius + 25.0;
            let x = center_x + label_radius * angle_rad.cos() as f32;
            let y = center_y + label_radius * angle_rad.sin() as f32;

            // Map display angle to data angle convention
            let display_angle =
                angle_min_data + (angle_deg / 360.0) * (angle_max_data - angle_min_data);

            div()
                .absolute()
                .left(px(x - 15.0))
                .top(px(y - 6.0))
                .child(render_vector_text(
                    &format!("{:.0}°", display_angle),
                    &angle_font_config,
                ))
        }));

        // Colorbar using div elements
        let colorbar_height = 200.0_f32;
        let colorbar_width = 20.0_f32;
        let num_color_steps = 20;

        let colorbar = div()
            .flex()
            .flex_col()
            .gap_1()
            .children((0..num_color_steps).map(|i| {
                let t = i as f64 / num_color_steps as f64;
                let color = color_scale(1.0 - t); // Invert so high values at top
                let h = colorbar_height / num_color_steps as f32;

                div().w(px(colorbar_width)).h(px(h)).bg(color.to_rgba())
            }));

        // Colorbar labels
        let colorbar_label_font = VectorFontConfig::horizontal(10.0, hsla(0.0, 0.0, 0.4, 1.0));
        let colorbar_labels = div()
            .flex()
            .flex_col()
            .justify_between()
            .h(px(colorbar_height))
            .children([0.0, 0.5, 1.0].iter().map(|&t| {
                let spl_val = spl_min + t * spl_range;
                div().child(render_vector_text(
                    &format!("{:.0} dB", spl_val),
                    &colorbar_label_font,
                ))
            }));

        // Colormap selector
        let colormaps = [
            (Colormap::Viridis, "Viridis"),
            (Colormap::Plasma, "Plasma"),
            (Colormap::Magma, "Magma"),
            (Colormap::Inferno, "Inferno"),
            (Colormap::Heat, "Heat"),
            (Colormap::Coolwarm, "Coolwarm"),
        ];

        let colormap_selector =
            div()
                .flex()
                .flex_row()
                .gap_2()
                .children(colormaps.iter().enumerate().map(|(i, &(cmap, label))| {
                    div()
                        .id(ElementId::NamedInteger(
                            "polar-contour-colormap".into(),
                            i as u64,
                        ))
                        .px_3()
                        .py_1()
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .when(self.contour_colormap == cmap, |el| {
                            el.bg(rgb(0x3b82f6)).text_color(rgb(0xffffff))
                        })
                        .when(self.contour_colormap != cmap, |el| {
                            el.bg(rgb(0xe5e7eb)).text_color(rgb(0x666666))
                        })
                        .child(label)
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            this.contour_colormap = cmap;
                            cx.notify();
                        }))
                }));

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
                        "Polar Contour - {}",
                        self.selected_speaker.as_deref().unwrap_or("Unknown")
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .items_center()
                    .child(div().text_sm().text_color(rgb(0x666666)).child("Colormap:"))
                    .child(colormap_selector),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_center()
                    .gap_8()
                    .child(
                        div()
                            .relative()
                            .w(px(chart_size))
                            .h(px(chart_size))
                            .child(polar_canvas)
                            .child(freq_labels)
                            .child(angle_labels),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_2()
                            .child(div().text_sm().text_color(rgb(0x666666)).child("SPL (dB)"))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(colorbar)
                                    .child(colorbar_labels),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_center()
                    .gap_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("Radial: Frequency (log scale)"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("Angular: Angle (degrees)"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("Color: SPL (dB)"),
                    ),
            )
    }
}
