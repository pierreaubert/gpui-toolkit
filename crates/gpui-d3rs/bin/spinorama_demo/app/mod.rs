#![allow(
    unused_imports,
    reason = "r2factor facade keeps shared imports for split render modules"
)]

use super::render::{freq_spl_plot_margins, render_freq_spl_plot};
use super::types::{
    BrushOverlay, ChartId, Colormap, ContourRenderMode, DirectivityPlane, LinePoint, LoadState,
    PlotCurve, PlotSection, SecondaryAxisConfig,
};
use super::utils::{
    CEA2034_CURVES, ChartTheme, cea2034_colors, format_frequency, get_angle_range,
    interpolate_spl_at_frequency,
};
use autoeq::read::{
    ContourPlotData, extract_cea2034_curves_original, fetch_available_speakers, fetch_contour_data,
    fetch_directivity_data, fetch_measurement_plot_data,
};
use autoeq::{Curve, DirectivityData};
use d3rs::axis::{AxisConfig, render_axis};
use d3rs::brush::{BrushSelection, BrushState};
use d3rs::color::D3Color;
use d3rs::contour::ContourGenerator;
use d3rs::gpu3d::{
    Colormap as Surface3DColormap, Surface3DConfig, Surface3DElement, Surface3DState,
    SurfaceData as Surface3DData, SurfacePlotType,
};
use d3rs::grid::{GridConfig, render_grid};
use d3rs::scale::{LinearScale, LogScale};
use d3rs::shape::StrokeDashArray;
use d3rs::shape::{
    ContourConfig, HeatmapData, render_contour_bands_selected as render_contour_bands,
    render_contour_selected as render_contour, render_heatmap_selected as render_heatmap,
};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use d3rs::zoom::ZoomState;
use gpui::prelude::*;
use gpui::{deferred, *};
use gpui_builder::{
    Axis, ContainerNode, LayoutNode, Sizing, SlotNode, SolvedNode, solve, types::LayoutPreferences,
};
use gpui_design::{DesignExt, DesignSystem};
use gpui_ui_kit::theme::{Theme, ThemeExt};
use gpui_ui_kit::{SelectOption, Spinner, SpinnerSize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::Runtime;

include!("render_cea2034.rs");
include!("render_directivity.rs");
include!("render_contour.rs");
include!("render_contour_2d.rs");
include!("render_contour_3d.rs");
include!("render_contour_old.rs");
include!("render_contour_polar.rs");
include!("render_legend.rs");
include!("render_mode_toggle.rs");
include!("render_polar.rs");
mod render_sphere;

mod spinorama_app;

pub use spinorama_app::*;
