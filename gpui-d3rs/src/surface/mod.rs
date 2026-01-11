//! 3D Surface plot module for isometric and projected surface visualization
//!
//! This module provides surface plotting capabilities using 2D projection of 3D data.
//! Surfaces are rendered using GPUI's native paint API via painter's algorithm.
//!
//! # Features
//!
//! - Surface data with color mapping via a fourth value `t`
//! - Multiple projection types: isometric, oblique, orthographic, perspective
//! - Configurable camera position and rotation
//! - Both continuous and discrete color scales
//! - Wireframe overlay support
//! - Simple ambient lighting model
//! - **Logarithmic axis sampling** for X, Y, or both axes (ideal for frequency domain plots)
//!
//! # Example
//!
//! ```rust,no_run
//! use d3rs::surface::{SurfaceData, SurfaceConfig, render_surface, ColorScaleType};
//!
//! // Create surface from a mathematical function
//! let data = SurfaceData::from_function(
//!     (-2.0, 2.0),  // x range
//!     (-2.0, 2.0),  // y range
//!     50,           // resolution
//!     |x, y| {
//!         let z = (x*x + y*y).sin();
//!         let t = z;  // Color by z value
//!         (z, t)
//!     },
//! );
//!
//! // Render with configuration
//! let element = render_surface(
//!     &data,
//!     SurfaceConfig::new()
//!         .isometric()
//!         .rotation(30.0, 45.0)
//!         .color_scale(ColorScaleType::Viridis)
//!         .opacity(0.8)
//!         .wireframe(true),
//!     600.0,
//!     400.0,
//! );
//! ```
//!
//! # Logarithmic Axis Example
//!
//! ```rust,no_run
//! use d3rs::surface::{SurfaceData, SurfaceConfig, render_surface};
//!
//! // Frequency response plot with logarithmic X-axis (20 Hz to 20 kHz)
//! let freq_response = SurfaceData::from_z_function_logx(
//!     (20.0, 20000.0),  // X: Frequency (logarithmic)
//!     (0.0, 1.0),       // Y: Time or channel (linear)
//!     100,
//!     |freq, time| {
//!         // Simulated frequency response magnitude
//!         let magnitude_db = if freq < 100.0 {
//!             -6.0 * (1.0 - freq / 100.0)
//!         } else if freq > 10000.0 {
//!             -3.0 * (freq - 10000.0) / 10000.0
//!         } else {
//!             0.0
//!         };
//!         magnitude_db
//!     },
//! );
//!
//! let element = render_surface(&freq_response, SurfaceConfig::new(), 800.0, 400.0);
//! ```

mod data;
mod mesh;
mod projection;
#[cfg(feature = "gpui")]
mod render;

pub use data::{SurfaceData, SurfacePoint3D};
pub use mesh::{SurfaceMesh, Triangle};
pub use projection::{
    Camera2D, IsometricProjection, ObliqueProjection, OrthographicProjection,
    PerspectiveProjection, Projection, ProjectionType,
};
#[cfg(feature = "gpui")]
pub use render::{ColorScaleType, SurfaceConfig, SurfaceElement, render_surface};
