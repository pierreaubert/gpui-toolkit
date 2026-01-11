use autoeq::DirectivityData;
use d3rs::gpu3d::{
    Colormap as Surface3DColormap, Surface3DConfig, Surface3DElement, SurfaceData as Surface3DData,
    SurfacePlotType,
};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_ui_kit::Slider;

use super::SpinoramaApp;
use crate::types::Colormap;

impl SpinoramaApp {
    /// Interpolate SPL at a given frequency from a directivity curve.
    /// Returns SPL value interpolated between frequency points.
    fn interpolate_spl_at_freq(freq: &[f64], spl: &[f64], target_freq: f64) -> f64 {
        if freq.is_empty() || spl.is_empty() {
            return 0.0;
        }
        if freq.len() == 1 {
            return spl[0];
        }

        // Binary search for the bracket
        for i in 0..freq.len() - 1 {
            if target_freq >= freq[i] && target_freq <= freq[i + 1] {
                let t = (target_freq - freq[i]) / (freq[i + 1] - freq[i]);
                return spl[i] * (1.0 - t) + spl[i + 1] * t;
            }
        }

        // Out of range: return nearest
        if target_freq < freq[0] {
            spl[0]
        } else {
            spl[freq.len() - 1]
        }
    }

    /// Get SPL at a given angle from directivity curves at a specific frequency.
    /// Interpolates between measured angles.
    fn get_spl_at_angle(
        curves: &[autoeq_cea2034::DirectivityCurve],
        angle: f64,
        target_freq: f64,
    ) -> f64 {
        if curves.is_empty() {
            return 0.0;
        }

        // Build angle -> SPL mapping at target frequency
        let mut angle_spl: Vec<(f64, f64)> = curves
            .iter()
            .map(|c| {
                let spl = Self::interpolate_spl_at_freq(
                    c.freq.as_slice().unwrap_or(&[]),
                    c.spl.as_slice().unwrap_or(&[]),
                    target_freq,
                );
                (c.angle, spl)
            })
            .collect();

        // Sort by angle
        angle_spl.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        if angle_spl.len() == 1 {
            return angle_spl[0].1;
        }

        // Interpolate between angles
        for i in 0..angle_spl.len() - 1 {
            let (a0, spl0) = angle_spl[i];
            let (a1, spl1) = angle_spl[i + 1];

            if angle >= a0 && angle <= a1 {
                let t = (angle - a0) / (a1 - a0);
                return spl0 * (1.0 - t) + spl1 * t;
            }
        }

        // Out of range: return nearest
        if angle < angle_spl[0].0 {
            angle_spl[0].1
        } else {
            angle_spl[angle_spl.len() - 1].1
        }
    }

    /// Interpolate SPL for a point (h_angle, v_angle) on the sphere.
    ///
    /// We have:
    /// - Horizontal plane data: SPL at various h_angles, v_angle=0
    /// - Vertical plane data: SPL at various v_angles, h_angle=0
    ///
    /// For a point (h, v), we use additive interpolation in dB:
    /// SPL(h, v) = SPL_h(h) + SPL_v(v) - SPL(0, 0)
    ///
    /// This assumes the directivity pattern is separable.
    fn interpolate_sphere_spl(
        directivity: &DirectivityData,
        h_angle: f64,
        v_angle: f64,
        target_freq: f64,
    ) -> f64 {
        // Get SPL at (h_angle, 0) from horizontal data
        let spl_h = Self::get_spl_at_angle(&directivity.horizontal, h_angle, target_freq);

        // Get SPL at (0, v_angle) from vertical data
        let spl_v = Self::get_spl_at_angle(&directivity.vertical, v_angle, target_freq);

        // Get SPL at (0, 0) - on-axis reference (from either dataset)
        let spl_00 = Self::get_spl_at_angle(&directivity.horizontal, 0.0, target_freq);

        // Additive interpolation: SPL(h,v) = SPL(h,0) + SPL(0,v) - SPL(0,0)
        spl_h + spl_v - spl_00
    }

    /// Render 3D Sphere plot - SPL of a single frequency mapped to color.
    /// Uses both horizontal and vertical directivity data to interpolate
    /// SPL values across the entire sphere.
    pub fn render_sphere_plot(&mut self, cx: &mut Context<Self>) -> Div {
        let Some(ref directivity) = self.directivity_data else {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No directivity data available for this speaker."),
            );
        };

        // Check we have both horizontal and vertical data
        if directivity.horizontal.is_empty() || directivity.vertical.is_empty() {
            return div().flex().items_center().justify_center().h_full().child(
                div().text_base().text_color(rgb(0x666666)).child(
                    "Both horizontal and vertical directivity data required for sphere plot.",
                ),
            );
        }

        // Get frequency values from the first horizontal curve
        let freq_values: Vec<f64> = directivity.horizontal[0]
            .freq
            .as_slice()
            .unwrap_or(&[])
            .to_vec();
        let freq_count = freq_values.len();

        if freq_count == 0 {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
                    .child("No frequency data available."),
            );
        }

        // Ensure frequency index is valid
        if self.sphere_freq_idx >= freq_count {
            self.sphere_freq_idx = freq_count.saturating_sub(1);
        }
        let current_freq = freq_values[self.sphere_freq_idx];

        // Get angle ranges from data
        let h_angles: Vec<f64> = directivity.horizontal.iter().map(|c| c.angle).collect();
        let v_angles: Vec<f64> = directivity.vertical.iter().map(|c| c.angle).collect();

        let h_min = h_angles.iter().cloned().fold(f64::INFINITY, f64::min);
        let h_max = h_angles.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let v_min = v_angles.iter().cloned().fold(f64::INFINITY, f64::min);
        let v_max = v_angles.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Generate sphere grid
        // Use the actual measured angle ranges, extended to full sphere with extrapolation
        let h_steps = 73; // -180 to 180 in 5° steps
        let v_steps = 37; // -90 to 90 in 5° steps

        let h_values: Vec<f64> = (0..h_steps).map(|i| -180.0 + (i as f64) * 5.0).collect();

        let v_values: Vec<f64> = (0..v_steps).map(|i| -90.0 + (i as f64) * 5.0).collect();

        // Build Z values grid: z_values[h_idx][v_idx]
        let mut z_values: Vec<Vec<f64>> = Vec::with_capacity(h_steps);

        for &h in &h_values {
            let spl_column: Vec<f64> = v_values
                .iter()
                .map(|&v| {
                    // Clamp angles to measured range for interpolation
                    let h_clamped = h.clamp(h_min, h_max);
                    let v_clamped = v.clamp(v_min, v_max);
                    Self::interpolate_sphere_spl(directivity, h_clamped, v_clamped, current_freq)
                })
                .collect();
            z_values.push(spl_column);
        }

        // Configure Surface Data
        // X axis = Elevation (v_angle), Y axis = Azimuth (h_angle)
        let surface_data = Surface3DData::from_grid(v_values.clone(), h_values.clone(), z_values)
            .with_log_x(false)
            .with_x_label("Elevation (°)")
            .with_x_range(-90.0, 90.0)
            .with_x_ticks(vec![-90.0, -45.0, 0.0, 45.0, 90.0])
            .with_y_label("Azimuth (°)")
            .with_y_range(-180.0, 180.0)
            .with_y_ticks(vec![-180.0, -90.0, 0.0, 90.0, 180.0])
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

        let config = Surface3DConfig::new()
            .colormap(colormap)
            .wireframe(self.surface_wireframe)
            .background_color(1.0, 1.0, 1.0)
            .opacity(self.surface_opacity)
            .isolines(self.surface_isolines)
            .plot_type(SurfacePlotType::Spherical)
            .camera_position(
                3.5,
                self.surface_rotation_azimuth,
                self.surface_rotation_elevation,
            );

        let surface_element =
            Surface3DElement::new(surface_data, config).with_state(self.surface_state.clone());

        // Colormap UI
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
            .gap_2()
            .children(colormaps.iter().map(|&(cm, label)| {
                div()
                    .id(ElementId::Name(format!("cmap-{}", label).into()))
                    .px_3()
                    .py_1()
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .when(self.contour_colormap == cm, |el| {
                        el.bg(rgb(0x3b82f6)).text_color(rgb(0xffffff))
                    })
                    .when(self.contour_colormap != cm, |el| {
                        el.bg(rgb(0xe5e7eb)).text_color(rgb(0x666666))
                    })
                    .child(label)
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.contour_colormap = cm;
                        cx.notify();
                    }))
            }));

        let wireframe_toggle = div()
            .id("surface-wireframe-toggle")
            .px_3()
            .py_1()
            .rounded(px(4.0))
            .cursor_pointer()
            .when(self.surface_wireframe, |el| {
                el.bg(rgb(0x3b82f6)).text_color(rgb(0xffffff))
            })
            .when(!self.surface_wireframe, |el| {
                el.bg(rgb(0xe5e7eb)).text_color(rgb(0x666666))
            })
            .child("Wireframe")
            .on_click(cx.listener(|this, _, _window, cx| {
                this.surface_wireframe = !this.surface_wireframe;
                cx.notify();
            }));

        let isolines_toggle = div()
            .id("surface-isolines-toggle")
            .px_3()
            .py_1()
            .rounded(px(4.0))
            .cursor_pointer()
            .text_sm()
            .bg(if self.surface_isolines {
                rgb(0x3b82f6)
            } else {
                rgb(0xe5e7eb)
            })
            .text_color(if self.surface_isolines {
                rgb(0xffffff)
            } else {
                rgb(0x666666)
            })
            .child("Isolines")
            .on_click(cx.listener(|this, _, _window, cx| {
                this.surface_isolines = !this.surface_isolines;
                cx.notify();
            }));

        // Opacity slider
        let entity = cx.entity().clone();
        let opacity_slider = Slider::new("opacity-slider")
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

        // Frequency slider - uses log scale to match audio perception
        // Slider value is log10(freq), mapped to nearest data index
        let freq_min = freq_values.first().copied().unwrap_or(20.0).max(1.0);
        let freq_max = freq_values.last().copied().unwrap_or(20000.0);
        let log_min = freq_min.log10() as f32;
        let log_max = freq_max.log10() as f32;
        let current_log_freq = current_freq.log10() as f32;

        let entity_freq = cx.entity().clone();
        let freq_values_clone = freq_values.clone();
        let freq_slider = Slider::new("freq-slider")
            .value(current_log_freq)
            .min(log_min)
            .max(log_max)
            .step(0.01) // Fine-grained log steps
            .width(400.0)
            .label("Frequency")
            .show_value(false)
            .on_change(move |log_value, _window, cx| {
                // Convert log value back to frequency
                let target_freq = 10.0_f64.powf(log_value as f64);
                // Find nearest index in freq_values
                let nearest_idx = freq_values_clone
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        let da = (*a - target_freq).abs();
                        let db = (*b - target_freq).abs();
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0);

                entity_freq.update(cx, |this, cx| {
                    this.sphere_freq_idx = nearest_idx;
                    cx.notify();
                });
            });

        // Frequency display
        let freq_display_text = if current_freq >= 1000.0 {
            format!("{:.1} kHz", current_freq / 1000.0)
        } else {
            format!("{:.0} Hz", current_freq)
        };

        // Frequency Overlay on the 3D view
        let freq_overlay = div()
            .absolute()
            .top(px(20.0))
            .right(px(20.0))
            .bg(gpui::hsla(0.0, 0.0, 0.0, 0.7))
            .text_color(rgb(0xffffff))
            .px_4()
            .py_2()
            .rounded_md()
            .text_lg()
            .font_weight(FontWeight::BOLD)
            .child(freq_display_text.clone());

        // Data range info
        let range_info = format!(
            "H: [{:.0}°, {:.0}°]  V: [{:.0}°, {:.0}°]  {} freq points",
            h_min, h_max, v_min, v_max, freq_count
        );

        // Interactive View
        let surface_view = div()
            .id("sphere-3d-view")
            .w(px(800.0))
            .h(px(800.0))
            .bg(rgb(0x1a1a1a))
            .relative()
            .child(surface_element)
            .child(freq_overlay)
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
            .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
                let delta_y = match event.delta {
                    ScrollDelta::Lines(lines) => lines.y,
                    ScrollDelta::Pixels(pixels) => {
                        let py: f32 = pixels.y.into();
                        if py.abs() > 0.0 { py.signum() } else { 0.0 }
                    }
                };

                if delta_y != 0.0 {
                    let max_idx = freq_count.saturating_sub(1);
                    if delta_y > 0.0 && this.sphere_freq_idx < max_idx {
                        this.sphere_freq_idx += 1;
                    } else if delta_y < 0.0 && this.sphere_freq_idx > 0 {
                        this.sphere_freq_idx -= 1;
                    }
                    cx.notify();
                }
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
                        "Sphere Plot - {}",
                        self.selected_speaker.as_deref().unwrap_or("Unknown")
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap_4()
                    .items_center()
                    .child(div().text_sm().text_color(rgb(0x666666)).child("Colormap:"))
                    .child(colormap_selector)
                    .child(wireframe_toggle)
                    .child(isolines_toggle)
                    .child(opacity_slider),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("Frequency:"),
                    )
                    .child(freq_slider)
                    .child(
                        div()
                            .min_w(px(80.0))
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0x333333))
                            .child(freq_display_text),
                    ),
            )
            .child(div().text_xs().text_color(rgb(0x888888)).child(range_info))
            .child(div().flex().justify_center().child(surface_view))
    }
}
