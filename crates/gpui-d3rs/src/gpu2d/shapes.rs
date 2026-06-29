//! GPU-accelerated shape rendering functions
//!
//! These functions mirror the API of src/shape/*.rs but use GPU rendering.

pub use crate::axis::{AxisConfig, AxisOrientation};
pub use crate::contour::{Contour, ContourBand};
pub use crate::shape::contour::{ContourConfig, HeatmapData};
pub use crate::shape::contour::{
    heat_color_scale, inferno_color_scale, magma_color_scale, plasma_color_scale,
    turbo_color_scale, viridis_color_scale,
};
pub use crate::shape::{
    BarConfig, BarDatum, CurveType, LineConfig, LinePoint, ScatterConfig, ScatterPoint,
};

mod gpu_axis_theme;
mod gpu_grid_config;
mod misc;
mod render;
mod types;

pub use gpu_axis_theme::*;
pub use gpu_grid_config::*;
pub use render::*;
