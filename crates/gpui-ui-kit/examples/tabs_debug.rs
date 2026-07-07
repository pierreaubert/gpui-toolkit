//! Tabs Debug Example
//!
//! Demonstrates the Tabs component:
//! - Underline, Enclosed, Pills, VerticalCard variants
//! - Tabs with icons and badges
//! - Disabled tabs

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Text;
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct TabsDebug {
    selected_index: usize,
    entity: Entity<Self>,
}

impl TabsDebug {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            selected_index: 0,
            entity: cx.entity().clone(),
        }
    }
}

impl Render for TabsDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

        div()
            .id("tabs-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .overflow_y_scroll()
            .child(Heading::h1("Tabs Debug"))
            // Underline
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Underline (Default)").weight(TextWeight::Bold))
                    .child(
                        Tabs::new("tabs-underline")
                            .tabs(vec![
                                TabItem::new("eq", "EQ"),
                                TabItem::new("comp", "Compressor"),
                                TabItem::new("upmixer", "Upmixer"),
                            ])
                            .selected_index(self.selected_index)
                            .variant(TabVariant::Underline)
                            .on_change({
                                let entity = entity.clone();
                                move |idx, _window, cx| {
                                    entity.update(cx, |this, _cx| this.selected_index = idx);
                                }
                            }),
                    ),
            )
            // Enclosed
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Enclosed").weight(TextWeight::Bold))
                    .child(
                        Tabs::new("tabs-enclosed")
                            .tabs(vec![
                                TabItem::new("lib", "Library"),
                                TabItem::new("queue", "Queue"),
                                TabItem::new("settings", "Settings"),
                            ])
                            .selected_index(0)
                            .variant(TabVariant::Enclosed),
                    ),
            )
            // Pills
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Pills").weight(TextWeight::Bold))
                    .child(
                        Tabs::new("tabs-pills")
                            .tabs(vec![
                                TabItem::new("all", "All"),
                                TabItem::new("flac", "FLAC"),
                                TabItem::new("mp3", "MP3"),
                                TabItem::new("aac", "AAC"),
                            ])
                            .selected_index(0)
                            .variant(TabVariant::Pills),
                    ),
            )
            // With badges and disabled
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("With Badges & Disabled").weight(TextWeight::Bold))
                    .child(
                        Tabs::new("tabs-badges")
                            .tabs(vec![
                                TabItem::new("tracks", "Tracks").badge("142"),
                                TabItem::new("albums", "Albums").badge("23"),
                                TabItem::new("locked", "Premium").disabled(true),
                            ])
                            .selected_index(0),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Tabs Debug")
            .size(700.0, 700.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(TabsDebug::new),
    );
}
