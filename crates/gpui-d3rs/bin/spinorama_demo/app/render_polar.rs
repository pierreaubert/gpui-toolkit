impl SpinoramaApp {
    /// Render polar directivity plot - SPL vs angle at selected frequencies
    fn render_polar_directivity_plot(&mut self, cx: &mut Context<Self>) -> Div {
        let Some(ref directivity) = self.directivity_data else {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No directivity data available for this speaker."),
            );
        };

        let chart_size = 500.0_f32;
        let center_x = chart_size / 2.0;
        let center_y = chart_size / 2.0;
        let outer_radius = (chart_size / 2.0) - 60.0; // Leave margin for labels

        // SPL range for normalization (0 dB at outer edge, -30 dB at center)
        let spl_max = 0.0_f64;
        let spl_min = -30.0_f64;
        let spl_range = spl_max - spl_min;

        // Get angle range from data
        let use_horizontal = self.polar_plane == DirectivityPlane::Horizontal;
        let (_angle_min, _angle_max) = get_angle_range(directivity, use_horizontal);

        // Colors for different frequencies (use viridis-like palette)
        let freq_colors = [
            D3Color::from_hex(0x440154), // Dark purple
            D3Color::from_hex(0x3b528b), // Blue-purple
            D3Color::from_hex(0x21918c), // Teal
            D3Color::from_hex(0x5ec962), // Green
            D3Color::from_hex(0xfde725), // Yellow
        ];

        // Build paths for each selected frequency
        type PolarPathData = (f64, Vec<(f32, f32)>, D3Color);
        let mut frequency_paths: Vec<PolarPathData> = Vec::new();

        for (i, &freq) in self.polar_selected_frequencies.iter().take(5).enumerate() {
            let color = freq_colors[i % freq_colors.len()];

            // Get SPL at each angle for this frequency
            let angle_spl_pairs = interpolate_spl_at_frequency(directivity, freq, use_horizontal);

            if angle_spl_pairs.is_empty() {
                continue;
            }

            // Convert to screen coordinates
            let points: Vec<(f32, f32)> = angle_spl_pairs
                .iter()
                .map(|&(angle_deg, spl)| {
                    // Convert angle: 0° at top means -PI/2 offset
                    let angle_rad = (angle_deg - 90.0).to_radians();
                    // Normalize SPL to radius (spl_max -> outer_radius, spl_min -> 0)
                    let normalized_spl = ((spl - spl_min) / spl_range).clamp(0.0, 1.0);
                    let radius = normalized_spl as f32 * outer_radius;
                    let x = center_x + radius * angle_rad.cos() as f32;
                    let y = center_y + radius * angle_rad.sin() as f32;
                    (x, y)
                })
                .collect();

            frequency_paths.push((freq, points, color));
        }

        // Generate polar grid paths as screen coordinates
        let grid_radii: Vec<f32> = [0.25, 0.5, 0.75, 1.0]
            .iter()
            .map(|&t| t * outer_radius)
            .collect();

        let grid_angles: Vec<f64> = (0..12).map(|i| (i as f64 * 30.0).to_radians()).collect();

        // Clone data for the canvas closure
        let frequency_paths_clone = frequency_paths.clone();
        let grid_radii_clone = grid_radii.clone();
        let grid_angles_clone = grid_angles.clone();

        // Canvas-based polar plot
        let polar_canvas = canvas(
            move |_bounds, _window, _cx| {
                // Prepaint: just pass through the data
                (
                    frequency_paths_clone.clone(),
                    grid_radii_clone.clone(),
                    grid_angles_clone.clone(),
                    center_x,
                    center_y,
                    outer_radius,
                )
            },
            move |bounds, (freq_paths, radii, angles, cx_f, cy_f, outer_r), window, _cx| {
                let origin_x: f32 = bounds.origin.x.into();
                let origin_y: f32 = bounds.origin.y.into();

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
                        window.paint_path(path, hsla(0.0, 0.0, 0.8, 1.0));
                    }
                }

                // Draw grid rays
                for &angle in &angles {
                    let mut builder = PathBuilder::stroke(px(1.0));
                    let x1 = origin_x + cx_f;
                    let y1 = origin_y + cy_f;
                    let x2 = origin_x + cx_f + outer_r * angle.cos() as f32;
                    let y2 = origin_y + cy_f + angle.sin() as f32 * outer_r;
                    builder.move_to(point(px(x1), px(y1)));
                    builder.line_to(point(px(x2), px(y2)));
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, hsla(0.0, 0.0, 0.85, 1.0));
                    }
                }

                // Draw frequency curves
                for (_, points, color) in &freq_paths {
                    if points.len() < 2 {
                        continue;
                    }
                    let mut builder = PathBuilder::stroke(px(2.0));
                    for (i, &(x, y)) in points.iter().enumerate() {
                        let screen_x = origin_x + x;
                        let screen_y = origin_y + y;
                        if i == 0 {
                            builder.move_to(point(px(screen_x), px(screen_y)));
                        } else {
                            builder.line_to(point(px(screen_x), px(screen_y)));
                        }
                    }
                    // Close the path
                    if let Some(&(x, y)) = points.first() {
                        builder.line_to(point(px(origin_x + x), px(origin_y + y)));
                    }
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, color.to_rgba());
                    }
                }
            },
        )
        .w(px(chart_size))
        .h(px(chart_size));

        // Build legend
        let legend =
            div()
                .flex()
                .flex_row()
                .gap_4()
                .justify_center()
                .children(frequency_paths.iter().map(|(freq, _, color)| {
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .w(px(16.0))
                                .h(px(3.0))
                                .bg(color.to_rgba())
                                .rounded(px(1.0)),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x666666))
                                .child(format_frequency(*freq).to_string()),
                        )
                }));

        // Angle labels using render_vector_text
        let font_config = VectorFontConfig::horizontal(10.0, hsla(0.0, 0.0, 0.4, 1.0));
        let angle_labels = div().absolute().inset_0().children((0..12).map(|i| {
            let angle_deg = i as f64 * 30.0;
            let angle_rad = (angle_deg - 90.0).to_radians();
            let label_radius = outer_radius + 25.0;
            let x = center_x + label_radius * angle_rad.cos() as f32;
            let y = center_y + label_radius * angle_rad.sin() as f32;

            let display_angle = if angle_deg <= 180.0 {
                angle_deg as i32
            } else {
                (angle_deg - 360.0) as i32
            };

            div()
                .absolute()
                .left(px(x - 15.0))
                .top(px(y - 6.0))
                .child(render_vector_text(
                    &format!("{}°", display_angle),
                    &font_config,
                ))
        }));

        // dB labels on radial axis
        let db_font_config = VectorFontConfig::horizontal(9.0, hsla(0.0, 0.0, 0.6, 1.0));
        let db_labels = div()
            .absolute()
            .inset_0()
            .children([0.0, -10.0, -20.0, -30.0].iter().map(|&db| {
                let normalized = (db - spl_min) / spl_range;
                let r = normalized as f32 * outer_radius;
                let x = center_x;
                let y = center_y - r - 8.0;

                div()
                    .absolute()
                    .left(px(x - 20.0))
                    .top(px(y))
                    .child(render_vector_text(
                        &format!("{} dB", db as i32),
                        &db_font_config,
                    ))
            }));

        // Frequency selection controls
        let available_frequencies: Vec<f64> = vec![
            100.0, 200.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 10000.0, 16000.0,
        ];

        let freq_selector = div().flex().flex_row().flex_wrap().gap_2().children(
            available_frequencies.iter().enumerate().map(|(i, &freq)| {
                let is_selected = self.polar_selected_frequencies.contains(&freq);
                let freq_clone = freq;
                div()
                    .id(ElementId::NamedInteger("polar-freq".into(), i as u64))
                    .px_3()
                    .py_1()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .border_1()
                    .when(is_selected, |el| {
                        el.bg(rgb(0x3b82f6))
                            .text_color(rgb(0xffffff))
                            .border_color(rgb(0x3b82f6))
                    })
                    .when(!is_selected, |el| {
                        el.bg(rgb(0xffffff))
                            .text_color(rgb(0x666666))
                            .border_color(rgb(0xdddddd))
                    })
                    .child(format_frequency(freq))
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        if this.polar_selected_frequencies.contains(&freq_clone) {
                            this.polar_selected_frequencies.retain(|&f| f != freq_clone);
                        } else if this.polar_selected_frequencies.len() < 5 {
                            this.polar_selected_frequencies.push(freq_clone);
                            this.polar_selected_frequencies
                                .sort_by(|a, b| a.partial_cmp(b).unwrap());
                        }
                        cx.notify();
                    }))
            }),
        );

        // Plane toggle
        let plane_toggle = div()
            .flex()
            .flex_row()
            .gap_2()
            .child(
                div()
                    .id("polar-plane-horizontal")
                    .px_3()
                    .py_1()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(self.polar_plane == DirectivityPlane::Horizontal, |el| {
                        el.bg(rgb(0x3b82f6)).text_color(rgb(0xffffff))
                    })
                    .when(self.polar_plane != DirectivityPlane::Horizontal, |el| {
                        el.bg(rgb(0xe5e7eb)).text_color(rgb(0x666666))
                    })
                    .child("Horizontal")
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.polar_plane = DirectivityPlane::Horizontal;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("polar-plane-vertical")
                    .px_3()
                    .py_1()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(self.polar_plane == DirectivityPlane::Vertical, |el| {
                        el.bg(rgb(0x3b82f6)).text_color(rgb(0xffffff))
                    })
                    .when(self.polar_plane != DirectivityPlane::Vertical, |el| {
                        el.bg(rgb(0xe5e7eb)).text_color(rgb(0x666666))
                    })
                    .child("Vertical")
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.polar_plane = DirectivityPlane::Vertical;
                        cx.notify();
                    })),
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
                        "Polar Directivity - {}",
                        self.selected_speaker.as_deref().unwrap_or("Unknown")
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .items_center()
                    .child(div().text_sm().text_color(rgb(0x666666)).child("Plane:"))
                    .child(plane_toggle)
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .ml_4()
                            .child("Frequencies:"),
                    )
                    .child(freq_selector),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .relative()
                            .w(px(chart_size))
                            .h(px(chart_size))
                            .child(polar_canvas)
                            .child(angle_labels)
                            .child(db_labels),
                    )
                    .child(legend),
            )
    }
}
