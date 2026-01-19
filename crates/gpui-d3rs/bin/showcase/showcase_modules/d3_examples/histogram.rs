use crate::ShowcaseApp;
use d3rs::scale::{LinearScale, Scale};
use gpui::prelude::*;
use gpui::*;

pub fn render(_app: &ShowcaseApp, _cx: &mut Context<ShowcaseApp>) -> Div {
    let width = 600.0;
    let height = 400.0;
    let margin_left = 60.0;
    let margin_right = 20.0;
    let margin_top = 20.0;
    let margin_bottom = 50.0;

    // Generate random data binned
    // In a real app we'd use random generator, here fixed mock
    let data = vec![
        1.0, 2.0, 2.5, 3.0, 3.5, 3.5, 4.0, 4.0, 4.0, 5.0, 5.5, 6.0, 9.0,
    ];

    // Binning
    let min_val = data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_val = data.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let bin_count = 10;

    // Manual binning logic:
    let mut bins = vec![0usize; bin_count];
    let step = (max_val - min_val) / bin_count as f64;
    for &d in &data {
        let idx = ((d - min_val) / step).floor() as usize;
        if idx < bin_count {
            bins[idx] += 1;
        } else if idx == bin_count {
            // Handle max value edge case
            bins[idx - 1] += 1;
        }
    }

    // Scales
    let chart_width = width - margin_left - margin_right;
    let chart_height = height - margin_top - margin_bottom;

    let x_scale = LinearScale::new()
        .domain(min_val, max_val)
        .range(0.0, chart_width);

    let max_bin = *bins.iter().max().unwrap_or(&0) as f64;
    let y_scale = LinearScale::new()
        .domain(0.0, max_bin)
        .range(chart_height, 0.0);

    let bar_width = chart_width / bin_count as f64 - 1.0;

    // Y-axis tick values (0 to max_bin, integer steps)
    let y_ticks: Vec<usize> = (0..=max_bin as usize).collect();

    div()
        .flex()
        .flex_col()
        .size_full()
        .p_4()
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .mb_4()
                .child("Histogram"),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(rgb(0xffffff))
                .border_1()
                .border_color(rgb(0xcccccc))
                .relative()
                // Y-axis label
                .child(
                    div()
                        .absolute()
                        .left(px(5.0))
                        .top(px((margin_top + chart_height / 2.0 - 40.0) as f32))
                        .text_xs()
                        .text_color(rgb(0x666666))
                        .child("Count"),
                )
                // Y-axis ticks and labels
                .children(y_ticks.iter().map(|&count| {
                    let y = y_scale.scale(count as f64);
                    div()
                        .absolute()
                        .left(px((margin_left - 30.0) as f32))
                        .top(px((margin_top + y - 6.0) as f32))
                        .w(px(25.0))
                        .text_xs()
                        .text_color(rgb(0x333333))
                        .flex()
                        .justify_end()
                        .child(format!("{}", count))
                }))
                // Y-axis tick marks
                .children(y_ticks.iter().map(|&count| {
                    let y = y_scale.scale(count as f64);
                    div()
                        .absolute()
                        .left(px((margin_left - 5.0) as f32))
                        .top(px((margin_top + y) as f32))
                        .w(px(5.0))
                        .h(px(1.0))
                        .bg(rgb(0x000000))
                }))
                // Y-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(1.0))
                        .h(px(chart_height as f32))
                        .bg(rgb(0x000000)),
                )
                // X-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + chart_height) as f32))
                        .w(px(chart_width as f32))
                        .h(px(1.0))
                        .bg(rgb(0x000000)),
                )
                // Bars
                .children((0..bin_count).map(|i| {
                    let count = bins[i] as f64;
                    let x0 = x_scale.scale(min_val + i as f64 * step);
                    let y0 = y_scale.scale(count);
                    let h = chart_height - y0;

                    div()
                        .absolute()
                        .left(px((margin_left + x0) as f32))
                        .top(px((margin_top + y0) as f32))
                        .w(px(bar_width as f32))
                        .h(px(h as f32))
                        .bg(rgb(0x4682b4))
                }))
                // X-axis ticks and labels (bin boundaries)
                .children((0..=bin_count).map(|i| {
                    let val = min_val + i as f64 * step;
                    let x = x_scale.scale(val);

                    div()
                        .absolute()
                        .left(px((margin_left + x - 12.0) as f32))
                        .top(px((margin_top + chart_height) as f32))
                        .w(px(24.0))
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(
                            // Tick mark
                            div().w(px(1.0)).h(px(5.0)).bg(rgb(0x000000)),
                        )
                        .child(
                            // Label
                            div()
                                .text_xs()
                                .text_color(rgb(0x333333))
                                .child(format!("{:.1}", val)),
                        )
                }))
                // X-axis label
                .child(
                    div()
                        .absolute()
                        .left(px((margin_left + chart_width / 2.0 - 20.0) as f32))
                        .top(px((height - 15.0) as f32))
                        .text_xs()
                        .text_color(rgb(0x666666))
                        .child("Value"),
                ),
        )
}
