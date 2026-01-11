//! Axis module for rendering chart axes
//!
//! Axes provide visual reference for scales, showing tick marks and labels.
//!
//! # Example
//!
//! ```rust,no_run
//! use d3rs::prelude::*;
//! use d3rs::axis::{render_axis, AxisConfig, DefaultAxisTheme};
//!
//! let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
//! let config = AxisConfig::bottom().with_ticks(10);
//! let theme = DefaultAxisTheme;
//!
//! // render_axis(&scale, &config, 400.0, &theme)
//! ```

mod config;
mod orientation;
mod render;
mod theme;

pub use config::AxisConfig;
pub use orientation::AxisOrientation;
pub use render::render_axis;
pub use theme::{AxisTheme, DefaultAxisTheme};
