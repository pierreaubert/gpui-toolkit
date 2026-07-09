//! UI Kit Showcase
//!
//! A thin example wrapper around `gpui_showcase::Showcase`.
//! Use View > Theme menu or Cmd+T to toggle between light/dark themes.
//! Use Language menu to switch between languages.

use gpui::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_showcase::{Showcase, showcase_release_artifact_report, showcase_visual_capture_manifest};

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
                println!("Usage: gpui-showcase [--release-artifacts | --visual-manifest [--json]]");
                return;
            }
            _ => {}
        }
    }

    MiniApp::run(
        MiniAppConfig::new("UI Kit Showcase")
            .size(1200.0, 900.0)
            .scrollable(true)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(Showcase::new),
    );
}
