#![recursion_limit = "512"]
#![forbid(unsafe_code)]
#![allow(
    unused_imports,
    reason = "r2factor facade keeps shared imports for split showcase modules"
)]

//! Native GPUI host for Python-authored showcase apps.

use gpui::*;
use gpui_design::{DesignExt, DesignSystem};
use gpui_miniapp::{MiniApp, MiniAppConfig};
use gpui_px::{ColorScale, ScaleType, bar, heatmap, line, scatter};
use gpui_python_runtime::gpui_adapter::Gpui3DCache;
use gpui_python_runtime::ui_ir::{
    BadgeNode, ButtonNode, CardNode, ProgressNode, PythonAppIr, Scene3dNode, SectionHeaderNode,
    SimpleNode, SpinnerNode, StackNode, TableNode, TabsNode, TextNode, UiNode,
};
use gpui_python_runtime::{LinesSpec, MeshSpec, SceneSpec, SurfaceSpec};
use gpui_ui_kit::Language;
use gpui_ui_kit::theme::{Theme, ThemeExt, ThemeVariant};
#[cfg(feature = "embedded")]
use gpui_ui_kit::{I18nState, ThemeState, accessibility::AccessibilityTree};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "showcase/credentials.rs"]
mod credentials;
#[path = "showcase/host_state.rs"]
mod host_state;
#[path = "showcase/misc.rs"]
mod misc;
#[path = "showcase/python.rs"]
mod python;
#[path = "showcase/python_ir_showcase.rs"]
mod python_ir_showcase;
#[path = "showcase/types.rs"]
mod types;

use host_state::PresentationStore;
use python_ir_showcase::PythonIrShowcase;

fn theme_variant(value: &str) -> ThemeVariant {
    match value.to_ascii_lowercase().as_str() {
        "light" => ThemeVariant::Light,
        "midnight" => ThemeVariant::Midnight,
        "forest" => ThemeVariant::Forest,
        "black_and_white" => ThemeVariant::BlackAndWhite,
        "onyx" => ThemeVariant::Onyx,
        "carbon_white" => ThemeVariant::CarbonWhite,
        "carbon_gray_10" => ThemeVariant::CarbonGray10,
        "carbon_gray_90" => ThemeVariant::CarbonGray90,
        "carbon_gray_100" => ThemeVariant::CarbonGray100,
        _ => ThemeVariant::Dark,
    }
}

fn language(value: &str) -> Language {
    match value.to_ascii_lowercase().as_str() {
        "french" => Language::French,
        "german" => Language::German,
        "spanish" => Language::Spanish,
        "japanese" => Language::Japanese,
        _ => Language::English,
    }
}

fn miniapp_config(app: &PythonAppIr, presentation: &PresentationStore) -> MiniAppConfig {
    let saved = presentation.snapshot();
    let shell = app.miniapp.as_ref();
    let title = shell.map_or(&app.title, |config| &config.title);
    let width = shell.map_or(app.width, |config| Some(config.width));
    let height = shell.map_or(app.height, |config| Some(config.height));
    let mut config = MiniAppConfig::new(title)
        .size(width.unwrap_or(saved.width), height.unwrap_or(saved.height))
        // PythonIrShowcase owns its content scroll so the shell must not add a
        // second, competing scroll container.
        .scrollable(false);
    if let Some(shell) = shell {
        config = config
            .app_name(shell.app_name.clone())
            .with_theme(shell.with_theme)
            .with_i18n(shell.with_i18n)
            .initial_theme(theme_variant(&shell.initial_theme))
            .initial_language(language(&shell.initial_language));
    }
    config
}

pub fn main() {
    if env::var_os("GPUI_TOOLKIT_VALIDATE_ONLY").is_some() {
        match python::load_python_app_blocking() {
            Ok(app) => {
                println!(
                    "validated Python GPUI app {:?} with {} sections",
                    app.title,
                    app.sections.len()
                );
            }
            Err(error) => {
                eprintln!("failed to validate Python GPUI app: {error}");
                std::process::exit(1);
            }
        }
        return;
    }

    let (app, session) = match python::load_python_session_blocking() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to initialize Python GPUI app: {error}");
            std::process::exit(1);
        }
    };
    let presentation = PresentationStore::open();
    let config = miniapp_config(&app, &presentation);
    MiniApp::run(config, move |cx| {
        cx.new(|cx| PythonIrShowcase::new_ready(cx, presentation, app, session))
    });
}

/// Launches a Python app from an embedded-host app callback (mobile).
///
/// The GPUI `Application` and its platform run loop are owned by the
/// mobile entry point (iOS `set_app_callback`/`run_app`, Android
/// `android_main`); this installs the same theme/design/accessibility/
/// i18n globals the scaffolder's mobile entries set, loads the app
/// through the in-process session, and opens the fullscreen window.
/// Startup failures are logged and leave the app without a window,
/// matching the native mobile showcases.
#[cfg(all(
    feature = "embedded",
    any(target_os = "ios", target_os = "tvos", target_os = "android")
))]
pub fn run_embedded_app(cx: &mut App, script: PathBuf, python_paths: Vec<PathBuf>) {
    let (app, session) = match python::load_embedded_python_session_blocking(script, python_paths) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to initialize embedded Python GPUI app: {error}");
            return;
        }
    };
    let presentation = PresentationStore::open();
    let config = miniapp_config(&app, &presentation);
    cx.set_global(ThemeState::with_variant(config.initial_theme));
    cx.set_global(gpui_design::DesignSystemState::new());
    cx.set_global(AccessibilityTree::new());
    let mut i18n = I18nState::new();
    i18n.set_language(config.initial_language);
    cx.set_global(i18n);

    let open_result = cx.open_window(
        WindowOptions {
            window_bounds: None,
            ..Default::default()
        },
        |_, cx| cx.new(|cx| PythonIrShowcase::new_ready(cx, presentation, app, session)),
    );
    match open_result {
        Ok(_) => cx.activate(true),
        Err(error) => eprintln!("failed to open embedded Python app window: {error}"),
    }
}
