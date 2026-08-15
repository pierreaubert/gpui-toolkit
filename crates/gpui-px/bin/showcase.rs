//! gpui-px Showcase - Demonstrates all chart types with the Plotly Express-style API.
//!
//! This showcase demonstrates the high-level gpui-px charting API built on top of d3rs.
//! Navigate through sections using the sidebar to see examples of each chart type.
//!
//! Build/serve the wasm version via `just wasm-serve-px` (Trunk, port 8082).
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};

#[path = "showcase/chart_section.rs"]
mod chart_section;
#[path = "showcase/color_scale_type.rs"]
mod color_scale_type;
#[path = "showcase/generate.rs"]
mod generate;
#[path = "showcase/showcase_app.rs"]
mod showcase_app;

use showcase_app::ShowcaseApp;

fn run_app() {
    MiniApp::run(
        MiniAppConfig::new("gpui-px Showcase")
            .size(1200.0, 800.0)
            .with_theme(true)
            .scrollable(false),
        |cx| cx.new(ShowcaseApp::new),
    );
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_app();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_miniapp::web_init();
    run_app();
}
