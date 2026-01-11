//! Kernel Density Estimation - D3.js Example Port
//!
//! This example demonstrates kernel density estimation with histogram overlay,
//! ported from: https://observablehq.com/@d3/kernel-density-estimation
//!
//! The example shows both:
//! 1. **High-level API**: Using d3rs shape primitives
//! 2. **Low-level API**: Direct computation of KDE with custom kernels

use super::faithful_data::{FAITHFUL_WAITING, faithful_stats};
use crate::ShowcaseApp;
use d3rs::color::D3Color;
use d3rs::prelude::*;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_ui_kit::Slider;
use std::sync::Arc;

/// Kernel type for density estimation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KernelType {
    #[default]
    Epanechnikov,
    Gaussian,
    Uniform,
}

impl KernelType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Epanechnikov => "Epanechnikov",
            Self::Gaussian => "Gaussian",
            Self::Uniform => "Uniform",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Epanechnikov => Self::Gaussian,
            Self::Gaussian => Self::Uniform,
            Self::Uniform => Self::Epanechnikov,
        }
    }
}

/// Epanechnikov (parabolic) kernel
/// K(x) = 0.75 * (1 - x^2) for |x| <= 1, 0 otherwise
fn epanechnikov_kernel(bandwidth: f64) -> impl Fn(f64) -> f64 {
    move |x: f64| {
        let u = x / bandwidth;
        if u.abs() <= 1.0 {
            0.75 * (1.0 - u * u) / bandwidth
        } else {
            0.0
        }
    }
}

/// Gaussian kernel
/// K(x) = (1/sqrt(2*pi)) * exp(-0.5 * x^2)
fn gaussian_kernel(bandwidth: f64) -> impl Fn(f64) -> f64 {
    let sqrt_2pi = (2.0 * std::f64::consts::PI).sqrt();
    move |x: f64| {
        let u = x / bandwidth;
        ((-0.5 * u * u).exp()) / (sqrt_2pi * bandwidth)
    }
}

/// Uniform (box) kernel
/// K(x) = 0.5 for |x| <= 1, 0 otherwise
fn uniform_kernel(bandwidth: f64) -> impl Fn(f64) -> f64 {
    move |x: f64| {
        let u = x / bandwidth;
        if u.abs() <= 1.0 { 0.5 / bandwidth } else { 0.0 }
    }
}

/// Compute kernel density estimation
fn kde<F>(kernel: F, thresholds: &[f64], data: &[f64]) -> Vec<(f64, f64)>
where
    F: Fn(f64) -> f64,
{
    thresholds
        .iter()
        .map(|&t| {
            let density = data.iter().map(|&d| kernel(t - d)).sum::<f64>() / data.len() as f64;
            (t, density)
        })
        .collect()
}

/// Compute histogram bins
fn histogram(data: &[f64], bin_count: usize, min: f64, max: f64) -> Vec<(f64, f64, usize)> {
    let bin_width = (max - min) / bin_count as f64;
    let mut bins = vec![0usize; bin_count];

    for &value in data {
        let bin_idx = ((value - min) / bin_width).floor() as usize;
        let bin_idx = bin_idx.min(bin_count - 1);
        bins[bin_idx] += 1;
    }

    bins.iter()
        .enumerate()
        .map(|(i, &count)| {
            let bin_start = min + i as f64 * bin_width;
            let bin_end = bin_start + bin_width;
            (bin_start, bin_end, count)
        })
        .collect()
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
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
                        .text_color(rgb(0x666666))
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
                                                .bg(rgb(0x007acc))
                                                .rounded_md()
                                                .text_xs()
                                                .text_color(rgb(0xffffff))
                                                .child(format!("Bandwidth: {:.1}", bandwidth)),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0x888888))
                                        .child("Time between eruptions (minutes)"),
                                )
                                .child(
                                    div()
                                        .w(px(plot_width))
                                        .h(px(plot_height))
                                        .bg(rgb(0xfafafa))
                                        .border_1()
                                        .border_color(rgb(0xe0e0e0))
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
                                                        .bg(rgba(0xbbbbbbaa))
                                                        .border_1()
                                                        .border_color(rgba(0x99999933))
                                                },
                                            ))
                                        })
                                        // KDE curve using custom element
                                        .child(KdeCurveElement {
                                            points: Arc::new(density_points.clone()),
                                            x_scale: x_scale.clone(),
                                            y_scale: y_scale.clone(),
                                            color: D3Color::from_hex(0x4169e1),
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
                                                .bg(rgb(0x333333)),
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
                                        .text_color(rgb(0x666666))
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
                                .bg(rgb(0xf8f8f8))
                                .border_1()
                                .border_color(rgb(0xe0e0e0))
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(0x333333))
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
                                            entity.update(cx, |this, _| {
                                                this.kde_bandwidth = value as f64;
                                            });
                                        })
                                })
                                // Kernel type toggle
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x555555))
                                                .child("Kernel"),
                                        )
                                        .child({
                                            let entity = entity.clone();
                                            div()
                                                .id("kernel-toggle")
                                                .px_3()
                                                .py_1()
                                                .bg(rgb(0x007acc))
                                                .hover(|s| s.bg(rgb(0x005a9e)))
                                                .rounded_md()
                                                .cursor_pointer()
                                                .text_sm()
                                                .text_color(rgb(0xffffff))
                                                .child(kernel_type.label())
                                                .on_click(move |_, _window, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.kde_kernel_type =
                                                            this.kde_kernel_type.next();
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
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x555555))
                                                .child("Show Histogram"),
                                        )
                                        .child({
                                            let entity = entity.clone();
                                            let bg = if show_histogram {
                                                rgb(0x28a745)
                                            } else {
                                                rgb(0xcccccc)
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
                                                .text_color(rgb(0xffffff))
                                                .child(if show_histogram { "On" } else { "Off" })
                                                .on_click(move |_, _window, cx| {
                                                    entity.update(cx, |this, _| {
                                                        this.kde_show_histogram =
                                                            !this.kde_show_histogram;
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
                                            entity.update(cx, |this, _| {
                                                this.kde_bin_count = value as usize;
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
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(0xe0e0e0))
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0x888888))
                                        .child("DATA STATISTICS"),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(0x333333))
                                        .child(format!("Observations: {}", stats.count)),
                                )
                                .child(div().text_sm().text_color(rgb(0x333333)).child(format!(
                                    "Range: {:.0} - {:.0} min",
                                    stats.min, stats.max
                                )))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(0x333333))
                                        .child(format!("Mean: {:.1} min", stats.mean)),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(0x333333))
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
                                .bg(rgb(0x1e1e1e))
                                .border_1()
                                .border_color(rgb(0x333333))
                                .rounded_lg()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(0x888888))
                                        .child("KDE ALGORITHM"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .font_family("monospace")
                                        .text_color(rgb(0xd4d4d4))
                                        .child(
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
                                        ),
                                ),
                        ),
                ),
        )
}

/// Custom element for drawing the KDE curve
struct KdeCurveElement {
    points: Arc<Vec<(f64, f64)>>,
    x_scale: LinearScale,
    y_scale: LinearScale,
    color: D3Color,
    stroke_width: f32,
    plot_width: f32,
    plot_height: f32,
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
