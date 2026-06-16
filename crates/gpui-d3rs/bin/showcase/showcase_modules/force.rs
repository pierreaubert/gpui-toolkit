use crate::ShowcaseApp;
use crate::demo_section::DemoSection;
use d3rs::gpu2d::Chart2DElement;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Start a background animation loop that advances the force simulation and
/// caches node positions. The loop stops automatically when the user leaves the
/// force demo section.
pub fn ensure_force_animation(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) {
    if app.force_running {
        return;
    }
    app.force_running = true;

    // Seed the position cache so the first frame is not blank.
    app.tick_force_simulation();

    cx.spawn(async move |this: WeakEntity<ShowcaseApp>, cx| {
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            let still_force = this
                .update(cx, |app, _cx| {
                    if app.current_section != DemoSection::Force {
                        app.force_running = false;
                        return false;
                    }
                    app.tick_force_simulation();
                    true
                })
                .unwrap_or(false);
            if !still_force {
                break;
            }
            this.update(cx, |_, cx| cx.notify()).ok();
        }
    })
    .detach();
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    ensure_force_animation(app, cx);

    let ui_theme = cx.theme();
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
