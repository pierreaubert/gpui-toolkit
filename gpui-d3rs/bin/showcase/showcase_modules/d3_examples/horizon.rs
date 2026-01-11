use crate::ShowcaseApp;
use d3rs::scale::{LinearScale, Scale};
use gpui::prelude::*;
use gpui::*;

pub fn render(app: &ShowcaseApp, _cx: &mut Context<ShowcaseApp>) -> Div {
    let width = 800.0;
    let height = 100.0; // Height per band
    let bands = 4;

    // Use realtime data from app state
    let data = &app.horizon_data;

    let min_val = -30.0;
    let max_val = 30.0;

    let x_scale = LinearScale::new()
        .domain(0.0, data.len() as f64 - 1.0)
        .range(0.0, width);

    let range = max_val - min_val;
    let step = range / bands as f64;

    let y_scale = LinearScale::new()
        .domain(0.0, step) // Map one band height
        .range(height, 0.0); // Upwards

    let colors = [rgb(0xeff3ff), rgb(0xbdd7e7), rgb(0x6baed6), rgb(0x2171b5)];

    // Generate path strings
    let mut band_paths = Vec::new();
    for b in 0..bands {
        let band_idx = b;
        let mut path_d = String::new();
        let y0 = y_scale.scale(0.0);
        path_d.push_str(&format!("M {} {}", x_scale.scale(0.0), y0));

        for (i, &v) in data.iter().enumerate() {
            let val_abs = v.abs();
            let remainder = val_abs - (band_idx as f64 * step);
            let y = if remainder < 0.0 {
                0.0
            } else {
                remainder.min(step)
            };
            path_d.push_str(&format!(
                " L {} {}",
                x_scale.scale(i as f64),
                y_scale.scale(y)
            ));
        }
        path_d.push_str(&format!(
            " L {} {}",
            x_scale.scale((data.len() - 1) as f64),
            y0
        ));
        path_d.push_str(" Z");
        band_paths.push(path_d);
    }

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
                .child("Horizon Chart (Realtime)"),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(rgb(0xffffff))
                .relative()
                .overflow_hidden()
                .child(canvas(
                    move |bounds, _, _| {
                        let parsed: Vec<_> = band_paths
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
                )),
        )
}
