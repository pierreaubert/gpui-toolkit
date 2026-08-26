//! GPU-accelerated sphere gallery component
//!
//! Displays a grid of images (e.g., album art) projected onto a 3D spherical
//! dome surface. The center of the grid is the apex (highest point), creating
//! a "halo" or dome effect where images curve away from the viewer at the edges.
//!
//! # Features
//!
//! - **GPU Rendering**: Hardware-accelerated via wgpu with texture atlas
//! - **Interactive**: Mouse hover/click, keyboard arrow navigation, scroll zoom
//! - **Sphere Projection**: Configurable dome angle, radius, and subdivisions
//! - **Selection**: Visual highlighting of hovered and selected cells
//!
//! # Usage
//!
//! ```rust,ignore
//! use d3rs::sphere_gallery::{SphereGalleryView, SphereGalleryItem, SphereGalleryConfig};
//!
//! // Create items (each is cell_size × cell_size RGBA pixels)
//! let items: Vec<SphereGalleryItem> = album_arts.iter().map(|art| {
//!     SphereGalleryItem {
//!         pixels: art.to_rgba_pixels(),
//!         label: Some(art.title.clone().into()),
//!     }
//! }).collect();
//!
//! // Configure grid
//! let config = SphereGalleryConfig::new(5, 4);
//!
//! // Create view
//! let view = cx.new(|_| SphereGalleryView::new(items, config));
//! ```
//!
//! ## Controls
//!
//! - **Left Click**: Select hovered cell (or start drag if no cell under cursor)
//! - **Left Drag**: Rotate the sphere
//! - **Scroll Wheel**: Zoom in/out
//! - **Double Click**: Reset camera to initial position
//! - **Arrow Keys**: Move selection between cells
//! - **Enter/Space**: Confirm selection
//! - **Home/End**: Jump to first/last cell
//! - **Escape**: Clear selection
//! - **R**: Reset camera

mod element;
mod mesh;
mod renderer;
mod shaders;
mod wgpu_draw;

pub use element::{SphereGalleryElement, SphereGalleryItem, SphereGalleryState, SphereGalleryView};
pub use mesh::{Projection, SphereMeshConfig};
pub use renderer::SphereGalleryConfig;
