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
    let config = MiniAppConfig::new("gpui-px Showcase")
        .size(1200.0, 800.0)
        .with_theme(true)
        .scrollable(false);
    #[cfg(target_family = "wasm")]
    let config = config.initial_theme(gpui_miniapp::web_initial_theme());
    MiniApp::run(config, |cx| cx.new(ShowcaseApp::new));
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--visual-manifest" => {
                if args.any(|arg| arg == "--json") {
                    println!("{}", chart_section::visual_manifest_json());
                } else {
                    println!("gpui-px visual capture manifest");
                    for section in chart_section::ChartSection::all() {
                        println!("- {}", section.label());
                    }
                }
                return;
            }
            "--help" | "-h" => {
                println!("Usage: px-showcase [--visual-manifest [--json]]");
                return;
            }
            _ => {}
        }
    }
    run_app();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_miniapp::web_init();
    run_app();
}
