//! Chromatic Scale Demo
//!
//! Visualizes various color schemes from d3-scale-chromatic.

use d3rs::color::chromatic::{DivergingScale, DivergingScheme, SequentialScale, SequentialScheme};
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

        let schemes: Vec<(&str, SequentialScale)> = vec![
            ("Turbo", SequentialScheme::turbo()),
            ("Viridis", SequentialScheme::viridis()),
            ("Magma", SequentialScheme::magma()),
        ];

        let diverging_schemes: Vec<(&str, DivergingScale)> =
            vec![("RdBu", DivergingScheme::rd_bu())];

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .p_8()
            .gap_4()
            .children(
                schemes
                    .into_iter()
                    .map(|(name, scale)| render_scale_row(name, scale)),
            )
            .children(
                diverging_schemes
                    .into_iter()
                    .map(|(name, scale)| render_diverging_scale_row(name, scale)),
            )
    }
}

fn render_scale_row(name: &str, scale: SequentialScale) -> impl IntoElement {
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
                .child(div().flex().size_full().children((0..100).map(|i| {
                    let t = i as f64 / 100.0;
                    let c = scale.get(t);
                    let r = (c.r * 255.0) as u32;
                    let g = (c.g * 255.0) as u32;
                    let b = (c.b * 255.0) as u32;
                    let hex = (r << 16) | (g << 8) | b;

                    div().h_full().flex_1().bg(rgb(hex))
                }))),
        )
}

fn render_diverging_scale_row(name: &str, scale: DivergingScale) -> impl IntoElement {
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
                .child(div().flex().size_full().children((0..100).map(|i| {
                    let t = i as f64 / 100.0;
                    let c = scale.get(t);
                    let r = (c.r * 255.0) as u32;
                    let g = (c.g * 255.0) as u32;
                    let b = (c.b * 255.0) as u32;
                    let hex = (r << 16) | (g << 8) | b;

                    div().h_full().flex_1().bg(rgb(hex))
                }))),
        )
}

fn main() {
    let platform = gpui_miniapp::current_platform().expect("failed to initialize GPUI platform");
    Application::with_platform(platform).run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(600.0), px(400.0)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(ChromaticDemo::new),
        )
        .unwrap();

        cx.activate(true);
    });
}
