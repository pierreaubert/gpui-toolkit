//! Flare.js Hierarchy Dataset
//!
//! This dataset represents the package structure of the Flare visualization library,
//! with byte sizes for each component.
//!
//! # Data Source
//! - Original: Flare visualization toolkit (<http://flare.prefuse.org/>)
//! - D3 version: <https://github.com/d3/d3-hierarchy/blob/main/test/data/flare.json>
//!
//! # License
//! The Flare library is licensed under the BSD license.
//! This data representation (file sizes/structure) is factual and non-copyrightable.

mod hierarchy_node;
mod misc;

pub use hierarchy_node::*;
pub use misc::*;
