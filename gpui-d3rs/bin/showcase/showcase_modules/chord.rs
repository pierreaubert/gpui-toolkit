use crate::ShowcaseApp;
use d3rs::chord::{ChordLayout, RibbonGenerator};
use gpui::*;
use std::f64::consts::PI;

pub fn render(_app: &ShowcaseApp, _cx: &mut Context<ShowcaseApp>) -> Div {
    let matrix = vec![
        vec![11975.0, 5871.0, 8916.0, 2868.0],
        vec![1951.0, 10048.0, 2060.0, 6171.0],
        vec![8010.0, 16145.0, 8090.0, 8045.0],
        vec![1013.0, 990.0, 940.0, 6907.0],
    ];

    let layout = ChordLayout::new().pad_angle(0.05);
    let chords = layout.compute(&matrix);

    let outer_radius = 200.0;
    let inner_radius = 180.0;

    let width = 600.0;
    let height = 600.0;

    let ribbon = RibbonGenerator::new(inner_radius);

    let colors = vec![rgb(0x000000), rgb(0xffdd89), rgb(0x957244), rgb(0xf26223)];

    // Arcs
    use d3rs::shape::arc::{Arc, ArcDatum};
    let arc_gen = Arc::new();

    // Canvas rendering
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .child("Chord Diagram (Canvas)"),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(rgb(0xffffff))
                .border_1()
                .border_color(rgb(0xcccccc))
                .child(
                    canvas(
                        |_bounds, _window, _cx| {},
                        move |bounds, _state, window, _cx| {
                            let center = bounds.center();
                            let paint_d3_path =
                                |d3_path: d3rs::shape::path::Path,
                                 color: Rgba,
                                 opacity: f32,
                                 window: &mut gpui::Window| {
                                    let points = d3_path.flatten(0.1);
                                    if points.is_empty() {
                                        return;
                                    }

                                    let mut builder = gpui::PathBuilder::fill();

                                    let start =
                                        point(px(points[0].x as f32), px(points[0].y as f32))
                                            + center;
                                    builder.move_to(start);
                                    for pt in &points[1..] {
                                        let p = point(px(pt.x as f32), px(pt.y as f32)) + center;
                                        builder.line_to(p);
                                    }
                                    builder.close();

                                    match builder.build() {
                                        Ok(path) => {
                                            let final_color = gpui::Rgba {
                                                r: color.r,
                                                g: color.g,
                                                b: color.b,
                                                a: opacity,
                                            };
                                            window.paint_path(path, final_color);
                                        }
                                        Err(e) => println!("ERROR: Failed to build path: {:?}", e),
                                    }
                                };

                            // Arcs
                            for group in &chords.groups {
                                let datum = ArcDatum::new()
                                    .inner_radius(inner_radius)
                                    .outer_radius(outer_radius)
                                    .start_angle(group.start_angle - PI / 2.0)
                                    .end_angle(group.end_angle - PI / 2.0);

                                let d3_path = arc_gen.generate(&datum);
                                let color = colors[group.index % colors.len()];
                                paint_d3_path(d3_path, color, 1.0, window);
                            }

                            // Ribbons
                            for chord in &chords.chords {
                                let d3_path = ribbon.generate_path(chord);
                                let color = colors[chord.target.index % colors.len()];
                                paint_d3_path(d3_path, color, 0.67, window);
                            }
                        },
                    )
                    .size_full(),
                ),
        )
}
