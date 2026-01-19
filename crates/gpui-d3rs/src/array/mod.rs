//! Array manipulation and data utilities
//!
//! This module provides functions for manipulating and summarizing data arrays,
//! inspired by d3-array. It includes statistics, search, binning, transformations,
//! and set operations.
//!
//! # Example
//!
//! ```rust
//! use d3rs::array::{mean, min_by, max_by, bisect_right_f64};
//!
//! let data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
//! assert_eq!(mean(&data), Some(3.875));
//! assert_eq!(min_by(&data, |a, b| a.partial_cmp(b).unwrap()), Some(&1.0));
//! assert_eq!(max_by(&data, |a, b| a.partial_cmp(b).unwrap()), Some(&9.0));
//!
//! let sorted = vec![1.0, 2.0, 3.0, 5.0, 8.0, 13.0];
//! assert_eq!(bisect_right_f64(&sorted, 4.0), 3);
//! ```

pub mod bin;
mod search;
mod sets;
pub mod statistics;
mod ticks;
mod transform;

pub use bin::*;
pub use search::*;
pub use sets::*;
pub use statistics::*;
pub use ticks::*;
pub use transform::*;
