//! Stacked to Grouped Bars - D3.js Example Port
//!
//! This example demonstrates animated transitions between stacked and grouped bar charts,
//! ported from: <https://observablehq.com/@d3/stacked-to-grouped-bars>
//!
//! Features:
//! - Smooth animated transitions between layouts
//! - Multiple data series with different colors
//! - Staggered animations for visual appeal

mod bar_layout;
mod compute;
mod misc;

pub use bar_layout::*;
