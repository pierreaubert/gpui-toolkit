//! GPU-accelerated 2D chart rendering module
//!
//! This module provides hardware-accelerated rendering for 2D charts including:
//! - Lines with configurable width and anti-aliasing
//! - Rectangles with optional rounded corners
//! - Circles/points with smooth edges
//! - Text rendering via font atlas
//!
//! # Architecture
//!
//! GPUI presentation uses the Vello custom-draw path. The former
//! render/readback/re-upload compatibility element has been removed; this
//! module now contains shared device, primitive, and shape data.

mod device;
mod shapes;

pub mod primitives;
pub mod text;

pub use device::Gpu2DContext;

// GPU-accelerated shape rendering functions
pub use shapes::{
    // Re-export types from shape module for convenience
    AxisConfig,
    AxisOrientation,
    BarConfig,
    BarDatum,
    Contour,
    ContourBand,
    // Contour types
    ContourConfig,
    CurveType,
    GpuAxisTheme,
    GpuGridConfig,
    HeatmapData,
    LineConfig,
    LinePoint,
    LodScatter,
    LodScatterConfig,
    ScatterConfig,
    ScatterPoint,
    heat_color_scale,
    inferno_color_scale,
    magma_color_scale,
    plasma_color_scale,
    turbo_color_scale,
    viridis_color_scale,
};

/// Legacy readback-backed shape renderers.
///
/// Prefer the `crate::shape` renderers, including their `*_vello` variants,
/// which paint through `WgpuCustomDraw` when the Vello backend is available.
#[deprecated(
    since = "0.1.0",
    note = "use the Vello-backed crate::shape renderers; gpu2d shape renderers perform a GPU readback"
)]
pub use shapes::{
    render_axis, render_bars, render_contour, render_contour_bands, render_grid, render_heatmap,
    render_line, render_lod_scatter, render_scatter,
};
