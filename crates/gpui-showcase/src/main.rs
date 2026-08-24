//! UI Kit Showcase
//!
//! A thin example wrapper around `gpui_showcase::Showcase`.
//! Use View > Theme menu or Cmd+T to toggle between light/dark themes.
//! Use Language menu to switch between languages.
#![cfg_attr(target_family = "wasm", no_main)]

#[cfg(not(target_family = "wasm"))]
use gpui_showcase::{showcase_release_artifact_report, showcase_visual_capture_manifest};

#[cfg(not(target_family = "wasm"))]
fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--release-artifacts" => {
                println!("{}", showcase_release_artifact_report().to_markdown());
                return;
            }
            "--visual-manifest" => {
                let manifest = showcase_visual_capture_manifest();
                if args.any(|arg| arg == "--json") {
                    println!("{}", manifest.to_json());
                } else {
                    println!("{}", manifest.to_markdown_table());
                }
                return;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: gpui-showcase [--window-min-size WIDTHxHEIGHT] [--release-artifacts | --visual-manifest [--json]]\n\nOptions:\n  --window-min-size WIDTHxHEIGHT  Set a native minimum window size (for example 400x400)"
                );
                return;
            }
            _ => {}
        }
    }

    gpui_showcase::run_showcase();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_miniapp::web_init();
    gpui_showcase::run_showcase();
}
