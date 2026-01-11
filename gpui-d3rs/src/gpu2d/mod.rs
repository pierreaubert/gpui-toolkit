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
//! The module follows the same pattern as `surface3d`: render to a wgpu texture,
//! copy pixels back, and paint via GPUI's `window.paint_image()`.
//!
//! # Example
//!
//! ```rust,ignore
//! use d3rs::gpu2d::{Chart2DElement, Chart2DRenderer};
//!
//! // Create a chart element
//! let element = Chart2DElement::new(|renderer, bounds| {
//!     renderer.draw_line(0.0, 0.0, 100.0, 100.0, 2.0, [1.0, 0.0, 0.0, 1.0]);
//!     renderer.draw_rect(Rect::new(10.0, 10.0, 50.0, 30.0), [0.0, 1.0, 0.0, 1.0], 4.0);
//!     renderer.draw_circle(75.0, 75.0, 10.0, [0.0, 0.0, 1.0, 1.0]);
//! });
//! ```

mod device;
mod element;
mod renderer;
mod shaders;
mod shapes;

pub mod primitives;
pub mod text;

pub use device::Gpu2DContext;
pub use element::Chart2DElement;
pub use renderer::Chart2DRenderer;

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
    ScatterConfig,
    ScatterPoint,
    heat_color_scale,
    inferno_color_scale,
    magma_color_scale,
    plasma_color_scale,
    render_axis,
    render_bars,
    render_contour,
    render_contour_bands,
    render_grid,
    render_heatmap,
    render_line,
    render_scatter,
    turbo_color_scale,
    viridis_color_scale,
};
