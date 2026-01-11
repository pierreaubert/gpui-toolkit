use crate::ShowcaseApp;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::stack::Stack;
use d3rs::text::{VectorFontConfig, render_vector_text};
use gpui::prelude::*;
use gpui::*;

pub fn render(_app: &ShowcaseApp, _cx: &mut Context<ShowcaseApp>) -> Div {
    let width = 700.0;
    let height = 400.0;
    let margin_left = 60.0;
    let margin_right = 20.0;
    let margin_top = 20.0;
    let margin_bottom = 50.0;
    let chart_width = width - margin_left - margin_right;
    let chart_height = height - margin_top - margin_bottom;

    // Mock data approximating RIAA revenue trends (in billions USD, scaled)
    // Based on actual historical patterns:
    // - Vinyl dominated 1973-1988, declined, then small revival 2010+
    // - Cassette peaked ~1988, declined by 2000
    // - CD peaked ~2000, declined after
    // - Download peaked ~2012
    // - Streaming grew rapidly from 2015+
    let years = 1973..2018;
    let n_years = years.end - years.start;
    let mut data: Vec<Vec<f64>> = Vec::new();
    for i in 0..n_years {
        let year = 1973 + i as i32;
        let y = i as f64;

        // Vinyl: peaked ~1978, declined until 2007, small revival after
        let vinyl = if year <= 1978 {
            2.0 + y * 0.3 // Rising to peak
        } else if year <= 1988 {
            4.5 - (year - 1978) as f64 * 0.35 // Decline
        } else if year <= 2007 {
            1.0 - (year - 1988) as f64 * 0.05 // Near zero
        } else {
            0.1 + (year - 2007) as f64 * 0.04 // Small revival
        };

        // Cassette: started ~1975, peaked ~1988, gone by 2005
        let cassette = if year < 1975 {
            0.0
        } else if year <= 1988 {
            (year - 1975) as f64 * 0.5 // Rising
        } else if year <= 2005 {
            6.5 - (year - 1988) as f64 * 0.4 // Decline
        } else {
            0.0
        };

        // CD: started ~1983, peaked ~2000, declined after
        let cd = if year < 1983 {
            0.0
        } else if year <= 2000 {
            (year - 1983) as f64 * 0.8 // Rising to peak ~13.6
        } else if year <= 2018 {
            13.6 - (year - 2000) as f64 * 0.7 // Decline
        } else {
            0.0
        };

        // Download: started ~2004, peaked ~2012, declined after
        let download = if year < 2004 {
            0.0
        } else if year <= 2012 {
            (year - 2004) as f64 * 0.4 // Rising
        } else {
            3.2 - (year - 2012) as f64 * 0.5 // Decline
        };

        // Streaming: started ~2011, rapid growth
        let streaming = if year < 2011 {
            0.0
        } else {
            (year - 2011) as f64 * 0.7 // Rapid growth
        };

        data.push(vec![
            vinyl.max(0.0),
            cassette.max(0.0),
            cd.max(0.0),
            download.max(0.0),
            streaming.max(0.0),
        ]);
    }

    // Stack and Shape logic
    // Use standard stacked area chart (zero baseline) like the original D3 example
    // Order by appearance so formats appear in chronological order
    let labels = ["Vinyl", "Cassette", "CD", "Download", "Streaming"];
    let keys: Vec<String> = labels.iter().map(|s| s.to_string()).collect();

    let stack = Stack::new()
        .keys(keys)
        .offset(d3rs::shape::stack::StackOffset::None)
        .order(d3rs::shape::stack::StackOrder::Appearance);
    let series = stack.generate(&data);

    // Scales using chart area (inside margins)
    let x_scale = LinearScale::new()
        .domain(1973.0, 2018.0)
        .range(0.0, chart_width);

    let min_y = series
        .iter()
        .flat_map(|s| s.values.iter())
        .flat_map(|p| [p[0], p[1]])
        .fold(f64::INFINITY, f64::min);
    let max_y = series
        .iter()
        .flat_map(|s| s.values.iter())
        .flat_map(|p| [p[0], p[1]])
        .fold(f64::NEG_INFINITY, f64::max);
    let y_scale = LinearScale::new()
        .domain(min_y, max_y)
        .range(chart_height, 0.0);

    // Generate path strings
    let mut series_paths = Vec::new();
    for s in series {
        let mut path_d = String::new();
        if let Some(first) = s.values.first() {
            path_d.push_str(&format!(
                "M {} {}",
                x_scale.scale(1973.0),
                y_scale.scale(first[1])
            ));
        }
        for (j, p) in s.values.iter().enumerate() {
            path_d.push_str(&format!(
                " L {} {}",
                x_scale.scale(1973.0 + j as f64),
                y_scale.scale(p[1])
            ));
        }
        if let Some(last) = s.values.last() {
            path_d.push_str(&format!(
                " L {} {}",
                x_scale.scale(1973.0 + (s.values.len() - 1) as f64),
                y_scale.scale(last[0])
            ));
        }
        for (j, p) in s.values.iter().enumerate().rev() {
            path_d.push_str(&format!(
                " L {} {}",
                x_scale.scale(1973.0 + j as f64),
                y_scale.scale(p[0])
            ));
        }
        path_d.push_str(" Z");
        series_paths.push(path_d);
    }

    let colors = [
        rgb(0x8dd3c7),
        rgb(0xffffb3),
        rgb(0xbebada),
        rgb(0xfb8072),
        rgb(0x80b1d3),
    ];
    let labels = ["Vinyl", "Cassette", "CD", "Download", "Streaming"];

    let legend_items = labels
        .iter()
        .enumerate()
        .map(|(i, &label)| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().size_3().bg(colors[i]))
                .child(div().text_xs().child(label))
        })
        .collect::<Vec<_>>();

    // X-axis ticks (every 5 years)
    let x_ticks: Vec<i32> = (1975..=2015).step_by(5).collect();

    // Y-axis ticks (revenue values)
    let y_tick_step = ((max_y - min_y) / 5.0).ceil();
    let y_ticks: Vec<f64> = (0..=5)
        .map(|i| (i as f64 * y_tick_step).min(max_y))
        .collect();

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
                .child("Revenue by Music Format 1973–2018"),
        )
        .child(div().flex().gap_4().mb_4().children(legend_items))
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(rgb(0xffffff))
                .border_1()
                .border_color(rgb(0xcccccc))
                .relative()
                // Y-axis label (rotated 90 degrees, reading bottom-to-top)
                .child(
                    div()
                        .absolute()
                        .left(px(2.0))
                        .top(px((margin_top + chart_height / 2.0 - 30.0) as f32))
                        .child(render_vector_text(
                            "Revenue ($B)",
                            &VectorFontConfig::vertical_bottom_to_top(10.0, rgb(0x666666).into()),
                        )),
                )
                // Y-axis ticks and labels
                .children(y_ticks.iter().map(|&val| {
                    let y = y_scale.scale(val);
                    let label_config = VectorFontConfig::horizontal(10.0, rgb(0x333333).into());
                    div()
                        .absolute()
                        .left(px((margin_left - 35.0) as f32))
                        .top(px((margin_top + y - 6.0) as f32))
                        .w(px(30.0))
                        .flex()
                        .justify_end()
                        .child(render_vector_text(&format!("{:.0}", val), &label_config))
                }))
                // Y-axis tick marks
                .children(y_ticks.iter().map(|&val| {
                    let y = y_scale.scale(val);
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
                // X-axis ticks and labels
                .children(x_ticks.iter().map(|&year| {
                    let x = x_scale.scale(year as f64);
                    let label_config = VectorFontConfig::horizontal(10.0, rgb(0x333333).into());
                    div()
                        .absolute()
                        .left(px((margin_left + x - 15.0) as f32))
                        .top(px((margin_top + chart_height) as f32))
                        .w(px(30.0))
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(div().w(px(1.0)).h(px(5.0)).bg(rgb(0x000000)))
                        .child(render_vector_text(&format!("{}", year), &label_config))
                }))
                // X-axis label
                .child(
                    div()
                        .absolute()
                        .left(px((margin_left + chart_width / 2.0 - 10.0) as f32))
                        .top(px((height - 18.0) as f32))
                        .child(render_vector_text(
                            "Year",
                            &VectorFontConfig::horizontal(10.0, rgb(0x666666).into()),
                        )),
                )
                // Chart area with stacked areas
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(chart_width as f32))
                        .h(px(chart_height as f32))
                        .child(
                            canvas(
                                move |bounds, _, _| {
                                    let parsed: Vec<_> = series_paths
                                        .iter()
                                        .map(|d| super::path_utils::parse_svg_path(d, bounds))
                                        .collect();
                                    parsed
                                },
                                move |_bounds, paths, window, _| {
                                    for (i, path_opt) in paths.into_iter().enumerate() {
                                        if let Some(path) = path_opt {
                                            window.paint_path(path, colors[i % colors.len()]);
                                        }
                                    }
                                },
                            )
                            .size_full(),
                        ),
                ),
        )
}
