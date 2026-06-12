//! Breadcrumbs Debug Example
//!
//! Demonstrates the Breadcrumbs component:
//! - Different separator styles
//! - With icons

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::theme::ThemeExt;
use gpui_ui_kit::*;

pub struct BreadcrumbsDebug;

impl Render for BreadcrumbsDebug {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("breadcrumbs-debug-root")
            .size_full()
            .bg(theme.background)
            .text_color(theme.text_primary)
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(Heading::h1("Breadcrumbs Debug"))
            // Slash separator
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Slash Separator (Default)").weight(TextWeight::Bold))
                    .child(
                        Breadcrumbs::new()
                            .items(vec![
                                BreadcrumbItem::new("home", "Home"),
                                BreadcrumbItem::new("library", "Library"),
                                BreadcrumbItem::new("album", "Beethoven - Complete Sonatas"),
                            ])
                            .separator(BreadcrumbSeparator::Slash),
                    ),
            )
            // Chevron separator
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Chevron Separator").weight(TextWeight::Bold))
                    .child(
                        Breadcrumbs::new()
                            .items(vec![
                                BreadcrumbItem::new("settings", "Settings"),
                                BreadcrumbItem::new("audio", "Audio"),
                                BreadcrumbItem::new("plugins", "Plugins"),
                                BreadcrumbItem::new("eq", "Parametric EQ"),
                            ])
                            .separator(BreadcrumbSeparator::Chevron),
                    ),
            )
            // Dot separator
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Text::new("Dot Separator").weight(TextWeight::Bold))
                    .child(
                        Breadcrumbs::new()
                            .items(vec![
                                BreadcrumbItem::new("root", "Root"),
                                BreadcrumbItem::new("child", "Child"),
                            ])
                            .separator(BreadcrumbSeparator::Dot),
                    ),
            )
    }
}

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Breadcrumbs Debug")
            .size(600.0, 500.0)
            .scrollable(true)
            .with_theme(true),
        |cx| cx.new(|_cx| BreadcrumbsDebug),
    );
}
