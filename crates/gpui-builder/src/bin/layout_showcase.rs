//! Layout Builder Showcase
//!
//! Interactive demo of the gpui-builder constraint solver rendered live in GPUI.
//! Demonstrates a 3-panel app layout with:
//! - Fixed header and footer (hard constraints)
//! - Collapsible sidebar and inspector (soft constraints)
//! - Auto-axis switching (resize window to portrait to see vertical stacking)
//! - Display tiers on the inspector panel (Full/Mini based on size)
//! - Draggable dividers to resize panels
//! - Real-time solver output in footer
//! - Visual solved-tree inspector with live node highlighting
//!
//! Run: cargo run -p gpui-builder --features showcase --bin layout-showcase

use gpui::AppContext;
use gpui::{App, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use gpui_design::DesignSystemState;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

#[path = "layout_showcase/drag_session.rs"]
mod drag_session;
#[path = "layout_showcase/misc.rs"]
mod misc;
#[path = "layout_showcase/showcase_theme.rs"]
mod showcase_theme;
#[path = "layout_showcase/showcase_view.rs"]
mod showcase_view;
#[path = "layout_showcase/types.rs"]
mod types;

use misc::current_platform;
use showcase_view::ShowcaseView;

fn main() {
    if let Err(error) = run() {
        eprintln!("Layout showcase error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let platform =
        current_platform().map_err(|error| format!("platform initialization failed: {error}"))?;
    let (smoke_test, smoke_artifact) = smoke_options()?;
    let render_count = Arc::new(AtomicUsize::new(0));
    let render_count_for_app = render_count.clone();

    gpui::Application::with_platform(platform).run(move |cx: &mut App| {
        cx.set_global(DesignSystemState::new());

        let bounds = Bounds::centered(None, size(px(1000.0), px(700.0)), cx);
        let render_probe = smoke_test.then(|| render_count_for_app.clone());
        let window = match cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Layout Builder Showcase".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_cx| ShowcaseView::new(render_probe)),
        ) {
            Ok(window) => window,
            Err(error) => {
                eprintln!("Layout showcase window error: {error:?}");
                if smoke_test {
                    std::process::exit(1);
                }
                cx.quit();
                return;
            }
        };

        cx.activate(true);

        if smoke_test {
            let render_count = render_count_for_app.clone();
            let smoke_artifact = smoke_artifact.clone();
            cx.spawn(async move |cx| {
                let executor = cx.background_executor().clone();
                for _ in 0..100 {
                    if render_count.load(Ordering::Acquire) >= 2 {
                        break;
                    }
                    executor.timer(Duration::from_millis(50)).await;
                }
                let final_render_count = render_count.load(Ordering::Acquire);
                if final_render_count < 2 {
                    eprintln!(
                        "native smoke state transition did not trigger a second render \
                         (renders={final_render_count})"
                    );
                    std::process::exit(1);
                }
                let transition_verified =
                    match window.read_with(cx, |view, _| view.sidebar_collapsed) {
                        Ok(verified) => verified,
                        Err(error) => {
                            eprintln!("native smoke state transition read failed: {error}");
                            std::process::exit(1);
                        }
                    };
                if !transition_verified {
                    eprintln!("native smoke second render did not retain sidebar transition");
                    std::process::exit(1);
                }
                if let Some(hold_ms) = smoke_hold_millis() {
                    executor.timer(Duration::from_millis(hold_ms)).await;
                }

                if let Err(error) = write_smoke_artifact(
                    smoke_artifact.as_deref(),
                    final_render_count,
                    transition_verified,
                ) {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
                println!(
                    "GPUI_NATIVE_SMOKE_OK platform={} renders={} transition={transition_verified}",
                    std::env::consts::OS,
                    final_render_count
                );
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
    });

    Ok(())
}

fn smoke_options() -> Result<(bool, Option<PathBuf>), String> {
    let mut smoke_test = std::env::var_os("GPUI_NATIVE_SMOKE").is_some();
    let mut artifact = std::env::var_os("GPUI_NATIVE_SMOKE_ARTIFACT").map(PathBuf::from);
    let mut arguments = std::env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--smoke-test" {
            smoke_test = true;
        } else if argument == "--smoke-artifact" {
            let Some(path) = arguments.next() else {
                return Err("--smoke-artifact requires a path".to_string());
            };
            artifact = Some(path.into());
        } else {
            return Err(format!("unknown argument: {}", argument.to_string_lossy()));
        }
    }
    Ok((smoke_test, artifact))
}

fn smoke_hold_millis() -> Option<u64> {
    std::env::var("GPUI_NATIVE_SMOKE_HOLD_MS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|hold_ms| *hold_ms > 0)
}

fn write_smoke_artifact(
    path: Option<&Path>,
    render_count: usize,
    transition_verified: bool,
) -> Result<(), String> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create smoke artifact directory: {error}"))?;
    }
    let report = format!(
        concat!(
            "{{\n",
            "  \"schema_version\": 3,\n",
            "  \"report_type\": \"gpui-native-smoke\",\n",
            "  \"crate\": \"gpui-builder\",\n",
            "  \"platform\": \"{}\",\n",
            "  \"window_opened\": true,\n",
            "  \"render_invoked\": true,\n",
            "  \"render_count\": {},\n",
            "  \"state_transition\": \"collapse-sidebar\",\n",
            "  \"state_transition_verified\": {},\n",
            "  \"interaction_scope\": [\"window-open\", \"render\", \"collapse-sidebar\"],\n",
            "  \"pixel_capture\": false\n",
            "}}\n"
        ),
        std::env::consts::OS,
        render_count,
        transition_verified
    );
    std::fs::write(path, report)
        .map_err(|error| format!("failed to write smoke artifact {}: {error}", path.display()))
}
