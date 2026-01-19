use d3rs::geo::{
    ConicEqualArea, Equirectangular, GeoPath, Graticule, Mercator, Orthographic, Projection,
    Stereographic,
};
use gpui::*;

use super::ShowcaseApp;
use super::world_data::get_world_data;
use crate::GeoProjectionType;

/// Famous cities with their coordinates
const CITIES: &[(&str, f64, f64)] = &[
    ("New York", -74.0, 40.7),
    ("London", -0.1, 51.5),
    ("Paris", 2.3, 48.9),
    ("Tokyo", 139.7, 35.7),
    ("Sydney", 151.2, -33.9),
    ("Rio de Janeiro", -43.2, -22.9),
    ("Cairo", 31.2, 30.0),
    ("Moscow", 37.6, 55.8),
    ("Mumbai", 72.9, 19.1),
    ("Beijing", 116.4, 39.9),
];

pub fn render(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let current_projection = app.geo_projection_type;
    let rotation_lon = app.geo_rotation_lon;
    let rotation_lat = app.geo_rotation_lat;
    let use_large_data = app.use_large_data;

    // Map dimensions
    let map_width = 800.0_f64;
    let map_height = 500.0_f64;
    let center_x = map_width / 2.0;
    let center_y = map_height / 2.0;

    div()
        .flex()
        .flex_col()
        .gap_6()
        .size_full()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Geographic Projections Demo"),
        )
        .child(
            div()
                .text_base()
                .text_color(rgb(0x666666))
                .max_w(px(700.0))
                .child("The d3-geo module provides geographic projections for mapping spherical coordinates (longitude, latitude) to planar coordinates (x, y). Select a projection below to see how it transforms the globe."),
        )
        // Projection selector
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Select Projection:"),
                )
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
                                .id(ElementId::Name(proj_type.label().into()))
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .cursor_pointer()
                                .bg(bg)
                                .hover(|s| s.bg(if is_selected { rgb(0x007acc) } else { rgb(0xd0d0d0) }))
                                .text_color(text_color)
                                .text_sm()
                                .child(proj_type.label())
                                .on_click(cx.listener(move |this, _, _window, _cx| {
                                    this.geo_projection_type = proj_type;
                                }))
                        })),
                ),
        )
        // Data Selector
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Select Dataset:"),
                )
                .child(
                    div()
                        .id("geo-data-toggle")
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .bg(if use_large_data { rgb(0x448844) } else { rgb(0xe8e8e8) })
                        .text_color(if use_large_data { rgb(0xffffff) } else { rgb(0x333333) })
                        .text_sm()
                        .max_w(px(200.0))
                        .child(if use_large_data { "Large (50m)" } else { "Small (Simplified)" })
                        .on_click(cx.listener(|this, _, _, _| {
                            this.use_large_data = !this.use_large_data;
                        }))
                ),
        )
        // Map visualization
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format!("{} Projection", current_projection.label())),
                )
                .child(
                    div()
                        .w(px(map_width as f32))
                        .h(px(map_height as f32))
                        .relative()
                        .bg(rgb(0xe8f4fc))
                        .border_1()
                        .border_color(rgb(0xcccccc))
                        .rounded_lg()
                        .overflow_hidden()
                        .child(
                            canvas(
                                move |bounds, _, _| bounds,
                                move |bounds, _, window, _| {
                                    let scale = match current_projection {
                                        GeoProjectionType::Mercator => map_height / 3.0,
                                        GeoProjectionType::Equirectangular => map_height / 2.0,
                                        GeoProjectionType::Orthographic => map_height / 2.5,
                                        GeoProjectionType::Stereographic => map_height / 4.0,
                                        GeoProjectionType::ConicEqualArea => map_height / 4.5,
                                    };

                                     // 1. Draw Continents (Fill)
                                     {
                                         let world_data = get_world_data(use_large_data);
                                         let continents_svg = match current_projection {
                                              GeoProjectionType::Mercator => { let p = Mercator::new().scale(scale).translate(center_x, center_y).rotate(rotation_lon, rotation_lat, 0.0); GeoPath::new(p).render(world_data) },
                                              GeoProjectionType::Equirectangular => { let p = Equirectangular::new().scale(scale).translate(center_x, center_y).rotate(rotation_lon, rotation_lat, 0.0); GeoPath::new(p).render(world_data) },
                                              GeoProjectionType::Orthographic => { let p = Orthographic::new().scale(scale).translate(center_x, center_y).rotate(rotation_lon, rotation_lat, 0.0); GeoPath::new(p).render(world_data) },
                                              GeoProjectionType::Stereographic => { let p = Stereographic::new().scale(scale).translate(center_x, center_y).rotate(rotation_lon, rotation_lat, 0.0); GeoPath::new(p).render(world_data) },
                                              GeoProjectionType::ConicEqualArea => { let p = ConicEqualArea::new().scale(scale).translate(center_x, center_y + 50.0).center(0.0, 30.0).rotate(rotation_lon, rotation_lat, 0.0); GeoPath::new(p).render(world_data) },
                                         };

                                         // VERY simple M/L parser for demo purposes
                                         // In production, use a real SVG path parser or d3rs should output Path events directly
                                         // Use PathBuilder::fill() for filled shapes
                                         let mut builder = PathBuilder::fill();
                                         let tokens = continents_svg.replace("M", " M ").replace("L", " L ").replace("Z", " Z ").replace("z", " Z ");
                                         let parts: Vec<&str> = tokens.split_whitespace().collect();
                                         let mut i = 0;
                                         while i < parts.len() {
                                             match parts[i] {
                                                 "M" => {
                                                     if i+1 < parts.len() {
                                                         let coords: Vec<&str> = parts[i+1].split(',').collect();
                                                         if coords.len() == 2
                                                             && let (Ok(x), Ok(y)) = (coords[0].parse::<f32>(), coords[1].parse::<f32>())
                                                             && x.is_finite() && y.is_finite()
                                                         {
                                                             builder.move_to(bounds.origin + point(px(x), px(y)));
                                                         }
                                                         i += 2;
                                                     } else { i += 1; }
                                                 },
                                                 "L" => {
                                                     if i+1 < parts.len() {
                                                         let coords: Vec<&str> = parts[i+1].split(',').collect();
                                                         if coords.len() == 2
                                                             && let (Ok(x), Ok(y)) = (coords[0].parse::<f32>(), coords[1].parse::<f32>())
                                                             && x.is_finite() && y.is_finite()
                                                         {
                                                             builder.line_to(bounds.origin + point(px(x), px(y)));
                                                         }
                                                         i += 2;
                                                     } else { i += 1; }
                                                 },
                                                 "Z" => {
                                                     builder.close();
                                                     i += 1;
                                                 },
                                                 _ => i += 1,
                                             }
                                         }

                                         if let Ok(path) = builder.build() {
                                             // paint_path takes (path, color) in recent GPUI versions?
                                             // Looking at the error: unexpected argument #3 of type `gpui::Rgba`
                                             // It seems paint_path expects 2 arguments: self and path? No, method signature is `paint_path(&mut self, path, color)`.
                                             // Let's try passing just the path and color (fill color).
                                             window.paint_path(path, rgb(0xd6e4ff));
                                         }
                                     }

                                    // 2. Draw Graticule
                                    let graticule = Graticule::new().step([30.0, 30.0]);
                                    for line in graticule.lines() {
                                        let grid_svg = match current_projection {
                                              GeoProjectionType::Mercator => { let p = Mercator::new().scale(scale).translate(center_x, center_y).rotate(rotation_lon, rotation_lat, 0.0); GeoPath::new(p).render(&d3rs::geo::GeoJsonGeometry::LineString(line)) },
                                              GeoProjectionType::Equirectangular => { let p = Equirectangular::new().scale(scale).translate(center_x, center_y).rotate(rotation_lon, rotation_lat, 0.0); GeoPath::new(p).render(&d3rs::geo::GeoJsonGeometry::LineString(line)) },
                                              GeoProjectionType::Orthographic => { let p = Orthographic::new().scale(scale).translate(center_x, center_y).rotate(rotation_lon, rotation_lat, 0.0); GeoPath::new(p).render(&d3rs::geo::GeoJsonGeometry::LineString(line)) },
                                              GeoProjectionType::Stereographic => { let p = Stereographic::new().scale(scale).translate(center_x, center_y).rotate(rotation_lon, rotation_lat, 0.0); GeoPath::new(p).render(&d3rs::geo::GeoJsonGeometry::LineString(line)) },
                                              GeoProjectionType::ConicEqualArea => { let p = ConicEqualArea::new().scale(scale).translate(center_x, center_y + 50.0).center(0.0, 30.0).rotate(rotation_lon, rotation_lat, 0.0); GeoPath::new(p).render(&d3rs::geo::GeoJsonGeometry::LineString(line)) },
                                         };

                                        // Use PathBuilder::stroke() for lines
                                        let mut builder = PathBuilder::stroke(px(1.0));
                                         let tokens = grid_svg.replace("M", " M ").replace("L", " L ");
                                         let parts: Vec<&str> = tokens.split_whitespace().collect();
                                         let mut i = 0;
                                         while i < parts.len() {
                                              match parts[i] {
                                                 "M" => {
                                                     if i+1 < parts.len() {
                                                         let coords: Vec<&str> = parts[i+1].split(',').collect();
                                                         if coords.len() == 2
                                                             && let (Ok(x), Ok(y)) = (coords[0].parse::<f32>(), coords[1].parse::<f32>())
                                                             && x.is_finite() && y.is_finite()
                                                         {
                                                             builder.move_to(bounds.origin + point(px(x), px(y)));
                                                         }
                                                         i += 2;
                                                     } else { i += 1; }
                                                 },
                                                 "L" => {
                                                     if i+1 < parts.len() {
                                                         let coords: Vec<&str> = parts[i+1].split(',').collect();
                                                         if coords.len() == 2
                                                             && let (Ok(x), Ok(y)) = (coords[0].parse::<f32>(), coords[1].parse::<f32>())
                                                             && x.is_finite() && y.is_finite()
                                                         {
                                                             builder.line_to(bounds.origin + point(px(x), px(y)));
                                                         }
                                                         i += 2;
                                                     } else { i += 1; }
                                                 },
                                                 _ => i += 1,
                                             }
                                         }
                                         // Don't close grid lines
                                         if let Ok(path) = builder.build() {
                                            window.paint_path(path, rgba(0x00000033));
                                         }
                                    }
                                }
                            )
                        )
                        // Render city markers (kept as overlay divs for easy text handling)
                        .children(render_cities(
                            current_projection,
                            map_width,
                            map_height,
                            center_x,
                            center_y,
                            rotation_lon,
                            rotation_lat,
                        )),
                ),
        )
        // Rotation controls
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Rotation Controls:"),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .items_center()
                                .child(div().text_sm().child("Longitude:"))
                                .child(
                                    div()
                                        .id("lon-minus")
                                        .px_3()
                                        .py_1()
                                        .bg(rgb(0xe8e8e8))
                                        .rounded_md()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(rgb(0xd0d0d0)))
                                        .child("-30°")
                                        .on_click(cx.listener(|this, _, _window, _cx| {
                                            this.geo_rotation_lon = (this.geo_rotation_lon - 30.0).rem_euclid(360.0);
                                        })),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .w(px(60.0))
                                        .text_center()
                                        .child(format!("{:.0}°", rotation_lon)),
                                )
                                .child(
                                    div()
                                        .id("lon-plus")
                                        .px_3()
                                        .py_1()
                                        .bg(rgb(0xe8e8e8))
                                        .rounded_md()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(rgb(0xd0d0d0)))
                                        .child("+30°")
                                        .on_click(cx.listener(|this, _, _window, _cx| {
                                            this.geo_rotation_lon = (this.geo_rotation_lon + 30.0).rem_euclid(360.0);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .items_center()
                                .child(div().text_sm().child("Latitude:"))
                                .child(
                                    div()
                                        .id("lat-minus")
                                        .px_3()
                                        .py_1()
                                        .bg(rgb(0xe8e8e8))
                                        .rounded_md()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(rgb(0xd0d0d0)))
                                        .child("-15°")
                                        .on_click(cx.listener(|this, _, _window, _cx| {
                                            this.geo_rotation_lat = (this.geo_rotation_lat - 15.0).max(-60.0);
                                        })),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .w(px(60.0))
                                        .text_center()
                                        .child(format!("{:.0}°", rotation_lat)),
                                )
                                .child(
                                    div()
                                        .id("lat-plus")
                                        .px_3()
                                        .py_1()
                                        .bg(rgb(0xe8e8e8))
                                        .rounded_md()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(rgb(0xd0d0d0)))
                                        .child("+15°")
                                        .on_click(cx.listener(|this, _, _window, _cx| {
                                            this.geo_rotation_lat = (this.geo_rotation_lat + 15.0).min(60.0);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .id("reset-rotation")
                                .px_3()
                                .py_1()
                                .bg(rgb(0x007acc))
                                .text_color(rgb(0xffffff))
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(0x005a9e)))
                                .child("Reset")
                                .on_click(cx.listener(|this, _, _window, _cx| {
                                    this.geo_rotation_lon = 0.0;
                                    this.geo_rotation_lat = 0.0;
                                })),
                        ),
                ),
        )
        // Legend
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .mt_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Legend:"),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().w(px(16.0)).h(px(2.0)).bg(rgba(0x00000033))) // Updated color to match
                                .child(div().text_sm().text_color(rgb(0x666666)).child("Graticule (30° grid)")),
                        )
                         .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().w(px(16.0)).h(px(16.0)).bg(rgb(0xd6e4ff)).border_1().border_color(rgb(0x3399ff)))
                                .child(div().text_sm().text_color(rgb(0x666666)).child("Continents")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().w(px(8.0)).h(px(8.0)).rounded_full().bg(rgb(0xd62728)))
                                .child(div().text_sm().text_color(rgb(0x666666)).child("Cities")),
                        ),
                ),
        )
        // Projection descriptions (kept)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_4()
                .bg(rgb(0xf5f5f5))
                .rounded_lg()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Projection Properties:"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x333333))
                        .child(projection_description(current_projection)),
                ),
        )
}

/// Project a point using the selected projection type
fn project_point(
    lon: f64,
    lat: f64,
    proj_type: GeoProjectionType,
    _map_width: f64,
    map_height: f64,
    center_x: f64,
    center_y: f64,
    rotation_lon: f64,
    rotation_lat: f64,
) -> Option<(f64, f64)> {
    // Check if point is visible (especially for azimuthal projections)
    // For Orthographic/Stereographic, we need to check visibility relative to the rotation center.
    // Since we are now using the projection's internal rotation, we can use the projection's capabilities
    // if it exposed visibility checking. However, d3rs `project` just returns coords.
    //
    // For Orthographic, the clipping logic in `render_cities` handles the "behind the globe" check
    // based on distance from center, which works reasonably well for centered globes.
    //
    // For now, we delegate visibility filtering to the caller or accept that points might wrap around.

    let scale = match proj_type {
        GeoProjectionType::Mercator => map_height / 3.0,
        GeoProjectionType::Equirectangular => map_height / 2.0,
        GeoProjectionType::Orthographic => map_height / 2.5,
        GeoProjectionType::Stereographic => map_height / 4.0,
        GeoProjectionType::ConicEqualArea => map_height / 4.5,
    };

    let (x, y) = match proj_type {
        GeoProjectionType::Mercator => {
            let proj = Mercator::new()
                .scale(scale)
                .translate(center_x, center_y)
                .rotate(rotation_lon, rotation_lat, 0.0);
            proj.project(lon, lat)
        }
        GeoProjectionType::Equirectangular => {
            let proj = Equirectangular::new()
                .scale(scale)
                .translate(center_x, center_y)
                .rotate(rotation_lon, rotation_lat, 0.0);
            proj.project(lon, lat)
        }
        GeoProjectionType::Orthographic => {
            let proj = Orthographic::new()
                .scale(scale)
                .translate(center_x, center_y)
                .rotate(rotation_lon, rotation_lat, 0.0);
            proj.project(lon, lat)
        }
        GeoProjectionType::Stereographic => {
            let proj = Stereographic::new()
                .scale(scale)
                .translate(center_x, center_y)
                .rotate(rotation_lon, rotation_lat, 0.0);
            proj.project(lon, lat)
        }
        GeoProjectionType::ConicEqualArea => {
            let proj = ConicEqualArea::new()
                .scale(scale)
                .translate(center_x, center_y + 50.0)
                .center(0.0, 30.0)
                .rotate(rotation_lon, rotation_lat, 0.0);
            proj.project(lon, lat)
        }
    };

    // Check bounds
    if x.is_finite() && y.is_finite() {
        // Relax strict bounds check to allow points slightly off-canvas (clipping handles it)
        Some((x, y))
    } else {
        None
    }
}

/// Render city markers
fn render_cities(
    proj_type: GeoProjectionType,
    map_width: f64,
    map_height: f64,
    center_x: f64,
    center_y: f64,
    rotation_lon: f64,
    rotation_lat: f64,
) -> Vec<Div> {
    let mut elements = Vec::new();

    for &(name, lon, lat) in CITIES {
        if let Some((x, y)) = project_point(
            lon,
            lat,
            proj_type,
            map_width,
            map_height,
            center_x,
            center_y,
            rotation_lon,
            rotation_lat,
        ) {
            // Basic visibility check for Orthographic (hide points behind globe)
            if matches!(proj_type, GeoProjectionType::Orthographic) {
                // If distance from center > radius (approx), hide
                let dx = x - center_x;
                let dy = y - center_y;
                let r = map_height / 2.5;
                if dx * dx + dy * dy > r * r + 1.0 {
                    // tolerance
                    continue;
                }
            }

            // City dot
            elements.push(
                div()
                    .absolute()
                    .left(px(x as f32 - 4.0))
                    .top(px(y as f32 - 4.0))
                    .w(px(8.0))
                    .h(px(8.0))
                    .rounded_full()
                    .bg(rgb(0xd62728))
                    .border_1()
                    .border_color(rgb(0xffffff)),
            );
            // City label
            elements.push(
                div()
                    .absolute()
                    .left(px(x as f32 + 6.0))
                    .top(px(y as f32 - 6.0))
                    .text_xs()
                    .text_color(rgb(0x333333))
                    .bg(rgba(0xffffffcc))
                    .px_1()
                    .rounded(px(2.0))
                    .child(name),
            );
        }
    }

    elements
}

/// Get description for a projection
fn projection_description(proj_type: GeoProjectionType) -> &'static str {
    match proj_type {
        GeoProjectionType::Mercator => {
            "Mercator: A conformal cylindrical projection that preserves angles and shapes locally. \
             Used for navigation and web maps. Distorts size near the poles."
        }
        GeoProjectionType::Equirectangular => {
            "Equirectangular (Plate Carrée): The simplest projection that maps longitude and latitude \
             directly to x and y. Preserves neither area nor shape, but is easy to compute."
        }
        GeoProjectionType::Orthographic => {
            "Orthographic: Shows the Earth as seen from space. An azimuthal projection that can only \
             display one hemisphere at a time. Useful for visualizing the globe."
        }
        GeoProjectionType::Stereographic => {
            "Stereographic: A conformal azimuthal projection. Preserves angles and local shapes. \
             Used in crystallography and complex analysis."
        }
        GeoProjectionType::ConicEqualArea => {
            "Conic Equal-Area (Albers): An equal-area projection using two standard parallels. \
             Excellent for regions with large east-west extent like the United States."
        }
    }
}
