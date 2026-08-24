use super::super::faithful_data::{FAITHFUL_WAITING, faithful_stats};
use super::kernel_type::KernelType;
use super::misc::epanechnikov_kernel;
use super::misc::gaussian_kernel;
use super::misc::histogram;
use super::misc::kde;
use super::misc::uniform_kernel;
use crate::ShowcaseApp;
use d3rs::color::D3Color;
use d3rs::scale::{LinearScale, Scale};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::Slider;
use gpui_ui_kit::theme::ThemeExt;
use std::sync::Arc;

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let entity = cx.entity().clone();

    // Get parameters from app state
    let bandwidth = app.kde_bandwidth;
    let kernel_type = app.kde_kernel_type;
    let show_histogram = app.kde_show_histogram;
    let bin_count = app.kde_bin_count;

    // Get data statistics
    let stats = faithful_stats();
    let data = FAITHFUL_WAITING;

    // Extend range slightly for nice visualization
    let x_min = stats.min - 5.0;
    let x_max = stats.max + 5.0;

    // Generate thresholds for KDE evaluation
    let num_points = 100;
    let thresholds: Vec<f64> = (0..num_points)
        .map(|i| x_min + (x_max - x_min) * i as f64 / (num_points - 1) as f64)
        .collect();

    // Compute KDE based on kernel type
    let density_points: Vec<(f64, f64)> = match kernel_type {
        KernelType::Epanechnikov => kde(epanechnikov_kernel(bandwidth), &thresholds, data),
        KernelType::Gaussian => kde(gaussian_kernel(bandwidth), &thresholds, data),
        KernelType::Uniform => kde(uniform_kernel(bandwidth), &thresholds, data),
    };

    // Find max density for Y scale
    let max_density = density_points
        .iter()
        .map(|(_, d)| *d)
        .fold(0.0_f64, f64::max);

    // Compute histogram
    let hist_bins = histogram(data, bin_count, x_min, x_max);
    let max_proportion = hist_bins
        .iter()
        .map(|(_, _, count)| *count as f64 / data.len() as f64)
        .fold(0.0_f64, f64::max);

    // Use the larger of density or histogram for Y scale
    let y_max = if show_histogram {
        max_density.max(max_proportion) * 1.1
    } else {
        max_density * 1.1
    };

    // Plot dimensions
    let plot_width = 600.0_f32;
    let plot_height = 350.0_f32;

    // Scales
    let x_scale = LinearScale::new()
        .domain(x_min, x_max)
        .range(0.0, plot_width as f64);
    let y_scale = LinearScale::new()
        .domain(0.0, y_max)
        .range(plot_height as f64, 0.0); // Inverted for screen coordinates

    div()
        .flex()
        .flex_col()
        .gap_6()
        // Title
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::BOLD)
                        .child("Kernel Density Estimation"),
                )
                .child(
                    div()
                        .text_sm()
                        .child("Ported from Observable: d3/kernel-density-estimation"),
                ),
        )
        // Main content
        .child(
            div()
                .flex()
                .gap_8()
                // Left: Visualization
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        // High-level API visualization
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_3()
                                        .child(
                                            div()
                                                .text_lg()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child("Old Faithful Waiting Times"),
                                        )
                                        .child(
                                            div()
                                                .px_2()
                                                .py_1()
                                                .bg(ui_theme.accent)
                                                .rounded_md()
                                                .text_xs()
                                                .child(format!("Bandwidth: {:.1}", bandwidth)),
                                        ),
                                )
                                .child(div().text_xs().child("Time between eruptions (minutes)"))
                                .child(
                                    div()
                                        .w(px(plot_width))
                                        .h(px(plot_height))
                                        .bg(ui_theme.surface)
                                        .border_1()
                                        .border_color(ui_theme.border)
                                        .rounded_md()
                                        .relative()
                                        .overflow_hidden()
                                        // Histogram bars (if enabled)
                                        .when(show_histogram, |this| {
                                            this.children(hist_bins.iter().map(
                                                |(bin_start, bin_end, count)| {
                                                    let x1 = x_scale.scale(*bin_start) as f32;
                                                    let x2 = x_scale.scale(*bin_end) as f32;
                                                    let proportion =
                                                        *count as f64 / data.len() as f64;
                                                    let y = y_scale.scale(proportion) as f32;
                                                    let bar_height = plot_height - y;

                                                    div()
                                                        .absolute()
                                                        .left(px(x1))
                                                        .top(px(y))
                                                        .w(px((x2 - x1).max(1.0)))
                                                        .h(px(bar_height))
                                                        .bg(Hsla::from(ui_theme.muted).opacity(0.6))
                                                        .border_1()
                                                        .border_color(
                                                            Hsla::from(ui_theme.border)
                                                                .opacity(0.3),
                                                        )
                                                },
                                            ))
                                        })
                                        // KDE curve using custom element
                                        .child(KdeCurveElement {
                                            points: Arc::new(density_points.clone()),
                                            x_scale,
                                            y_scale,
                                            color: D3Color::from_rgba(ui_theme.accent),
                                            stroke_width: 2.5,
                                            plot_width,
                                            plot_height,
                                        })
                                        // Zero line
                                        .child(
                                            div()
                                                .absolute()
                                                .left(px(0.0))
                                                .bottom(px(0.0))
                                                .w_full()
                                                .h(px(1.0))
                                                .bg(ui_theme.border),
                                        ),
                                )
                                // X-axis labels
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .w(px(plot_width))
                                        .mt_1()
                                        .text_xs()
                                        .child(format!("{:.0}", x_min))
                                        .child(format!("{:.0}", (x_min + x_max) / 2.0))
                                        .child(format!("{:.0}", x_max)),
                                ),
                        ),
                )
                // Right: Controls
                .child(
                    div()
                        .w(px(300.0))
                        .flex()
                        .flex_col()
                        .gap_4()
                        // Controls panel
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .p_4()
                                .bg(ui_theme.surface)
                                .border_1()
                                .border_color(ui_theme.border)
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Controls"),
                                )
                                // Bandwidth slider
                                .child({
                                    let entity = entity.clone();
                                    Slider::new("kde-bandwidth")
                                        .label("Bandwidth")
                                        .value(bandwidth as f32)
                                        .min(1.0)
                                        .max(20.0)
                                        .step(0.5)
                                        .show_value(true)
                                        .width(230.0)
                                        .on_change(move |value, _window, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.kde_bandwidth = value as f64;
                                                cx.notify();
                                            });
                                        })
                                })
                                // Kernel type toggle
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(div().text_sm().child("Kernel"))
                                        .child({
                                            let entity = entity.clone();
                                            div()
                                                .id("kernel-toggle")
                                                .px_3()
                                                .py_1()
                                                .bg(ui_theme.accent)
                                                .hover(|s| s.bg(ui_theme.accent_hover))
                                                .rounded_md()
                                                .cursor_pointer()
                                                .text_sm()
                                                .child(kernel_type.label())
                                                .on_click(move |_, _window, cx| {
                                                    entity.update(cx, |this, cx| {
                                                        this.kde_kernel_type =
                                                            this.kde_kernel_type.next();
                                                        cx.notify();
                                                    });
                                                })
                                        }),
                                )
                                // Show histogram toggle
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(div().text_sm().child("Show Histogram"))
                                        .child({
                                            let entity = entity.clone();
                                            let bg = if show_histogram {
                                                ui_theme.success
                                            } else {
                                                ui_theme.muted
                                            };
                                            div()
                                                .id("histogram-toggle")
                                                .px_3()
                                                .py_1()
                                                .bg(bg)
                                                .hover(|s| s.opacity(0.8))
                                                .rounded_md()
                                                .cursor_pointer()
                                                .text_sm()
                                                .child(if show_histogram { "On" } else { "Off" })
                                                .on_click(move |_, _window, cx| {
                                                    entity.update(cx, |this, cx| {
                                                        this.kde_show_histogram =
                                                            !this.kde_show_histogram;
                                                        cx.notify();
                                                    });
                                                })
                                        }),
                                )
                                // Bin count slider
                                .child({
                                    let entity = entity.clone();
                                    Slider::new("kde-bins")
                                        .label("Histogram Bins")
                                        .value(bin_count as f32)
                                        .min(5.0)
                                        .max(40.0)
                                        .step(1.0)
                                        .show_value(true)
                                        .width(230.0)
                                        .on_change(move |value, _window, cx| {
                                            entity.update(cx, |this, cx| {
                                                this.kde_bin_count = value as usize;
                                                cx.notify();
                                            });
                                        })
                                }),
                        )
                        // Statistics panel
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .p_4()
                                .bg(ui_theme.surface)
                                .border_1()
                                .border_color(ui_theme.border)
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child("DATA STATISTICS"),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .child(format!("Observations: {}", stats.count)),
                                )
                                .child(div().text_sm().child(format!(
                                    "Range: {:.0} - {:.0} min",
                                    stats.min, stats.max
                                )))
                                .child(
                                    div()
                                        .text_sm()
                                        .child(format!("Mean: {:.1} min", stats.mean)),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .child(format!("Std Dev: {:.1} min", stats.std_dev)),
                                ),
                        )
                        // Code sample
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .p_4()
                                .bg(ui_theme.muted)
                                .border_1()
                                .border_color(ui_theme.border)
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child("KDE ALGORITHM"),
                                )
                                .child(div().text_xs().font_family("monospace").child(
                                    r#"// Epanechnikov kernel
fn kernel(bandwidth: f64) -> impl Fn(f64) -> f64 {
  move |x| {
    let u = x / bandwidth;
    if u.abs() <= 1.0 {
      0.75 * (1.0 - u*u) / bandwidth
    } else { 0.0 }
  }
}

// KDE computation
fn kde(kernel, thresholds, data) {
  thresholds.map(|t| {
    let d = data.iter()
      .map(|d| kernel(t - d))
      .sum() / data.len();
    (t, d)
  })
}"#,
                                )),
                        ),
                ),
        )
}

/// Custom element for drawing the KDE curve
struct KdeCurveElement {
    pub(super) points: Arc<Vec<(f64, f64)>>,
    pub(super) x_scale: LinearScale,
    pub(super) y_scale: LinearScale,
    pub(super) color: D3Color,
    pub(super) stroke_width: f32,
    pub(super) plot_width: f32,
    pub(super) plot_height: f32,
}

impl IntoElement for KdeCurveElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for KdeCurveElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = window.request_layout(
            Style {
                position: Position::Absolute,
                size: size(px(self.plot_width).into(), px(self.plot_height).into()),
                ..Default::default()
            },
            [],
            cx,
        );
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        if self.points.len() < 2 {
            return;
        }

        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();

        // Convert points to screen coordinates
        let screen_points: Vec<Point<Pixels>> = self
            .points
            .iter()
            .map(|(x, y)| {
                let sx = origin_x + self.x_scale.scale(*x) as f32;
                let sy = origin_y + self.y_scale.scale(*y) as f32;
                point(px(sx), px(sy))
            })
            .collect();

        // Draw the curve using line segments
        // With 100 points, line segments appear smooth
        let mut builder = PathBuilder::stroke(px(self.stroke_width));

        builder.move_to(screen_points[0]);
        for pt in &screen_points[1..] {
            builder.line_to(*pt);
        }

        if let Ok(path) = builder.build() {
            window.paint_path(path, self.color.to_rgba());
        }
    }
}
