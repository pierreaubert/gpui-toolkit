//! Calendar Heatmap — Observable example
//!
//! Loads dji.csv (daily Dow Jones data) and renders a multi-year calendar
//! heatmap with one row per year, 52 weeks × 7 days.
//! Uses d3rs PathBuilder, LinearScale, and SequentialScheme.
//!
//! Source: <https://observablehq.com/@d3/calendar>

use crate::ShowcaseApp;
use d3rs::color::SequentialScheme as SeqScheme;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const DJI_CSV: &str = include_str!("../../data/dji.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let cell_size = 10.0;
    let cell_pad = 1.5;
    let year_height = 7.0 * (cell_size + cell_pad) + 15.0; // 7 days + label space
    let margin_left = 40.0;
    let margin_top = 20.0;

    // Parse DJI CSV: Date,Open,High,Low,Close,Adj Close,Volume
    // Compute daily return: (Close - Open) / Open
    struct DayData {
        year: u32,
        month: u32,
        day: u32,
        pct_change: f64,
    }

    let days: Vec<DayData> = DJI_CSV
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 5 {
                return None;
            }
            let date = cols[0];
            let year: u32 = date[..4].parse().ok()?;
            let month: u32 = date[5..7].parse().ok()?;
            let day: u32 = date[8..10].parse().ok()?;
            let open: f64 = cols[1].parse().ok()?;
            let close: f64 = cols[4].parse().ok()?;
            if open <= 0.0 {
                return None;
            }
            let pct_change = (close - open) / open * 100.0;
            Some(DayData {
                year,
                month,
                day,
                pct_change,
            })
        })
        .collect();

    if days.is_empty() {
        return div().child("No DJI data found");
    }

    // Group by year
    let mut years: Vec<u32> = days.iter().map(|d| d.year).collect();
    years.sort();
    years.dedup();

    // Value range for diverging color scale (negative = red, positive = green)
    let max_abs = days
        .iter()
        .map(|d| d.pct_change.abs())
        .fold(0.0f64, f64::max)
        .min(5.0); // Cap at 5% for better color contrast

    // Diverging: t=0 → red (negative), t=0.5 → white (zero), t=1 → green (positive)
    let color_scale = LinearScale::new().domain(-max_abs, max_abs).range(0.0, 1.0);
    let scheme = SeqScheme::turbo();

    // Day-of-year → (week, dow) computation
    let month_days: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let width = margin_left + 53.0 * (cell_size + cell_pad) + 10.0;
    let height = margin_top + years.len() as f64 * year_height + 10.0;

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Year labels and month separators
    let mut year_labels: Vec<(u32, f64)> = Vec::new();

    for (yi, &year) in years.iter().enumerate() {
        let y_base = margin_top + yi as f64 * year_height + 12.0;
        year_labels.push((year, y_base));

        // Month day offsets for this year
        let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let mut mdays = month_days;
        if is_leap {
            mdays[1] = 29;
        }
        let mut month_offsets = [0u32; 12];
        for i in 1..12 {
            month_offsets[i] = month_offsets[i - 1] + mdays[i - 1];
        }

        // Determine day-of-week for Jan 1 (Zeller's simplified)
        // 2000-01-01 was a Saturday (dow=6)
        let days_since_2000 = {
            let mut d: i64 = 0;
            for y in 2000..year {
                let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
                d += if leap { 366 } else { 365 };
            }
            d
        };
        let jan1_dow = ((6 + days_since_2000 % 7 + 7) % 7) as u32; // 0=Sun

        for day_data in days.iter().filter(|d| d.year == year) {
            let mi = (day_data.month as usize).saturating_sub(1).min(11);
            let day_of_year = month_offsets[mi] + day_data.day.saturating_sub(1);
            let absolute_day = day_of_year + jan1_dow;
            let week = absolute_day / 7;
            let dow = absolute_day % 7;

            let x = margin_left + week as f64 * (cell_size + cell_pad);
            let y = y_base + dow as f64 * (cell_size + cell_pad);

            let path = D3PathBuilder::new()
                .move_to(x, y)
                .line_to(x + cell_size, y)
                .line_to(x + cell_size, y + cell_size)
                .line_to(x, y + cell_size)
                .close_path()
                .build();
            d3_paths.push(path);

            let t = color_scale.scale(day_data.pct_change.clamp(-max_abs, max_abs));
            let d3_color = scheme.get(t);
            all_colors.push(d3_color.to_rgba().into());
        }
    }

    // Month labels along top
    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    div()
        .flex()
        .flex_col()
        .size_full()
        .p_4()
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .mb_2()
                .child("Calendar Heatmap — DJI Daily Returns"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/calendar — {} trading days, {} years from dji.csv",
            days.len(),
            years.len()
        )))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .mb_2()
                .child(div().text_xs().child(format!("-{max_abs:.0}%")))
                .child(
                    div()
                        .flex()
                        .h(px(10.0))
                        .w(px(200.0))
                        .rounded_sm()
                        .overflow_hidden()
                        .children((0..20).map(|i| {
                            let t = i as f64 / 19.0;
                            let c = scheme.get(t);
                            div().flex_1().h_full().bg(c.to_rgba())
                        })),
                )
                .child(div().text_xs().child(format!("+{max_abs:.0}%"))),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .relative()
                .overflow_hidden()
                .child(
                    canvas(
                        move |bounds, _, _| {
                            d3_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_simple(p, bounds, 0.0, 0.0)
                                })
                                .collect::<Vec<_>>()
                        },
                        move |_bounds, paths, window, _| {
                            for (i, path_opt) in paths.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, all_colors[i]);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                // Year labels on the left
                .children(year_labels.into_iter().map(|(year, y)| {
                    div()
                        .absolute()
                        .left(px(2.0))
                        .top(px((y + 25.0) as f32))
                        .text_size(px(9.0))
                        .child(format!("{year}"))
                }))
                // Month labels at top
                .children((0..12).map(|mi| {
                    // Approximate week position for each month start
                    let week = [0, 4, 9, 13, 17, 22, 26, 31, 35, 39, 44, 48][mi];
                    let x = margin_left + week as f64 * (cell_size + cell_pad);
                    div()
                        .absolute()
                        .left(px(x as f32))
                        .top(px(5.0))
                        .text_size(px(8.0))
                        .child(month_names[mi].to_string())
                })),
        )
}
