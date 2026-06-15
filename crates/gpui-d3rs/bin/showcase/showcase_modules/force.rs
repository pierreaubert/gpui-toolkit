use crate::ShowcaseApp;
use d3rs::gpu2d::Chart2DElement;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;
use std::cell::RefCell;
use std::rc::Rc;

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
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

    // Share the latest node positions via a reusable Rc<RefCell<Vec>> so the
    // render closure does not need to copy them every frame.
    {
        let mut positions = app.force_node_positions.borrow_mut();
        positions.clear();
        positions.extend(app.force_simulation.nodes.iter().map(|n| {
            let n = n.borrow();
            (n.x as f32, n.y as f32)
        }));
    }

    let node_data: Rc<RefCell<Vec<(f32, f32)>>> = app.force_node_positions.clone();

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
                .child("Nodes repel each other and are attracted to the center."),
        )
        .child({
            let width = app.content_width;
            let height = (width * 0.75).min(app.content_height * 0.8);
            div()
                .w(px(width))
                .h(px(height))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .overflow_hidden()
                .child(
                    Chart2DElement::new(move |renderer, _bounds| {
                        for (x, y) in node_data.borrow().iter() {
                            renderer.draw_circle(*x, *y, 5.0, [1.0, 0.2, 0.2, 1.0]);
                        }
                    })
                    .background_color([0.94, 0.94, 0.94, 1.0]),
                )
        })
}
