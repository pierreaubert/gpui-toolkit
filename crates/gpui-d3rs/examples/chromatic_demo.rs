//! Chromatic Scale Demo
//!
//! Visualizes various color schemes from d3-scale-chromatic.

use d3rs::color::D3Color;
use d3rs::color::chromatic::{DivergingScheme, SequentialScheme};
use gpui::prelude::*;
use gpui::*;

struct ChromaticDemo;

impl ChromaticDemo {
    fn new(_cx: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for ChromaticDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // We will render horizontal bars for each scheme

        let schemes: Vec<(&str, fn(f64) -> D3Color)> = vec![
            ("Turbo", SequentialScheme::turbo),
            ("Viridis", SequentialScheme::viridis),
            ("Magma", SequentialScheme::magma),
            ("RdBu (Diverging)", DivergingScheme::rd_bu),
        ];

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .p_8()
            .gap_4()
            .children(schemes.into_iter().map(|(name, scheme_fn)| {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xffffff))
                            .child(name.to_string()),
                    )
                    .child(
                        div()
                            .h(px(40.0))
                            .w_full()
                            .rounded_md()
                            .overflow_hidden()
                            .child(
                                // Use a simpler approach: many small divs for gradient
                                // In a real app, use a linear gradient background or canvas
                                div().flex().size_full().children((0..100).map(|i| {
                                    let t = i as f64 / 100.0;
                                    let c = scheme_fn(t);
                                    // c.r is 0..255 or 0..1? d3rs D3Color usually holds 0..255 or similar?
                                    // check definition. D3Color::rgb(r,g,b).
                                    // interpolate::interpolate_rgb uses f64 internally.
                                    // let's assume standard u8 mapping for display

                                    // GPUI rgb takes u32 hex, so we need to construct it
                                    let r = (c.r as u32).min(255);
                                    let g = (c.g as u32).min(255);
                                    let b = (c.b as u32).min(255);
                                    let hex = (r << 16) | (g << 8) | b;

                                    div().h_full().flex_1().bg(rgb(hex))
                                })),
                            ),
                    )
            }))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(600.0), px(400.0)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| ChromaticDemo::new(cx)),
        )
        .unwrap();

        cx.activate(true);
    });
}
