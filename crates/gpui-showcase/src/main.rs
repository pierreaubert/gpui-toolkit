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
            "--visual-capture" => {
                run_visual_capture(args.collect());
                return;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: gpui-showcase [--window-min-size WIDTHxHEIGHT] [--release-artifacts | --visual-manifest [--json] | --visual-capture --visual-output-root DIR [--visual-case ID ...] [--theme ID] [--design ID] [--viewport WxH]]\n\nOptions:\n  --window-min-size WIDTHxHEIGHT  Set a native minimum window size (for example 400x400)"
                );
                return;
            }
            _ => {}
        }
    }

    gpui_showcase::run_showcase();
}

#[cfg(all(not(target_family = "wasm"), feature = "visual-capture"))]
fn run_visual_capture(args: Vec<String>) {
    use gpui_showcase::{showcase_visual_capture_manifest, visual_capture};
    use std::path::PathBuf;

    let mut output_root: Option<PathBuf> = None;
    let mut cases: Vec<String> = Vec::new();
    let mut theme_id = "dark".to_string();
    let mut design_id = "neutral".to_string();
    let mut viewport_override: Option<(u32, u32)> = None;
    let mut rest = args.into_iter().peekable();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--visual-output-root" => {
                output_root = rest.next().map(PathBuf::from);
            }
            "--visual-case" => {
                if let Some(id) = rest.next() {
                    cases.push(id);
                }
            }
            "--theme" => {
                if let Some(id) = rest.next() {
                    theme_id = id;
                }
            }
            "--design" => {
                if let Some(id) = rest.next() {
                    design_id = id;
                }
            }
            "--viewport" => {
                if let Some(spec) = rest.next() {
                    let (w, h) = spec.split_once(['x', 'X']).unwrap_or(("", ""));
                    match (w.parse::<u32>(), h.parse::<u32>()) {
                        (Ok(w), Ok(h)) if w > 0 && h > 0 => viewport_override = Some((w, h)),
                        _ => {
                            eprintln!("error: --viewport expects WIDTHxHEIGHT with positive integers");
                            std::process::exit(2);
                        }
                    }
                }
            }
            other => {
                eprintln!("error: unknown --visual-capture flag {other:?}");
                std::process::exit(2);
            }
        }
    }
    let Some(output_root) = output_root else {
        eprintln!("error: --visual-capture requires --visual-output-root DIR");
        std::process::exit(2);
    };
    let manifest = showcase_visual_capture_manifest();
    let selected: Vec<_> = manifest
        .captures
        .iter()
        .filter(|capture| cases.is_empty() || cases.iter().any(|id| id == &capture.id))
        .cloned()
        .collect();
    if selected.is_empty() {
        eprintln!("error: no manifest captures match the requested --visual-case ids");
        std::process::exit(2);
    }
    let request = visual_capture::ShowcaseCaptureRequest {
        captures: &selected,
        output_root: &output_root,
        theme_id: &theme_id,
        design_id: &design_id,
        viewport_override,
    };
    match visual_capture::capture_showcase_cases("metal-headless", &request) {
        Ok(report) => {
            println!("{}", report.to_markdown_table());
            if !report.passed {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("error: showcase capture failed: {error:#}");
            std::process::exit(1);
        }
    }
}

#[cfg(all(not(target_family = "wasm"), not(feature = "visual-capture")))]
fn run_visual_capture(_args: Vec<String>) {
    eprintln!("error: rebuild with --features visual-capture to enable --visual-capture");
    std::process::exit(2);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_miniapp::web_init();
    gpui_showcase::run_showcase();
}
