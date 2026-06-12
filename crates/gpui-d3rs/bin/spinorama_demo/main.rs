#![allow(
    clippy::too_many_arguments,
    reason = "spinorama chart helpers mirror plotting formulas and visual geometry"
)]

mod app;
mod render;
mod types;
mod utils;

use app::SpinoramaApp;
use gpui::AppContext as _;
use gpui_miniapp::{MiniApp, MiniAppConfig};

fn main() {
    MiniApp::run(
        MiniAppConfig::new("Spinorama Viewer")
            .size(1200.0, 800.0)
            .with_theme(true)
            .scrollable(false),
        |cx| cx.new(SpinoramaApp::new),
    );
}
