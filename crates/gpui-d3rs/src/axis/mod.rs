//! Axis module for chart axes
//!
//! Axes provide visual reference for scales, showing tick marks and labels.
//! The [`AxisLayout`] surface is renderer-independent and available in
//! metadata/no-default builds; `render_axis` is available in GPUI builds.
//!
//! # Example
//!
//! ```rust,no_run
//! # #[cfg(feature = "gpui")]
//! # {
//! use d3rs::scale::LinearScale;
//! use d3rs::axis::{render_axis, AxisConfig, DefaultAxisTheme};
//!
//! let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
//! let config = AxisConfig::bottom().with_ticks(10);
//! let theme = DefaultAxisTheme;
//!
//! // render_axis(&scale, &config, 400.0, &theme)
//! # }
//! ```

mod config;
mod layout;
mod orientation;
#[cfg(all(feature = "gpui", not(test)))]
mod render;
#[cfg(feature = "gpui")]
mod theme;

pub use config::AxisConfig;
pub use layout::{
    AxisLayout, AxisLayoutError, AxisLine, AxisPoint, AxisTick, AxisTitle, axis_layout,
};
pub use orientation::AxisOrientation;
#[cfg(all(feature = "gpui", not(test)))]
pub use render::render_axis;
#[cfg(feature = "gpui")]
pub use theme::{AxisTheme, DefaultAxisTheme};
