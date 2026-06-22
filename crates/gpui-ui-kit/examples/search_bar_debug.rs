//! Search Bar Debug Example
//!
//! Demonstrates the SearchBar component:
//! - Different sizes
//! - Placeholder text
//! - Pre-filled value

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct SearchBarDebug {
    filled_value: SharedString,
    entity: Entity<Self>,
}

impl Render for SearchBarDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();

        div()
            .id("search-bar-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Search Bar Debug"))
            // Default size
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Default Size (Md)").weight(TextWeight::Bold))
                    .child(
                        div()
                            .w(px(300.0))
                            .child(SearchBar::new("search-default").placeholder("Search...")),
                    ),
            )
            // Small size
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Small Size").weight(TextWeight::Bold))
                    .child(
                        div().w(px(250.0)).child(
                            SearchBar::new("search-small")
                                .placeholder("Quick search")
                                .size(SearchBarSize::Sm),
                        ),
                    ),
            )
            // Pre-filled
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Pre-filled Value").weight(TextWeight::Bold))
                    .child(
                        div().w(px(300.0)).child(
                            SearchBar::new("search-filled")
                                .value(self.filled_value.clone())
                                .on_change(move |query, _window, cx| {
                                    let query = SharedString::from(query.to_string());
                                    entity.update(cx, |this, _cx| {
                                        this.filled_value = query;
                                    });
                                }),
                        ),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Search Bar Debug")
            .size(500.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| {
            cx.new(|cx| SearchBarDebug {
                filled_value: "Beethoven".into(),
                entity: cx.entity().clone(),
            })
        },
    );
}
