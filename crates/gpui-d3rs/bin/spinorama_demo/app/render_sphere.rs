#![allow(
    unused_imports,
    reason = "r2factor facade keeps shared imports for split sphere render helpers"
)]

use super::SpinoramaApp;
use crate::types::Colormap;
use autoeq::DirectivityData;
use d3rs::gpu3d::{
    Colormap as Surface3DColormap, Surface3DConfig, Surface3DElement, SurfaceData as Surface3DData,
    SurfacePlotType,
};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_design::DesignExt;
use gpui_ui_kit::Slider;
use gpui_ui_kit::theme::ThemeExt;

mod misc;
