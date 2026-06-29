#![recursion_limit = "512"]
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
    BadgeNode, ButtonNode, CardNode, ChartKind, ChartNode, ProgressNode, PythonAppIr, Scene3dNode,
    SectionHeaderNode, SimpleNode, SpinnerNode, StackNode, TableNode, TabsNode, TextNode, UiNode,
};
use gpui_python_runtime::{LinesSpec, MeshSpec, SceneSpec, SurfaceSpec};
use gpui_ui_kit::theme::{Theme, ThemeExt};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "showcase/misc.rs"]
mod misc;
#[path = "showcase/python.rs"]
mod python;
#[path = "showcase/python_ir_showcase.rs"]
mod python_ir_showcase;
#[path = "showcase/types.rs"]
mod types;

use python_ir_showcase::PythonIrShowcase;

fn main() {
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

    MiniApp::run(
        MiniAppConfig::new("Python Showcase")
            .with_theme(true)
            .scrollable(false),
        |cx| cx.new(PythonIrShowcase::new_loading),
    );
}
