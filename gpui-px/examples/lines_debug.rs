//! Interactive Line Chart Debug - Demonstrates legend position and graph ratio controls.
//!
//! Use the buttons to select legend position (or Auto for ratio-based positioning)
//! and the slider to adjust graph ratio.

use gpui::*;
use gpui_px::*;
use gpui_ui_kit::{MiniApp, MiniAppConfig};

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Line Chart - Legend & Ratio Debug").size(1000.0, 700.0),
        |cx| cx.new(LinesDebugApp::new),
    );
}

struct LinesDebugApp {
    // Data
    x1: Vec<f64>,
    y1: Vec<f64>,
    y2: Vec<f64>,
    y3: Vec<f64>,
    // Settings
    legend_position: Option<LegendPosition>, // None = auto (ratio-based)
    graph_ratio: f32,
}

impl LinesDebugApp {
    fn new(_cx: &mut Context<Self>) -> Self {
        // Generate sample data
        let x1: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
        let y1: Vec<f64> = x1.iter().map(|&x| (x * 2.0).sin() * 30.0 + 50.0).collect();
        let y2: Vec<f64> = x1.iter().map(|&x| (x * 2.0).cos() * 25.0 + 50.0).collect();
        let y3: Vec<f64> = x1
            .iter()
            .map(|&x| (x * 1.5).sin() * 20.0 + (x * 3.0).cos() * 10.0 + 50.0)
            .collect();

        Self {
            x1,
            y1,
            y2,
            y3,
            legend_position: None, // Start with auto
            graph_ratio: 1.414,
        }
    }

    fn render_controls(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().clone();
        let current_position = self.legend_position;
        let current_ratio = self.graph_ratio;

        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .bg(rgb(0xf5f5f5))
            .border_1()
            .border_color(rgb(0xe0e0e0))
            .rounded_lg()
            // Legend Position Selector
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Legend Position"),
                    )
                    // Row 1: Left, Right
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .children(
                                [
                                    ("Left", Some(LegendPosition::Left)),
                                    ("Right", Some(LegendPosition::Right)),
                                ]
                                .into_iter()
                                .map(|(label, position)| {
                                    let entity = entity.clone();
                                    let is_selected = current_position == position;

                                    div()
                                        .id(ElementId::Name(format!("pos-{}", label).into()))
                                        .px_4()
                                        .py_2()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .flex_1()
                                        .text_center()
                                        .bg(if is_selected {
                                            rgb(0x3b82f6)
                                        } else {
                                            rgb(0xe5e7eb)
                                        })
                                        .hover(|s| {
                                            s.bg(if is_selected {
                                                rgb(0x2563eb)
                                            } else {
                                                rgb(0xd1d5db)
                                            })
                                        })
                                        .text_color(if is_selected {
                                            rgb(0xffffff)
                                        } else {
                                            rgb(0x374151)
                                        })
                                        .text_sm()
                                        .child(label)
                                        .on_click(move |_, _window, cx| {
                                            entity.update(cx, |this, _| {
                                                this.legend_position = position;
                                            });
                                        })
                                }),
                            ),
                    )
                    // Row 2: Top, Bottom
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .children(
                                [
                                    ("Top", Some(LegendPosition::Top)),
                                    ("Bottom", Some(LegendPosition::Bottom)),
                                ]
                                .into_iter()
                                .map(|(label, position)| {
                                    let entity = entity.clone();
                                    let is_selected = current_position == position;

                                    div()
                                        .id(ElementId::Name(format!("pos-{}", label).into()))
                                        .px_4()
                                        .py_2()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .flex_1()
                                        .text_center()
                                        .bg(if is_selected {
                                            rgb(0x3b82f6)
                                        } else {
                                            rgb(0xe5e7eb)
                                        })
                                        .hover(|s| {
                                            s.bg(if is_selected {
                                                rgb(0x2563eb)
                                            } else {
                                                rgb(0xd1d5db)
                                            })
                                        })
                                        .text_color(if is_selected {
                                            rgb(0xffffff)
                                        } else {
                                            rgb(0x374151)
                                        })
                                        .text_sm()
                                        .child(label)
                                        .on_click(move |_, _window, cx| {
                                            entity.update(cx, |this, _| {
                                                this.legend_position = position;
                                            });
                                        })
                                }),
                            ),
                    )
                    // Row 3: Auto (ratio-based)
                    .child({
                        let entity = entity.clone();
                        let is_selected = current_position.is_none();

                        div()
                            .id(ElementId::Name("pos-auto".into()))
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .cursor_pointer()
                            .text_center()
                            .bg(if is_selected {
                                rgb(0x10b981) // Green for auto
                            } else {
                                rgb(0xe5e7eb)
                            })
                            .hover(|s| {
                                s.bg(if is_selected {
                                    rgb(0x059669)
                                } else {
                                    rgb(0xd1d5db)
                                })
                            })
                            .text_color(if is_selected {
                                rgb(0xffffff)
                            } else {
                                rgb(0x374151)
                            })
                            .text_sm()
                            .child("Auto (ratio-based)")
                            .on_click(move |_, _window, cx| {
                                entity.update(cx, |this, _| {
                                    this.legend_position = None;
                                });
                            })
                    }),
            )
            // Graph Ratio Slider
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Graph Ratio (height/width)"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(0x3b82f6))
                                    .child(format!("{:.2}", current_ratio)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child(div().text_xs().text_color(rgb(0x6b7280)).child("0.5"))
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .children((0..10).map(|i| {
                                        let entity = entity.clone();
                                        let ratio = 0.5 + (i as f32) * 0.15;
                                        let is_active = (current_ratio - ratio).abs() < 0.08;

                                        div()
                                            .id(ElementId::Name(format!("ratio-{}", i).into()))
                                            .w(px(24.0))
                                            .h(px(24.0))
                                            .rounded_md()
                                            .cursor_pointer()
                                            .bg(if is_active {
                                                rgb(0x3b82f6)
                                            } else {
                                                rgb(0xd1d5db)
                                            })
                                            .hover(|s| s.bg(rgb(0x93c5fd)))
                                            .on_click(move |_, _window, cx| {
                                                entity.update(cx, |this, _| {
                                                    this.graph_ratio = ratio;
                                                });
                                            })
                                    })),
                            )
                            .child(div().text_xs().text_color(rgb(0x6b7280)).child("2.0")),
                    ),
            )
            // Preset Ratios
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Preset Ratios"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .flex_wrap()
                            .children(
                                [
                                    ("Square (1.0)", 1.0),
                                    ("A4 (1.414)", 1.414),
                                    ("Golden (1.618)", 1.618),
                                    ("Wide (0.75)", 0.75),
                                ]
                                .into_iter()
                                .map(|(label, ratio)| {
                                    let entity = entity.clone();
                                    let is_selected = (current_ratio - ratio).abs() < 0.01;

                                    div()
                                        .id(ElementId::Name(format!("preset-{}", label).into()))
                                        .px_2()
                                        .py_1()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .bg(if is_selected {
                                            rgb(0x10b981)
                                        } else {
                                            rgb(0xe5e7eb)
                                        })
                                        .hover(|s| {
                                            s.bg(if is_selected {
                                                rgb(0x059669)
                                            } else {
                                                rgb(0xd1d5db)
                                            })
                                        })
                                        .text_color(if is_selected {
                                            rgb(0xffffff)
                                        } else {
                                            rgb(0x374151)
                                        })
                                        .text_xs()
                                        .child(label)
                                        .on_click(move |_, _window, cx| {
                                            entity.update(cx, |this, _| {
                                                this.graph_ratio = ratio;
                                            });
                                        })
                                }),
                            ),
                    ),
            )
            // Info
            .child(
                div()
                    .mt_2()
                    .p_3()
                    .bg(rgb(0xdbeafe))
                    .rounded_md()
                    .text_xs()
                    .text_color(rgb(0x1e40af))
                    .child(
                        "Select 'Auto' to let the chart automatically choose legend position \
                         based on the graph ratio. Try different ratios to see the auto-selection change.",
                    ),
            )
    }

    fn render_chart(&self) -> impl IntoElement {
        // Calculate chart dimensions based on ratio
        // Keep width fixed, adjust height based on ratio
        let chart_width = 600.0;
        let chart_height = chart_width * self.graph_ratio;

        let mut chart = line(&self.x1, &self.y1)
            .label("Sine Wave")
            .color(0x3b82f6)
            .stroke_width(2.0)
            .add_series(&self.y2, Some("Cosine Wave"), 0xff7f0e, 2.0, 1.0)
            .add_series(&self.y3, Some("Combined"), 0x2ca02c, 2.0, 1.0)
            .title("Multi-Series Line Chart")
            .graph_ratio(self.graph_ratio)
            .size(chart_width, chart_height);

        // Only set legend position if explicitly chosen
        if let Some(pos) = self.legend_position {
            chart = chart.legend_position(pos);
        }

        chart.build().unwrap()
    }
}

impl Render for LinesDebugApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xffffff))
            .p_6()
            .gap_6()
            // Header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child("Line Chart - Legend & Graph Ratio Debug"),
                    )
                    .child(div().text_sm().text_color(rgb(0x6b7280)).child(
                        "Adjust legend position and graph ratio to see how they affect layout",
                    )),
            )
            // Content
            .child(
                div()
                    .flex()
                    .gap_6()
                    .flex_1()
                    // Chart
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_1()
                            .bg(rgb(0xfafafa))
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(0xe5e7eb))
                            .child(self.render_chart()),
                    )
                    // Controls
                    .child(div().w(px(280.0)).child(self.render_controls(cx))),
            )
    }
}
