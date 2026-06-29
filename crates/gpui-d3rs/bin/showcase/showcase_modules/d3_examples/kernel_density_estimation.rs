//! Kernel Density Estimation - D3.js Example Port
//!
//! This example demonstrates kernel density estimation with histogram overlay,
//! ported from: <https://observablehq.com/@d3/kernel-density-estimation>
//!
//! The example shows both:
//! 1. **High-level API**: Using d3rs shape primitives
//! 2. **Low-level API**: Direct computation of KDE with custom kernels

mod kde_curve_element;
mod kernel_type;
mod misc;

pub use kde_curve_element::*;
pub use kernel_type::*;
