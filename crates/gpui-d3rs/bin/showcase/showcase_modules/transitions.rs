use d3rs::ease::{
    ease_back_in_out, ease_bounce_out, ease_circle_in_out, ease_cubic_in_out, ease_elastic_out,
    ease_exp_in_out, ease_linear, ease_quad_in_out, ease_sin_in_out,
};
use gpui::*;

pub fn render(_app: &ShowcaseApp) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Transitions Demo"),
        )
        .child(
            div()
                .text_base()
                .text_color(rgb(0x666666))
                .max_w(px(700.0))
                .child("The d3-transition module provides easing functions for smooth animations. Each curve shows how the easing transforms time (0→1) into progress."),
        )
        // Row 1: Basic easings
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .child(render_easing_card("Linear", ease_linear))
                .child(render_easing_card("Quad In-Out", ease_quad_in_out))
                .child(render_easing_card("Cubic In-Out", ease_cubic_in_out))
                .child(render_easing_card("Sin In-Out", ease_sin_in_out)),
        )
        // Row 2: Advanced easings
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .child(render_easing_card("Exp In-Out", ease_exp_in_out))
                .child(render_easing_card("Circle In-Out", ease_circle_in_out))
                .child(render_easing_card("Back In-Out", ease_back_in_out))
                .child(render_easing_card("Bounce Out", ease_bounce_out)),
        )
        // Row 3: Special easings
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_4()
                .child(render_easing_card("Elastic Out", ease_elastic_out)),
        )
        // Motion comparison
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .mt_4()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Motion Comparison"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x666666))
                        .child("Progress bars showing 50% completion with different easings:"),
                )
                .child(render_progress_row("Linear", ease_linear(0.5)))
                .child(render_progress_row("Cubic In-Out", ease_cubic_in_out(0.5)))
                .child(render_progress_row("Exp In-Out", ease_exp_in_out(0.5)))
                .child(render_progress_row("Back In-Out", ease_back_in_out(0.5)))
                .child(render_progress_row("Bounce Out", ease_bounce_out(0.5)))
                .child(render_progress_row("Elastic Out", ease_elastic_out(0.5))),
        )
        // Code example
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .mt_4()
                .p_6()
                .bg(rgb(0xf5f5f5))
                .rounded_lg()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Usage Example"),
                )
                .child(
                    div()
                        .font_family("monospace")
                        .text_sm()
                        .p_4()
                        .bg(rgb(0xffffff))
                        .rounded_md()
                        .child(
                            "use d3rs::transition::Transition;\n\
                             use d3rs::ease::ease_cubic_in_out;\n\n\
                             let mut transition = Transition::new()\n    \
                                 .duration(1000.0)  // 1 second\n    \
                                 .ease(ease_cubic_in_out)\n    \
                                 .from_to(0.0, 100.0);\n\n\
                             // Update each frame with delta time\n\
                             let value = transition.tick(16.0);",
                        ),
                ),
        )
}

/// Render an easing curve visualization
fn render_easing_card(name: &'static str, ease_fn: fn(f64) -> f64) -> Div {
    let width = 140.0_f32;
    let height = 100.0_f32;
    let padding = 10.0_f32;

    // Sample the easing function
    let num_samples = 50;
    let mut points = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let t = i as f64 / (num_samples - 1) as f64;
        let eased = ease_fn(t);
        points.push((t, eased));
    }

    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(name),
        )
        .child(
            div()
                .w(px(width))
                .h(px(height))
                .relative()
                .bg(rgb(0xf8f8f8))
                .border_1()
                .border_color(rgb(0xe0e0e0))
                .rounded_md()
                // Background grid
                .child(
                    div()
                        .absolute()
                        .left(px(padding))
                        .bottom(px(padding))
                        .w(px(width - 2.0 * padding))
                        .h_px()
                        .bg(rgb(0xdddddd)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(padding))
                        .bottom(px(padding))
                        .w_px()
                        .h(px(height - 2.0 * padding))
                        .bg(rgb(0xdddddd)),
                )
                // Draw the curve as a series of points
                .children(points.iter().map(|(t, eased)| {
                    let t_f32 = *t as f32;
                    let x = padding + t_f32 * (width - 2.0 * padding);
                    // Clamp eased value for display (back/elastic can go outside 0-1)
                    let eased_clamped = eased.clamp(-0.2, 1.2) as f32;
                    let y = height - padding - eased_clamped * (height - 2.0 * padding);

                    div()
                        .absolute()
                        .left(px(x - 1.5))
                        .top(px(y - 1.5))
                        .w(px(3.0))
                        .h(px(3.0))
                        .rounded_full()
                        .bg(rgb(0x007acc))
                })),
        )
}

/// Render a progress bar comparing easing at a given point
fn render_progress_row(name: &'static str, progress: f64) -> Div {
    let width = 300.0_f32;
    let bar_width = (progress as f32 * width).clamp(0.0, width);

    div()
        .flex()
        .items_center()
        .gap_4()
        .child(
            div()
                .w(px(120.0))
                .text_sm()
                .text_color(rgb(0x333333))
                .child(name),
        )
        .child(
            div()
                .w(px(width))
                .h(px(20.0))
                .bg(rgb(0xe8e8e8))
                .rounded(px(4.0))
                .relative()
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .h_full()
                        .w(px(bar_width))
                        .bg(rgb(0x007acc))
                        .rounded(px(4.0)),
                ),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x666666))
                .child(format!("{:.0}%", progress * 100.0)),
        )
}

use super::ShowcaseApp;
