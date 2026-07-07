//! DragList Debug Example
//!
//! Demonstrates the DragList component:
//! - Vertical and horizontal orientations
//! - With drag handles

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct DragListDebug {
    vertical_items: Vec<SharedString>,
    horizontal_items: Vec<SharedString>,
    entity: Entity<Self>,
}

impl DragListDebug {
    fn reorder(items: &mut Vec<SharedString>, from: usize, to: usize) {
        if from >= items.len() || to >= items.len() || from == to {
            return;
        }

        let item = items.remove(from);
        items.insert(to, item);
    }

    fn plugin_label(id: &SharedString) -> &'static str {
        match id.as_ref() {
            "eq" => "Parametric EQ",
            "comp" => "Compressor",
            "upmix" => "Upmixer",
            "limiter" => "Limiter",
            _ => "Unknown",
        }
    }
}

impl Render for DragListDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

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
                            .child(
                                DragList::new(
                                    "drag-vert",
                                    self.vertical_items
                                        .iter()
                                        .enumerate()
                                        .map(|(index, id)| {
                                            DragItem::new(
                                                id.clone(),
                                                div().p_2().child(Text::new(format!(
                                                    "{}. {}",
                                                    index + 1,
                                                    Self::plugin_label(id)
                                                ))),
                                            )
                                        })
                                        .collect(),
                                )
                                .on_reorder({
                                    let entity = entity.clone();
                                    move |from, to, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            Self::reorder(&mut this.vertical_items, from, to);
                                        });
                                    }
                                }),
                            ),
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
                                    self.horizontal_items
                                        .iter()
                                        .map(|id| {
                                            DragItem::new(
                                                id.clone(),
                                                div().p_2().child(Text::new(id.to_string())),
                                            )
                                        })
                                        .collect(),
                                )
                                .orientation(DragListOrientation::Horizontal)
                                .on_reorder({
                                    move |from, to, _window, cx| {
                                        entity.update(cx, |this, _cx| {
                                            Self::reorder(&mut this.horizontal_items, from, to);
                                        });
                                    }
                                }),
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
        |cx| {
            cx.new(|cx| DragListDebug {
                vertical_items: vec!["eq".into(), "comp".into(), "upmix".into(), "limiter".into()],
                horizontal_items: vec!["A".into(), "B".into(), "C".into()],
                entity: cx.entity().clone(),
            })
        },
    );
}
