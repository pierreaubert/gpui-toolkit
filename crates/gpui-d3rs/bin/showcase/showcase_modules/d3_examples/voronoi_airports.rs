//! World Airports Voronoi — Observable example
//!
//! Renders Voronoi cells for airports on an orthographic globe.
//! Source: <https://observablehq.com/@d3/world-airports-voronoi>

use crate::ShowcaseApp;
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const AIRPORTS_CSV: &str = include_str!("../../data/airports.csv");

/// Build a thin closed ribbon from (x0,y0) to (x1,y1) with given half-width.
fn ribbon(x0: f64, y0: f64, x1: f64, y1: f64, hw: f64) -> d3rs::shape::path::Path {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.01 {
        return D3PathBuilder::new().build();
    }
    let nx = -dy / len * hw;
    let ny = dx / len * hw;
    D3PathBuilder::new()
        .move_to(x0 + nx, y0 + ny)
        .line_to(x1 + nx, y1 + ny)
        .line_to(x1 - nx, y1 - ny)
        .line_to(x0 - nx, y0 - ny)
        .close_path()
        .build()
}

pub fn render(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let coords = d3rs::examples::voronoi_airports::load_csv(AIRPORTS_CSV);
    let rotation = (app.geo_rotation_lon, app.geo_rotation_lat);
    let zoom = app.geo_zoom;
    let result = d3rs::examples::voronoi_airports::compute_with_zoom(&coords, rotation, zoom);

    let width = result.width;
    let height = result.height;

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // 1. Ocean disc (filled circle — light blue)
    d3_paths.push(result.globe_outline.clone());
    all_colors.push(hsla(0.56, 0.3, 0.9, 1.0));

    // 2. Graticule ribbons
    {
        use d3rs::shape::path::PathCommand;
        let cmds = result.graticule_path.commands();
        let mut prev: Option<(f64, f64)> = None;
        for cmd in cmds {
            match cmd {
                PathCommand::MoveTo { x, y } => {
                    prev = Some((*x, *y));
                }
                PathCommand::LineTo { x, y } => {
                    if let Some((px, py)) = prev {
                        let r = ribbon(px, py, *x, *y, 0.4);
                        if !r.commands().is_empty() {
                            d3_paths.push(r);
                            all_colors.push(hsla(0.56, 0.2, 0.8, 0.4));
                        }
                    }
                    prev = Some((*x, *y));
                }
                _ => {
                    prev = None;
                }
            }
        }
    }

    // 3. Voronoi mesh ribbons
    {
        use d3rs::shape::path::PathCommand;
        let cmds = result.voronoi_mesh_path.commands();
        let mut prev: Option<(f64, f64)> = None;
        for cmd in cmds {
            match cmd {
                PathCommand::MoveTo { x, y } => {
                    prev = Some((*x, *y));
                }
                PathCommand::LineTo { x, y } => {
                    if let Some((px, py)) = prev {
                        let r = ribbon(px, py, *x, *y, 0.4);
                        if !r.commands().is_empty() {
                            d3_paths.push(r);
                            all_colors.push(hsla(0.0, 0.0, 0.2, 0.5));
                        }
                    }
                    prev = Some((*x, *y));
                }
                _ => {
                    prev = None;
                }
            }
        }
    }

    // 4. Airport dots
    let n_sides = 10;
    let dot_r = 1.2;
    for (px, py) in result.projected_points.iter().flatten() {
        let mut builder = D3PathBuilder::new();
        for v in 0..n_sides {
            let angle = std::f64::consts::TAU * v as f64 / n_sides as f64;
            let x = px + dot_r * angle.cos();
            let y = py + dot_r * angle.sin();
            if v == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());
        all_colors.push(hsla(0.0, 0.85, 0.5, 0.9));
    }

    let visible_count = result
        .projected_points
        .iter()
        .filter(|p| p.is_some())
        .count();

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
                .child("World Airports Voronoi"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/world-airports-voronoi — {} airports ({} visible)",
            result.point_count, visible_count
        )))
        .child(
            div()
                .id("voronoi-globe")
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .cursor_pointer()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseDownEvent, _, _| {
                        this.is_dragging = true;
                        this.last_mouse_pos = Some(event.position);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, _| {
                        this.is_dragging = false;
                        this.last_mouse_pos = None;
                    }),
                )
                .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, _| {
                    if this.is_dragging
                        && let Some(last_pos) = this.last_mouse_pos
                    {
                        let delta_x: f32 = (event.position.x - last_pos.x).into();
                        let delta_y: f32 = (event.position.y - last_pos.y).into();
                        this.geo_rotation_lon += delta_x as f64 * 0.5;
                        this.geo_rotation_lat -= delta_y as f64 * 0.5;
                        this.geo_rotation_lat = this.geo_rotation_lat.clamp(-90.0, 90.0);
                        this.last_mouse_pos = Some(event.position);
                    }
                }))
                .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, _| {
                    let delta = match event.delta {
                        ScrollDelta::Lines(lines) => {
                            let y: f32 = lines.y;
                            y * 0.15
                        }
                        ScrollDelta::Pixels(pixels) => {
                            let y: f32 = pixels.y.into();
                            y * 0.003
                        }
                    };
                    this.geo_zoom = (this.geo_zoom * (1.0 + delta as f64)).clamp(0.3, 10.0);
                }))
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
