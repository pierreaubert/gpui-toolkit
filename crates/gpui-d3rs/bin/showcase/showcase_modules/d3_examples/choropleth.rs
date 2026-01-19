use super::super::world_data::get_world_data;
use crate::ShowcaseApp;
use d3rs::geo::{GeoJsonGeometry, GeoPath, projection::Mercator};
use gpui::prelude::*;
use gpui::*;

pub fn render(app: &ShowcaseApp, _cx: &mut Context<ShowcaseApp>) -> Div {
    let width = 800.0;
    let height = 500.0;

    // Get world data
    let geometry = get_world_data(app.use_large_data);

    // Split MultiPolygon into individual polygons to color them
    let features: Vec<GeoJsonGeometry> = match geometry {
        GeoJsonGeometry::MultiPolygon(polys) => polys
            .iter()
            .map(|p| GeoJsonGeometry::Polygon(p.clone()))
            .collect(),
        _ => vec![],
    };

    let colors = [
        rgb(0xf7fbff),
        rgb(0xdeebf7),
        rgb(0xc6dbef),
        rgb(0x9ecae1),
        rgb(0x6baed6),
        rgb(0x4292c6),
        rgb(0x2171b5),
        rgb(0x08519c),
        rgb(0x08306b),
    ];

    // Generate paths
    let mut feature_paths = Vec::new();
    for (i, geo) in features.iter().enumerate() {
        let proj = Mercator::new()
            .scale(120.0)
            .translate(width as f64 / 2.0, height as f64 / 2.0 + 50.0);
        let path = GeoPath::new(proj);
        let d = path.render(geo);
        feature_paths.push((d, i));
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
                .child("Choropleth (Mercator)"),
        )
        .child(
            div()
                .w(px(width))
                .h(px(height))
                .bg(rgb(0xaadaff)) // Ocean color
                .relative()
                .child(canvas(
                    move |bounds, _, _| {
                        let parsed: Vec<_> = feature_paths
                            .iter()
                            .map(|(d, i)| (super::path_utils::parse_svg_path(d, bounds), *i))
                            .collect();
                        parsed
                    },
                    move |_bounds, paths, window, _| {
                        for (path_opt, i) in paths {
                            if let Some(path) = path_opt {
                                // Mock value to choose color
                                let val_idx = (i * 3 + 1) % colors.len();
                                let color = colors[val_idx];
                                window.paint_path(path, color);

                                // Stroke? paint_path fills.
                                // To stroke, we need PathBuilder::stroke logic or another path.
                                // Ignoring stroke for filled choropleth demo.
                            }
                        }
                    },
                )),
        )
}
