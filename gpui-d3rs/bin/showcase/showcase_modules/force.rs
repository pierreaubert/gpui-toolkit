use crate::ShowcaseApp;
use d3rs::gpu2d::Chart2DElement;
use gpui::*;

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    if app.force_running {
        for _ in 0..5 {
            app.force_simulation.tick();
        }
        cx.notify();
    } else {
        // Start running if not already
        app.force_running = true;
        cx.notify();
    }

    // Extract node positions to pass to the closure
    let node_data: Vec<(f32, f32)> = app
        .force_simulation
        .nodes
        .iter()
        .map(|n| {
            let n = n.borrow();
            (n.x as f32, n.y as f32)
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .child("Force Directed Graph (GPU Accelerated)"),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x666666))
                .child("Nodes repel each other and are attracted to the center."),
        )
        .child(
            div()
                .w(px(800.0))
                .h(px(600.0))
                .bg(rgb(0xf0f0f0))
                .border_1()
                .border_color(rgb(0xcccccc))
                .overflow_hidden()
                .child(
                    Chart2DElement::new(move |renderer, _bounds| {
                        for (x, y) in &node_data {
                            renderer.draw_circle(*x, *y, 5.0, [1.0, 0.2, 0.2, 1.0]);
                        }
                    })
                    .background_color([0.94, 0.94, 0.94, 1.0]),
                ),
        )
}
