//! Voronoi Stippling — weighted Lloyd's relaxation on an image.
//! Source: <https://observablehq.com/@mbostock/voronoi-stippling>

use crate::ShowcaseApp;
use d3rs::examples::voronoi_stippling::StipplingState;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const WOOD_JPEG: &[u8] = include_bytes!("../../data/wood.jpeg");

/// Decode JPEG to density map for white-on-black stippling.
/// Light pixels → high density (place dots there), dark uniform areas → low density.
fn decode_density(jpeg_bytes: &[u8]) -> (Vec<f64>, usize, usize) {
    let img = image::load_from_memory(jpeg_bytes).expect("failed to decode JPEG");
    let gray = img.to_luma8();
    let width = gray.width() as usize;
    let height = gray.height() as usize;
    // For white dots on black: density = luma (light areas get more dots)
    let density: Vec<f64> = gray
        .pixels()
        .map(|p| (p.0[0] as f64 / 255.0).max(0.0))
        .collect();
    (density, width, height)
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    // Lazy-init: decode image and create state on first render
    if app.stippling_density.is_none() {
        let (density, w, h) = decode_density(WOOD_JPEG);
        let n = (w * h) / 40;
        app.stippling_state = Some(StipplingState::new(&density, w, h, n));
        app.stippling_img_size = (w, h);
        app.stippling_density = Some(density);
        app.stippling_iterations = 0;
    }

    let (img_w, img_h) = app.stippling_img_size;
    let current = app.stippling_iterations;
    let target = app.stippling_target;
    let theme = cx.theme();

    // Advance one iteration per frame if we haven't reached target
    if current < target {
        if let (Some(state), Some(density)) =
            (app.stippling_state.as_mut(), app.stippling_density.as_ref())
        {
            state.step(density);
            app.stippling_iterations = state.iteration;
        }
        // Schedule next frame via async timer — cx.notify() alone doesn't
        // trigger a repaint without user interaction
        cx.spawn(async move |this: WeakEntity<ShowcaseApp>, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1))
                .await;
            let _ = cx.update(|cx| {
                this.update(cx, |_, cx| {
                    cx.notify();
                })
            });
        })
        .detach();
    }

    // Build paths from current state
    let (d3_paths, n_points) = if let Some(state) = app.stippling_state.as_ref() {
        (state.build_dot_paths(), state.n)
    } else {
        (Vec::new(), 0)
    };

    let width = img_w as f64;
    let height = img_h as f64;
    let dot_color = hsla(0.0, 0.0, 1.0, 1.0);
    let all_colors: Vec<Hsla> = vec![dot_color; d3_paths.len()];

    let presets: Vec<usize> = vec![1, 5, 10, 20, 40, 60, 80, 120];
    let current = app.stippling_iterations; // re-read after possible step

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
                .child("Voronoi Stippling"),
        )
        .child(div().text_xs().mb_1().child(format!(
            "Source: observablehq.com/@mbostock/voronoi-stippling — {} dots, wood.jpeg {}×{}",
            n_points, img_w, img_h
        )))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .mb_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Iterations:"),
                )
                .children(presets.into_iter().map(|preset| {
                    let is_selected = preset == target;
                    let bg = if is_selected {
                        theme.accent
                    } else {
                        theme.surface_hover
                    };
                    let tc = if is_selected {
                        theme.text_on_accent
                    } else {
                        theme.text_primary
                    };
                    let hover_bg = if is_selected {
                        theme.accent_hover
                    } else {
                        theme.muted
                    };
                    div()
                        .id(ElementId::Name(format!("stip-{preset}").into()))
                        .px_2()
                        .py(px(2.0))
                        .rounded_md()
                        .bg(bg)
                        .text_xs()
                        .text_color(tc)
                        .cursor_pointer()
                        .hover(move |s| s.bg(hover_bg))
                        .child(format!("{preset}"))
                        .on_click(cx.listener(move |this, _, _, _| {
                            this.stippling_target = preset;
                            // Don't reset state — continue from current position
                            // If user picks a lower target, just stop animating
                        }))
                }))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if current < target {
                            theme.accent
                        } else {
                            theme.text_muted
                        })
                        .ml_2()
                        .child(format!(
                            "{}{}",
                            current,
                            if current < target {
                                format!(" → {target}")
                            } else {
                                String::new()
                            }
                        )),
                )
                // Reset button
                .child(
                    div()
                        .id("stip-reset")
                        .px_2()
                        .py(px(2.0))
                        .rounded_md()
                        .bg(rgb(0xffcccc))
                        .text_xs()
                        .cursor_pointer()
                        .ml_2()
                        .child("Reset")
                        .on_click(cx.listener(|this, _, _, _| {
                            // Re-seed from scratch
                            this.stippling_state = None;
                            this.stippling_density = None;
                            this.stippling_iterations = 0;
                        })),
                ),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(rgb(0x111111))
                .border_1()
                .border_color(rgb(0x333333))
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
                ),
        )
}
