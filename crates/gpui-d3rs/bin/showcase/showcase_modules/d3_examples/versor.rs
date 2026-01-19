use super::super::world_data::get_world_data;
use crate::{GeoProjectionType, ShowcaseApp};
use d3rs::geo::{
    GeoPath,
    projection::{ConicEqualArea, Equirectangular, Mercator, Orthographic, Stereographic},
};
use gpui::prelude::*;
use gpui::*;

pub fn render(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let width = 600.0f32;
    let height = 400.0f32;

    // Use app state for rotation
    let rotation = [app.geo_rotation_lon, app.geo_rotation_lat];
    let use_large_data = app.use_large_data;
    let current_projection = app.geo_projection_type;

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
                .child("Versor Dragging"),
        )
        .child(
            div()
                .flex()
                .gap_4()
                .mb_4()
                .items_center()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .children(GeoProjectionType::all().into_iter().map(|proj_type| {
                            let is_selected = proj_type == current_projection;
                            let bg = if is_selected {
                                rgb(0x007acc)
                            } else {
                                rgb(0xe8e8e8)
                            };
                            let text_color = if is_selected {
                                rgb(0xffffff)
                            } else {
                                rgb(0x333333)
                            };

                            div()
                                .id(ElementId::Name(
                                    format!("proj-{}", proj_type.label()).into(),
                                ))
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .cursor_pointer()
                                .bg(bg)
                                .hover(|s| {
                                    s.bg(if is_selected {
                                        rgb(0x007acc)
                                    } else {
                                        rgb(0xd0d0d0)
                                    })
                                })
                                .text_color(text_color)
                                .text_xs()
                                .child(proj_type.label())
                                .on_click(cx.listener(move |this, _, _window, _cx| {
                                    this.geo_projection_type = proj_type;
                                }))
                        })),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().text_sm().child("Dataset:"))
                        .child(
                            div()
                                .id("versor-data-toggle")
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .cursor_pointer()
                                .bg(if use_large_data {
                                    rgb(0x448844)
                                } else {
                                    rgb(0xe8e8e8)
                                })
                                .text_color(if use_large_data {
                                    rgb(0xffffff)
                                } else {
                                    rgb(0x333333)
                                })
                                .text_xs()
                                .child(if use_large_data {
                                    "Large (50m)"
                                } else {
                                    "Small"
                                })
                                .on_click(cx.listener(|this, _, _, _| {
                                    this.use_large_data = !this.use_large_data;
                                })),
                        ),
                ),
        )
        .child(
            div()
                .w(px(width))
                .h(px(height))
                .bg(rgb(0xf0f0f0))
                .relative()
                // Mouse event listeners
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

                        // Tuned sensitivity
                        this.geo_rotation_lon += delta_x as f64 * 0.5;
                        this.geo_rotation_lat -= delta_y as f64 * 0.5;
                        this.geo_rotation_lat = this.geo_rotation_lat.clamp(-90.0, 90.0);

                        this.last_mouse_pos = Some(event.position);
                    }
                }))
                .child(
                    canvas(
                        move |bounds, _, _| bounds,
                        move |bounds, _, window, _| {
                            let width = f32::from(bounds.size.width) as f64;
                            let height = f32::from(bounds.size.height) as f64;
                            let min_dim = width.min(height);

                            macro_rules! draw_geo {
                                ($projection:expr) => {{
                                    let geometry = get_world_data(use_large_data);
                                    let path = GeoPath::new($projection.clone());
                                    let d = path.render(&geometry);
                                    if let Some(p) = super::path_utils::parse_svg_path(&d, bounds) {
                                        window.paint_path(p, rgb(0x228822));
                                    }
                                }};
                            }

                            if matches!(
                                current_projection,
                                GeoProjectionType::Orthographic | GeoProjectionType::Stereographic
                            ) {
                                let center = bounds.origin
                                    + point(px(width as f32 / 2.0), px(height as f32 / 2.0));
                                // Scale factor is min_dim / 2.0 * 0.9
                                let radius = px(min_dim as f32 / 2.0 * 0.9);
                                let sphere_bounds = Bounds {
                                    origin: center - point(radius, radius),
                                    size: size(radius * 2.0, radius * 2.0),
                                };
                                window.paint_quad(PaintQuad {
                                    bounds: sphere_bounds,
                                    corner_radii: Corners::all(radius),
                                    background: rgb(0xe0e0ff).into(),
                                    border_widths: Edges::all(px(1.0)),
                                    border_color: rgb(0x000000).into(),
                                    border_style: BorderStyle::default(),
                                });
                            }

                            // Dynamic scaling based on bounds
                            let pi = std::f64::consts::PI;
                            match current_projection {
                                GeoProjectionType::Mercator => {
                                    // Mercator is roughly square (up to ~85 deg lat)
                                    let scale = min_dim / (2.0 * pi) * 0.9;
                                    let p = Mercator::new()
                                        .scale(scale)
                                        .translate(width / 2.0, height / 2.0)
                                        .rotate(rotation[0], 0.0, 0.0);
                                    draw_geo!(p);
                                }
                                GeoProjectionType::Orthographic => {
                                    // Radius = scale
                                    let scale = min_dim / 2.0 * 0.9;
                                    let p = Orthographic::new()
                                        .scale(scale)
                                        .translate(width / 2.0, height / 2.0)
                                        .rotate(rotation[0], rotation[1], 0.0);
                                    draw_geo!(p);
                                }
                                GeoProjectionType::Equirectangular => {
                                    // Width = 2*PI*k, Height = PI*k
                                    // k = min(width/(2*PI), height/PI)
                                    let scale = (width / 2.0).min(height) / pi * 0.9;
                                    let p = Equirectangular::new()
                                        .scale(scale)
                                        .translate(width / 2.0, height / 2.0)
                                        .rotate(rotation[0], rotation[1], 0.0);
                                    draw_geo!(p);
                                }
                                GeoProjectionType::Stereographic => {
                                    // Stereographic projects to infinity at horizon, use smaller scale
                                    let scale = min_dim / 4.0 * 0.9;
                                    let p = Stereographic::new()
                                        .scale(scale)
                                        .translate(width / 2.0, height / 2.0)
                                        .rotate(rotation[0], rotation[1], 0.0);
                                    draw_geo!(p);
                                }
                                GeoProjectionType::ConicEqualArea => {
                                    // Conic fits well in square or landscape
                                    let scale = min_dim / 6.0; // Conservative scale
                                    let p = ConicEqualArea::new()
                                        .scale(scale)
                                        .translate(width / 2.0, height / 2.0)
                                        .center(0.0, 0.0)
                                        .rotate(rotation[0], rotation[1], 0.0);
                                    draw_geo!(p);
                                }
                            };
                        },
                    )
                    .size_full(),
                ),
        )
}
