#![allow(
    clippy::too_many_arguments,
    reason = "showcase chart helpers mirror plotting formulas and visual geometry"
)]

//! d3rs Showcase - Unified demo application
//!
//! Demonstrates all d3rs functionality in a single application with tabbed navigation.

use gpui::prelude::*;
use gpui_miniapp::{MiniApp, MiniAppConfig};

mod showcase_modules;

#[path = "main/contour_render_mode.rs"]
mod contour_render_mode;
#[path = "main/demo_section.rs"]
mod demo_section;
#[path = "main/geo_projection_type.rs"]
mod geo_projection_type;
#[path = "main/showcase_app.rs"]
mod showcase_app;

pub use contour_render_mode::*;
pub use demo_section::*;
pub use geo_projection_type::*;
pub use showcase_app::*;

fn main() {
    MiniApp::run(
        MiniAppConfig::new("d3rs Showcase")
            .size(1000.0, 800.0)
            .with_theme(true)
            .with_i18n(true),
        |cx| cx.new(ShowcaseApp::new),
    );
}
