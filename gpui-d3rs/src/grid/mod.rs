//! Grid module for rendering background grids
//!
//! Grids provide visual guides for reading chart values.
//!
//! # Example
//!
//! ```rust,no_run
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
//! ```

mod config;
mod render;

pub use config::GridConfig;
pub use render::render_grid;
