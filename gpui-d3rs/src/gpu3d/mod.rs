//! GPU-accelerated 3D surface rendering module
//!
//! This module provides real-time 3D surface visualization using wgpu,
//! designed for scientific data like spinorama dispersion plots and
//! general z=f(x,y) surface rendering.
//!
//! # Features
//!
//! - **GPU Rendering**: Hardware-accelerated rendering via wgpu
//! - **Interactive Controls**: Orbit, zoom, and pan camera controls
//! - **Colormap Support**: Multiple scientific colormaps for data visualization
//! - **GPUI Integration**: Seamless integration with GPUI elements
//!
//! # Example
//!
//! ```rust,ignore
//! use d3rs::surface3d::{Surface3DElement, Surface3DConfig, SurfaceData};
//!
//! // Create surface data
//! let data = SurfaceData::from_grid(x_values, y_values, z_values);
//!
//! // Configure and create element
//! let config = Surface3DConfig::new()
//!     .colormap(Colormap::Viridis)
//!     .wireframe(false);
//!
//! // Use in GPUI view
//! Surface3DElement::new(data, config)
//! ```

mod camera;
mod config;
mod data;
mod element;
mod mesh;
mod renderer;
mod shaders;

pub use camera::{Camera3D, OrbitControls};
pub use config::{Colormap, Surface3DConfig, SurfacePlotType};
pub use data::{SurfaceData, SurfaceVertex};
pub use element::{Surface3DElement, Surface3DState};
pub use mesh::SurfaceMesh;
pub use renderer::Surface3DRenderer;
pub mod projection_tests;
