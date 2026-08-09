//! Grid module for rendering background grids
//!
//! Grids provide visual guides for reading chart values.
//!
//! # Example
//!
//! ```rust,no_run
//! # #[cfg(feature = "gpui")]
//! # {
//! use d3rs::prelude::*;
//! use d3rs::grid::{render_grid, GridConfig};
//! use d3rs::axis::DefaultAxisTheme;
//!
//! let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 400.0);
//! let y_scale = LinearScale::new().domain(0.0, 100.0).range(300.0, 0.0);
//! let config = GridConfig::with_lines();
//! let theme = DefaultAxisTheme;
//!
//! // render_grid(&x_scale, &y_scale, &config, 400.0, 300.0, &theme)
//! # }
//! ```

mod config;
mod layout;
#[cfg(all(feature = "gpui", not(test)))]
mod render;

pub use config::GridConfig;
pub use layout::{GridDot, GridLayout, GridLayoutError, GridLine, GridPoint, grid_layout};
#[cfg(all(feature = "gpui", not(test)))]
pub use render::render_grid;
