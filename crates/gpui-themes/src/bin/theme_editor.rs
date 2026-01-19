//! Theme Editor - A MiniApp-based theme editing application
//!
//! This binary provides a visual theme editor for GPUI applications.
//! It allows editing theme colors with live preview and exporting to JSON or Rust.

use gpui::AppContext;
use gpui_themes::ThemeEditor;
use gpui_ui_kit::{MiniApp, MiniAppConfig};

fn main() {
    MiniApp::run(
        MiniAppConfig::new("GPUI Theme Editor")
            .size(1200.0, 800.0)
            .scrollable(false)
            .with_theme(false), // Editor manages its own theme
        |cx| cx.new(ThemeEditor::new),
    );
}
