//! Chord Diagram Demo
//!
//! Visualizes a chord diagram (flow matrix).

use d3rs::chord::{ChordLayout, RibbonGenerator};
use gpui::prelude::*;
use gpui::*;
use std::f64::consts::PI;

struct ChordDemo {
    width: f64,
    height: f64,
}

impl ChordDemo {
    fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            width: 800.0,
            height: 600.0,
        }
    }
}

impl Render for ChordDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let matrix = vec![
            vec![11975.0, 5871.0, 8916.0, 2868.0],
            vec![1951.0, 10048.0, 2060.0, 6171.0],
            vec![8010.0, 16145.0, 8090.0, 8045.0],
            vec![1013.0, 990.0, 940.0, 6907.0],
        ];

        let layout = ChordLayout::new().pad_angle(0.05);
        let chords = layout.compute(&matrix);

        // Ribbon generator
        let outer_radius = 200.0;
        let inner_radius = 180.0;

        let center_x = self.width / 2.0;
        let center_y = self.height / 2.0;

        let ribbon = RibbonGenerator::new(inner_radius).center(center_x, center_y);

        let colors = vec![rgb(0x000000), rgb(0xffdd89), rgb(0x957244), rgb(0xf26223)];

        let mut elements = Vec::new();

        // Render Groups (Arcs)
        use d3rs::shape::arc::{Arc, ArcDatum};
        let arc_gen = Arc::new().center(center_x, center_y);

        for group in &chords.groups {
            let datum = ArcDatum::new()
                .inner_radius(inner_radius)
                .outer_radius(outer_radius)
                .start_angle(group.start_angle - PI / 2.0)
                .end_angle(group.end_angle - PI / 2.0);

            let path = arc_gen.generate(&datum);

            elements.push(
                div().absolute().size_full().child(
                    svg()
                        .path(path.to_svg_string())
                        .text_color(colors[group.index % colors.len()])
                        .size_full(),
                ),
            );
        }

        // Render Chords (Ribbons)
        for chord in &chords.chords {
            let path_d = ribbon.generate(chord);

            elements.push(
                div().absolute().size_full().child(
                    svg()
                        .path(path_d)
                        .text_color(colors[chord.target.index % colors.len()])
                        .opacity(0.67)
                        .size_full(),
                ),
            );
        }

        div()
            .size_full()
            .bg(rgb(0xffffff))
            .child(div().relative().size_full().children(elements))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(800.0), px(600.0)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| ChordDemo::new(cx)),
        )
        .unwrap();

        cx.activate(true);
    });
}
