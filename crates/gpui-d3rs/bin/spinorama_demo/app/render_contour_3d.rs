impl SpinoramaApp {
    /// Render 3D surface plot - SPL as a function of frequency and angle
    fn render_surface_3d_plot(&mut self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let ds = cx.design();
        let s = self.font_scale();
        let Some(ref contour_data) = self.contour_data else {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_size(px(ds.typography.base_size * s))
                    .text_color(theme.text_secondary)
                    .child("No contour data available for this speaker."),
            );
        };

        // Build surface data from contour data
        let freq_values = contour_data.freq.clone();
        let angle_values = contour_data.angles.clone();
        let spl_values = contour_data.spl.clone();
        let freq_count = contour_data.freq_count;
        let angle_count = contour_data.angle_count;

        // Reshape SPL values into 2D grid [angle][freq]
        let mut z_values = Vec::with_capacity(angle_count);
        for i in 0..angle_count {
            let start = i * freq_count;
            let end = start + freq_count;
            if end <= spl_values.len() {
                z_values.push(spl_values[start..end].to_vec());
            } else {
                // Should not happen if data is consistent, but handle gracefully
                z_values.push(vec![0.0; freq_count]);
            }
        }

        let surface_data = Surface3DData::from_grid(freq_values, angle_values, z_values)
            .with_log_x(true)
            .with_x_label("Frequency (Hz)")
            .with_x_range(100.0, 20000.0)
            .with_x_ticks(vec![
                100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0,
            ])
            .with_y_label("Angle (deg)")
            .with_y_ticks(vec![
                -180.0, -120.0, -60.0, 0.0, 60.0, 120.0, 180.0,
            ])
            .with_z_label("SPL (dB)")
            .with_z_range(-40.0, 10.0)
            .with_z_ticks(vec![-40.0, -30.0, -20.0, -10.0, 0.0, 10.0]);

        // Map colormap
        let colormap = match self.contour_colormap {
            Colormap::Viridis => Surface3DColormap::Viridis,
            Colormap::Plasma => Surface3DColormap::Plasma,
            Colormap::Magma => Surface3DColormap::Inferno,
            Colormap::Inferno => Surface3DColormap::Inferno,
            Colormap::Heat => Surface3DColormap::Inferno,
            Colormap::Coolwarm => Surface3DColormap::CoolWarm,
        };

        let bg = theme.surface;
        let config = Surface3DConfig::new()
            .colormap(colormap)
            .wireframe(self.surface_wireframe)
            .background_color(bg.r, bg.g, bg.b)
            .opacity(self.surface_opacity)
            .isolines(self.surface_isolines)
            .show_grid(self.surface_show_grid)
            .plot_type(SurfacePlotType::Cartesian) // Set plot type
            .camera_position(
                5.0,
                self.surface_rotation_azimuth,
                self.surface_rotation_elevation,
            );

        // Create element with shared state
        let surface_element =
            Surface3DElement::new(surface_data, config).with_state(self.surface_state.clone());

        // Colormap selector
        let colormaps = [
            (Colormap::Viridis, "Viridis"),
            (Colormap::Plasma, "Plasma"),
            (Colormap::Magma, "Magma"),
            (Colormap::Inferno, "Inferno"),
            (Colormap::Heat, "Heat"),
            (Colormap::Coolwarm, "Coolwarm"),
        ];

        let colormap_selector = div()
            .flex()
            .flex_row()
            .gap(px(ds.spacing.control_gap))
            .children(colormaps.iter().map(|&(cm, label)| {
                div()
                    .id(ElementId::Name(format!("cmap-{}", label).into()))
                    .px(px(ds.spacing.control_padding_x))
                    .py(px(ds.spacing.control_padding_y * 0.5))
                    .rounded(px(ds.corners.sm))
                    .cursor_pointer()
                    .when(self.contour_colormap == cm, |el| {
                        el.bg(theme.accent).text_color(theme.text_on_accent)
                    })
                    .when(self.contour_colormap != cm, |el| {
                        el.bg(theme.muted).text_color(theme.text_secondary)
                    })
                    .child(label)
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.contour_colormap = cm;
                        cx.notify();
                    }))
            }));

        // Wireframe toggle
        let wireframe_toggle = div()
            .id("surface-wireframe-toggle")
            .px(px(ds.spacing.control_padding_x))
            .py(px(ds.spacing.control_padding_y * 0.5))
            .rounded(px(ds.corners.sm))
            .cursor_pointer()
            .when(self.surface_wireframe, |el| {
                el.bg(theme.accent).text_color(theme.text_on_accent)
            })
            .when(!self.surface_wireframe, |el| {
                el.bg(theme.muted).text_color(theme.text_secondary)
            })
            .child("Wireframe")
            .on_click(cx.listener(|this, _, _window, cx| {
                this.surface_wireframe = !this.surface_wireframe;
                cx.notify();
            }));

        // Isolines toggle
        let isolines_toggle = div()
            .id("surface-isolines-toggle-3d")
            .px(px(ds.spacing.control_padding_x))
            .py(px(ds.spacing.control_padding_y * 0.5))
            .rounded(px(ds.corners.sm))
            .cursor_pointer()
            .text_size(px(ds.typography.small_size * s))
            .bg(if self.surface_isolines {
                theme.accent
            } else {
                theme.muted
            })
            .text_color(if self.surface_isolines {
                theme.text_on_accent
            } else {
                theme.text_secondary
            })
            .child("Isolines")
            .on_click(cx.listener(|this, _, _window, cx| {
                this.surface_isolines = !this.surface_isolines;
                cx.notify();
            }));

        // Grid toggle
        let grid_toggle = div()
            .id("surface-grid-toggle-3d")
            .px(px(ds.spacing.control_padding_x))
            .py(px(ds.spacing.control_padding_y * 0.5))
            .rounded(px(ds.corners.sm))
            .cursor_pointer()
            .text_size(px(ds.typography.small_size * s))
            .bg(if self.surface_show_grid {
                theme.accent
            } else {
                theme.muted
            })
            .text_color(if self.surface_show_grid {
                theme.text_on_accent
            } else {
                theme.text_secondary
            })
            .child("Grid")
            .on_click(cx.listener(|this, _, _window, cx| {
                this.surface_show_grid = !this.surface_show_grid;
                cx.notify();
            }));

        // Opacity slider using gpui-ui-kit Slider
        let entity = cx.entity().clone();
        let opacity_slider = gpui_ui_kit::Slider::new("opacity-slider-3d")
            .value(self.surface_opacity * 100.0)
            .min(0.0)
            .max(100.0)
            .step(5.0)
            .width(120.0)
            .label("Opacity")
            .show_value(true)
            .on_change(move |value, _window, cx| {
                entity.update(cx, |this, cx| {
                    this.surface_opacity = value / 100.0;
                    cx.notify();
                });
            });

        // Interactive container for the 3D view — fills remaining space
        let surface_view = div()
            .id("surface-3d-view")
            .flex_1()
            .w_full()
            .bg(theme.surface)
            .child(surface_element)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    let mut state = this.surface_state.borrow_mut();
                    state.dragging = true;
                    state.last_mouse = Some(event.position);
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    let mut state = this.surface_state.borrow_mut();
                    state.dragging = false;
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                let mut state = this.surface_state.borrow_mut();
                if let Some(last) = state.last_mouse {
                    let delta_x: f32 = event.position.x.into();
                    let delta_y: f32 = event.position.y.into();
                    let last_x: f32 = last.x.into();
                    let last_y: f32 = last.y.into();
                    let dx = delta_x - last_x;
                    let dy = delta_y - last_y;

                    if state.dragging {
                        state.controls.rotate(dx, dy);
                        state.update_camera();
                        cx.notify();
                    }
                }
                if state.dragging {
                    state.last_mouse = Some(event.position);
                }
            }))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                let mut state = this.surface_state.borrow_mut();
                let delta = match event.delta {
                    ScrollDelta::Lines(lines) => lines.y * 0.5,
                    ScrollDelta::Pixels(pixels) => {
                        let py: f32 = pixels.y.into();
                        py * 0.01
                    }
                };
                state.controls.zoom(delta);
                state.update_camera();
                cx.notify();
            }));

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(ds.spacing.control_gap))
            .child(
                div()
                    .text_size(px(ds.typography.large_size * s))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(format!(
                        "3D Surface - {}",
                        self.selected_speaker.as_deref().unwrap_or("Unknown")
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(ds.spacing.section_gap))
                    .items_center()
                    .child(div().text_size(px(ds.typography.small_size * s)).text_color(theme.text_secondary).child("Colormap:"))
                    .child(colormap_selector)
                    .child(wireframe_toggle)
                    .child(isolines_toggle)
                    .child(grid_toggle)
                    .child(opacity_slider),
            )
            .child(surface_view)

    }
}
