//! QuadTree module for spatial indexing (d3-quadtree inspired)
//!
//! This module provides a quadtree data structure for efficient 2D spatial queries.
//! Quadtrees recursively subdivide 2D space into four quadrants, enabling O(log n)
//! nearest neighbor searches and range queries.
//!
//! # Features
//!
//! - **Spatial Indexing**: Fast insertion and lookup of 2D points
//! - **Nearest Neighbor Search**: Find closest point to any location
//! - **Range Queries**: Find all points within a radius
//! - **Tree Traversal**: Visit nodes in pre-order or post-order
//!
//! # Example
//!
//! ```rust
//! use d3rs::quadtree::QuadTree;
//!
//! // Create a quadtree with some points
//! let points = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
//! let tree = QuadTree::from_data(&points, |p| p.0, |p| p.1);
//!
//! // Find nearest point to (1.5, 1.5)
//! if let Some(point) = tree.find(1.5, 1.5, None) {
//!     println!("Nearest point: {:?}", point);
//! }
//! ```

mod extent;
mod quad_node;
mod quad_point;
mod quad_tree;

pub use extent::*;
pub use quad_node::*;
pub use quad_point::*;
pub use quad_tree::*;
