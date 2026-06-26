//! UI Kit Showcase
//!
//! A thin example wrapper around `gpui_ui_kit::Showcase`.
//! Use View > Theme menu or Cmd+T to toggle between light/dark themes.
//! Use Language menu to switch between languages.

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_ui_kit::Showcase;

fn main() {
    MiniApp::run(
        MiniAppConfig::new("UI Kit Showcase")
            .size(1200.0, 900.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(Showcase::new),
    );
}
