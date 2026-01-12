//! Stacked to Grouped Bars - D3.js Example Port
//!
//! This example demonstrates animated transitions between stacked and grouped bar charts,
//! ported from: https://observablehq.com/@d3/stacked-to-grouped-bars
//!
//! Features:
//! - Smooth animated transitions between layouts
//! - Multiple data series with different colors
//! - Staggered animations for visual appeal

use crate::ShowcaseApp;
use d3rs::color::D3Color;
use d3rs::ease::ease_cubic_in_out;
use gpui::prelude::FluentBuilder;
use gpui::*;
use std::time::Duration;

/// Layout mode for bar chart
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarLayout {
    #[default]
    Stacked,
    Grouped,
}

impl BarLayout {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Stacked => "Stacked",
            Self::Grouped => "Grouped",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Self::Stacked => Self::Grouped,
            Self::Grouped => Self::Stacked,
        }
    }
}

/// Sample data for the bar chart
/// Returns (n_series, m_samples, data) where data[series][sample] = value
pub fn generate_sample_data(n_series: usize, m_samples: usize) -> Vec<Vec<f64>> {
    let mut data = Vec::with_capacity(n_series);

    for i in 0..n_series {
        let mut series = Vec::with_capacity(m_samples);
        for j in 0..m_samples {
            // Generate bumpy data similar to D3's bumps function
            let base = 0.5 + 0.5 * ((j as f64 * 0.15 + i as f64 * 0.8).sin());
            let bump1 = 0.3 * (-(((j as f64 - 10.0 - i as f64 * 5.0) / 8.0).powi(2))).exp();
            let bump2 = 0.2 * (-(((j as f64 - 25.0 + i as f64 * 3.0) / 6.0).powi(2))).exp();
            let bump3 = 0.25 * (-(((j as f64 - 40.0 - i as f64 * 2.0) / 10.0).powi(2))).exp();

            series.push((base + bump1 + bump2 + bump3).max(0.1));
        }
        data.push(series);
    }

    data
}

/// Color for a series using a blue sequential scale
fn series_color(index: usize, total: usize) -> D3Color {
    let t = (index as f64 + 1.0) / (total as f64 + 1.0);
    // Interpolate blues from light to dark
    let r = (255.0 * (1.0 - t * 0.7)) as u8;
    let g = (255.0 * (1.0 - t * 0.5)) as u8;
    let b = 255;
    D3Color::rgb(r, g, b)
}

/// Compute bar positions for stacked layout
fn compute_stacked_layout(
    data: &[Vec<f64>],
    plot_width: f64,
    plot_height: f64,
    padding: f64,
) -> Vec<Vec<(f64, f64, f64, f64)>> {
    let n_series = data.len();
    if n_series == 0 {
        return Vec::new();
    }

    let m_samples = data[0].len();
    let bar_width = (plot_width - padding * (m_samples as f64 - 1.0)) / m_samples as f64;

    // Compute stacked totals for y-scaling
    let mut max_stack = 0.0_f64;
    for j in 0..m_samples {
        let stack_total: f64 = data.iter().map(|s| s[j]).sum();
        max_stack = max_stack.max(stack_total);
    }

    let y_scale = if max_stack > 0.0 {
        plot_height / max_stack
    } else {
        1.0
    };

    let mut result = Vec::with_capacity(n_series);

    for i in 0..n_series {
        let mut series_rects = Vec::with_capacity(m_samples);

        for j in 0..m_samples {
            let x = j as f64 * (bar_width + padding);

            // Compute y0 (bottom of this segment)
            let y0: f64 = data[..i].iter().map(|s| s[j]).sum();
            let y1 = y0 + data[i][j];

            let screen_y0 = plot_height - y0 * y_scale;
            let screen_y1 = plot_height - y1 * y_scale;
            let height = screen_y0 - screen_y1;

            series_rects.push((x, screen_y1, bar_width, height));
        }

        result.push(series_rects);
    }

    result
}

/// Compute bar positions for grouped layout
fn compute_grouped_layout(
    data: &[Vec<f64>],
    plot_width: f64,
    plot_height: f64,
    padding: f64,
) -> Vec<Vec<(f64, f64, f64, f64)>> {
    let n_series = data.len();
    if n_series == 0 {
        return Vec::new();
    }

    let m_samples = data[0].len();
    let group_width = (plot_width - padding * (m_samples as f64 - 1.0)) / m_samples as f64;
    let bar_width = group_width / n_series as f64;

    // Find max value for y-scaling
    let max_value = data
        .iter()
        .flat_map(|s| s.iter())
        .cloned()
        .fold(0.0_f64, f64::max);

    let y_scale = if max_value > 0.0 {
        plot_height / max_value
    } else {
        1.0
    };

    let mut result = Vec::with_capacity(n_series);

    for (i, row) in data.iter().enumerate().take(n_series) {
        let mut series_rects = Vec::with_capacity(m_samples);

        for (j, &value) in row.iter().enumerate().take(m_samples) {
            let group_x = j as f64 * (group_width + padding);
            let x = group_x + i as f64 * bar_width;

            let height = value * y_scale;
            let y = plot_height - height;

            series_rects.push((x, y, bar_width, height));
        }

        result.push(series_rects);
    }

    result
}

/// Interpolate between two layouts
fn interpolate_layouts(
    from: &[Vec<(f64, f64, f64, f64)>],
    to: &[Vec<(f64, f64, f64, f64)>],
    t: f64,
) -> Vec<Vec<(f64, f64, f64, f64)>> {
    let t_eased = ease_cubic_in_out(t);

    from.iter()
        .zip(to.iter())
        .map(|(from_series, to_series)| {
            from_series
                .iter()
                .zip(to_series.iter())
                .map(|(&(fx, fy, fw, fh), &(tx, ty, tw, th))| {
                    (
                        fx + (tx - fx) * t_eased,
                        fy + (ty - fy) * t_eased,
                        fw + (tw - fw) * t_eased,
                        fh + (th - fh) * t_eased,
                    )
                })
                .collect()
        })
        .collect()
}

fn start_animation_loop(entity: Entity<ShowcaseApp>, cx: &mut Context<ShowcaseApp>) {
    let animation_entity = entity.clone();
    cx.spawn(async move |_this: WeakEntity<ShowcaseApp>, cx| {
        loop {
            Timer::after(Duration::from_millis(16)).await;
            let should_continue = cx.update(|cx| {
                animation_entity.update(cx, |this, cx| {
                    if !this.stacked_bars_animating {
                        return false;
                    }
                    this.stacked_bars_animation_progress += 0.04; // ~25 frames for full animation
                    if this.stacked_bars_animation_progress >= 1.0 {
                        this.stacked_bars_animation_progress = 1.0;
                        this.stacked_bars_animating = false;
                        cx.notify();
                        return false;
                    }
                    cx.notify();
                    true
                })
            });

            if !should_continue {
                break;
            }
        }
    })
    .detach();
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let entity = cx.entity().clone();

    // Get parameters from app state
    let layout = app.stacked_bars_layout;
    let n_series = app.stacked_bars_n_series;
    let m_samples = app.stacked_bars_m_samples;
    let animation_progress = app.stacked_bars_animation_progress;
    let animating = app.stacked_bars_animating;

    // Generate data
    let data = generate_sample_data(n_series, m_samples);

    // Plot dimensions
    let plot_width = 700.0_f64;
    let plot_height = 350.0_f64;
    let bar_padding = 2.0;

    // Compute both layouts
    let stacked_layout = compute_stacked_layout(&data, plot_width, plot_height, bar_padding);
    let grouped_layout = compute_grouped_layout(&data, plot_width, plot_height, bar_padding);

    // Determine current and target layouts based on animation state
    let current_rects = if animating {
        let (from, to) = match layout {
            BarLayout::Stacked => (&grouped_layout, &stacked_layout),
            BarLayout::Grouped => (&stacked_layout, &grouped_layout),
        };
        interpolate_layouts(from, to, animation_progress)
    } else {
        match layout {
            BarLayout::Stacked => stacked_layout.clone(),
            BarLayout::Grouped => grouped_layout.clone(),
        }
    };

    // Animation loop is now handled by start_animation_loop triggered by actions

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
                        .child("Stacked to Grouped Bars"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x666666))
                        .child("Ported from Observable: d3/stacked-to-grouped-bars"),
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
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("Animated Bar Chart"),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .bg(rgb(0x007acc))
                                        .rounded_md()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child(layout.label()),
                                )
                                .when(animating, |this| {
                                    this.child(
                                        div()
                                            .px_2()
                                            .py_1()
                                            .bg(rgb(0x28a745))
                                            .rounded_md()
                                            .text_xs()
                                            .text_color(rgb(0xffffff))
                                            .child(format!(
                                                "{:.0}%",
                                                animation_progress * 100.0
                                            )),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .w(px(plot_width as f32))
                                .h(px(plot_height as f32))
                                .bg(rgb(0xfafafa))
                                .border_1()
                                .border_color(rgb(0xe0e0e0))
                                .rounded_md()
                                .relative()
                                .overflow_hidden()
                                // Render all bars
                                .children(current_rects.iter().enumerate().flat_map(
                                    |(series_idx, series_rects)| {
                                        let color = series_color(series_idx, n_series);
                                        series_rects.iter().map(move |&(x, y, w, h)| {
                                            div()
                                                .absolute()
                                                .left(px(x as f32))
                                                .top(px(y as f32))
                                                .w(px(w as f32))
                                                .h(px(h.max(0.0) as f32))
                                                .bg(color.to_rgba())
                                        })
                                    },
                                )),
                        )
                        // X-axis labels (simplified)
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .w(px(plot_width as f32))
                                .mt_1()
                                .text_xs()
                                .text_color(rgb(0x666666))
                                .child("0")
                                .child(format!("{}", m_samples / 2))
                                .child(format!("{}", m_samples)),
                        ),
                )
                // Right: Controls
                .child(
                    div()
                        .w(px(280.0))
                        .flex()
                        .flex_col()
                        .gap_4()
                        // Toggle button
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
                                        .child("Layout"),
                                )
                                .child({
                                    let entity = entity.clone();
                                    let disabled = animating;
                                    div()
                                        .id("toggle-layout")
                                        .px_4()
                                        .py_2()
                                        .bg(if disabled {
                                            rgb(0xcccccc)
                                        } else {
                                            rgb(0x007acc)
                                        })
                                        .when(!disabled, |this| {
                                            this.hover(|s| s.bg(rgb(0x005a9e)))
                                        })
                                        .rounded_md()
                                        .cursor(if disabled {
                                            CursorStyle::default()
                                        } else {
                                            CursorStyle::PointingHand
                                        })
                                        .text_sm()
                                        .text_color(rgb(0xffffff))
                                        .text_center()
                                        .child(format!(
                                            "Switch to {}",
                                            layout.toggle().label()
                                        ))
                                        .when(!disabled, |this| {
                                            this.on_click(move |_, _window, cx| {
                                                let entity_clone = entity.clone();
                                                entity.update(cx, |this, cx| {
                                                    if !this.stacked_bars_animating {
                                                        this.stacked_bars_layout =
                                                            this.stacked_bars_layout.toggle();
                                                        this.stacked_bars_animating = true;
                                                        this.stacked_bars_animation_progress = 0.0;
                                                        start_animation_loop(entity_clone, cx);
                                                    }
                                                });
                                            })
                                        })
                                })
                                // Layout buttons
                                .child(
                                    div()
                                        .flex()
                                        .gap_2()
                                        .children(
                                            [BarLayout::Stacked, BarLayout::Grouped]
                                                .iter()
                                                .map(|&l| {
                                                    let entity = entity.clone();
                                                    let is_selected = l == layout && !animating;
                                                    let bg = if is_selected {
                                                        rgb(0x007acc)
                                                    } else {
                                                        rgb(0xe0e0e0)
                                                    };
                                                    let text_color = if is_selected {
                                                        rgb(0xffffff)
                                                    } else {
                                                        rgb(0x333333)
                                                    };

                                                    div()
                                                        .id(ElementId::Name(
                                                            format!("layout-{}", l.label()).into(),
                                                        ))
                                                        .flex_1()
                                                        .px_3()
                                                        .py_2()
                                                        .bg(bg)
                                                        .hover(|s| s.opacity(0.8))
                                                        .rounded_md()
                                                        .cursor_pointer()
                                                        .text_xs()
                                                        .text_color(text_color)
                                                        .text_center()
                                                        .child(l.label())
                                                        .on_click(move |_, _window, cx| {
                                                            let entity_clone = entity.clone();
                                                            entity.update(cx, |this, cx| {
                                                                if !this.stacked_bars_animating
                                                                    && this.stacked_bars_layout != l
                                                                {
                                                                    this.stacked_bars_layout = l;
                                                                    this.stacked_bars_animating =
                                                                        true;
                                                                    this
                                                                        .stacked_bars_animation_progress = 0.0;
                                                                    start_animation_loop(entity_clone, cx);
                                                                }
                                                            });
                                                        })
                                                }),
                                        ),
                                ),
                        )
                        // Data parameters
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
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
                                        .child("DATA PARAMETERS"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x333333))
                                                .child("Series"),
                                        )
                                        .child({
                                            let entity = entity.clone();
                                            div()
                                                .flex()
                                                .gap_1()
                                                .children([3_usize, 4, 5, 6].iter().map(|&n| {
                                                    let entity = entity.clone();
                                                    let is_selected = n == n_series;
                                                    let bg = if is_selected {
                                                        rgb(0x007acc)
                                                    } else {
                                                        rgb(0xe0e0e0)
                                                    };
                                                    let text_color = if is_selected {
                                                        rgb(0xffffff)
                                                    } else {
                                                        rgb(0x333333)
                                                    };

                                                    div()
                                                        .id(ElementId::Name(
                                                            format!("series-{}", n).into(),
                                                        ))
                                                        .px_2()
                                                        .py_1()
                                                        .bg(bg)
                                                        .hover(|s| s.opacity(0.8))
                                                        .rounded_sm()
                                                        .cursor_pointer()
                                                        .text_xs()
                                                        .text_color(text_color)
                                                        .child(format!("{}", n))
                                                        .on_click(move |_, _window, cx| {
                                                            entity.update(cx, |this, _| {
                                                                this.stacked_bars_n_series = n;
                                                            });
                                                        })
                                                }))
                                        }),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x333333))
                                                .child("Samples"),
                                        )
                                        .child({
                                            let entity = entity.clone();
                                            div()
                                                .flex()
                                                .gap_1()
                                                .children(
                                                    [20_usize, 30, 40, 50].iter().map(|&m| {
                                                        let entity = entity.clone();
                                                        let is_selected = m == m_samples;
                                                        let bg = if is_selected {
                                                            rgb(0x007acc)
                                                        } else {
                                                            rgb(0xe0e0e0)
                                                        };
                                                        let text_color = if is_selected {
                                                            rgb(0xffffff)
                                                        } else {
                                                            rgb(0x333333)
                                                        };

                                                        div()
                                                            .id(ElementId::Name(
                                                                format!("samples-{}", m).into(),
                                                            ))
                                                            .px_2()
                                                            .py_1()
                                                            .bg(bg)
                                                            .hover(|s| s.opacity(0.8))
                                                            .rounded_sm()
                                                            .cursor_pointer()
                                                            .text_xs()
                                                            .text_color(text_color)
                                                            .child(format!("{}", m))
                                                            .on_click(move |_, _window, cx| {
                                                                entity.update(cx, |this, _| {
                                                                    this.stacked_bars_m_samples = m;
                                                                });
                                                            })
                                                    }),
                                                )
                                        }),
                                ),
                        )
                        // Legend
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
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
                                        .child("SERIES"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .gap_2()
                                        .children((0..n_series).map(|i| {
                                            let color = series_color(i, n_series);
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .w(px(12.0))
                                                        .h(px(12.0))
                                                        .rounded_sm()
                                                        .bg(color.to_rgba()),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(0x333333))
                                                        .child(format!("S{}", i + 1)),
                                                )
                                        })),
                                ),
                        )
                        // Animation info
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
                                        .child("ANIMATION"),
                                )
                                .child(
                                    div().text_xs().text_color(rgb(0xd4d4d4)).child(
                                        "Click 'Switch' or the layout buttons to animate between stacked and grouped views. The transition uses cubic ease-in-out for smooth motion.",
                                    ),
                                ),
                        ),
                ),
        )
}
