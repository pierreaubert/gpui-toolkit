//! DragList Debug Example
//!
//! Demonstrates the DragList component:
//! - Vertical and horizontal orientations
//! - With drag handles

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct DragListDebug;

impl Render for DragListDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("drag-list-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("DragList Debug"))
            // Vertical
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Vertical (Default)").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(DragList::new(
                                "drag-vert",
                                vec![
                                    DragItem::new(
                                        "eq",
                                        div().p_2().child(Text::new("1. Parametric EQ")),
                                    ),
                                    DragItem::new(
                                        "comp",
                                        div().p_2().child(Text::new("2. Compressor")),
                                    ),
                                    DragItem::new(
                                        "upmix",
                                        div().p_2().child(Text::new("3. Upmixer")),
                                    ),
                                    DragItem::new(
                                        "limiter",
                                        div().p_2().child(Text::new("4. Limiter")),
                                    ),
                                ],
                            )),
                    ),
            )
            // Horizontal
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Horizontal").weight(TextWeight::Bold))
                    .child(
                        div()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .p_4()
                            .child(
                                DragList::new(
                                    "drag-horiz",
                                    vec![
                                        DragItem::new("a", div().p_2().child(Text::new("A"))),
                                        DragItem::new("b", div().p_2().child(Text::new("B"))),
                                        DragItem::new("c", div().p_2().child(Text::new("C"))),
                                    ],
                                )
                                .orientation(DragListOrientation::Horizontal),
                            ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("DragList Debug")
            .size(600.0, 600.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| DragListDebug),
    );
}
