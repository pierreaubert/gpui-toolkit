//! Full D3rs Spinorama viewer with live speaker measurements, polar and 3D views.
//!
//! Build/serve the wasm version via `just wasm-serve-spinorama` (Trunk, port 8084).
#![cfg_attr(target_family = "wasm", no_main)]

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};

mod api;
mod app;
#[cfg(all(target_os = "macos", feature = "capture-bench"))]
mod capture_bench;
#[cfg(all(target_os = "macos", feature = "capture-bench"))]
mod headless;
#[cfg(all(target_os = "macos", feature = "capture-bench"))]
mod render_images;
mod render;
mod spinorama_section;
mod types;
mod utils;

use app::SpinoramaApp;

fn run_app() {
    let config = MiniAppConfig::new("Spinorama Showcase")
        .size(1200.0, 800.0)
        .with_theme(true)
        .scrollable(false);
    #[cfg(target_family = "wasm")]
    let config = config.initial_theme(gpui_miniapp::web_initial_theme());
    #[cfg(target_family = "wasm")]
    let config = config.initial_design(gpui_miniapp::web_initial_design_style());
    MiniApp::run(config, |cx| cx.new(SpinoramaApp::new));
}

#[cfg(not(target_family = "wasm"))]
fn run_render_images(args: &[String]) {
    #[cfg(all(target_os = "macos", feature = "capture-bench"))]
    {
        if let Err(error) = render_images::run(args) {
            eprintln!("render-images failed: {error}");
            std::process::exit(1);
        }
    }
    #[cfg(not(all(target_os = "macos", feature = "capture-bench")))]
    {
        let _ = args;
        eprintln!("--render-images requires macOS and `--features capture-bench`");
        std::process::exit(1);
    }
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    // The `spinorama-render-images` binary shares this entry point and
    // defaults to the batch renderer without an explicit mode flag.
    let invoked_as_renderer = std::env::args()
        .next()
        .is_some_and(|arg0| arg0.contains("spinorama-render-images"));
    if invoked_as_renderer {
        let rest: Vec<String> = std::env::args().skip(1).collect();
        let rest = if rest.first().map(String::as_str) == Some("--render-images") {
            rest[1..].to_vec()
        } else {
            rest
        };
        run_render_images(&rest);
        return;
    }
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--visual-manifest" => {
                if args.any(|arg| arg == "--json") {
                    println!("{}", spinorama_section::visual_manifest_json());
                } else {
                    println!("gpui-spinorama visual capture manifest");
                    for section in spinorama_section::SpinoramaSection::all() {
                        println!("- {}", section.label());
                    }
                }
                return;
            }
            "--capture-bench" => {
                #[cfg(all(target_os = "macos", feature = "capture-bench"))]
                {
                    let rest: Vec<String> = args.collect();
                    if let Err(error) = capture_bench::run(&rest) {
                        eprintln!("capture-bench failed: {error}");
                        std::process::exit(1);
                    }
                }
                #[cfg(not(all(target_os = "macos", feature = "capture-bench")))]
                {
                    eprintln!("--capture-bench requires macOS and `--features capture-bench`");
                    std::process::exit(1);
                }
                return;
            }
            "--render-images" => {
                let rest: Vec<String> = args.collect();
                run_render_images(&rest);
                return;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: spinorama-showcase [--visual-manifest [--json]] [--capture-bench <cea2034|contour> <fixture.json> <out.png> [iterations]] [--render-images <private_root> <out_root> [--limit N] [--filter SUBSTR]]"
                );
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
