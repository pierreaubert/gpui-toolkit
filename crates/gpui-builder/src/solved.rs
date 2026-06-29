//! Solver output types.
//!
//! The solver produces a `SolvedNode` tree that mirrors the input `LayoutNode`
//! tree, with concrete pixel sizes and visibility states resolved.

mod format;
mod layout_debug_report;
mod layout_debug_warning;
mod misc;
mod solved_node;
mod solved_tree;
#[cfg(test)]
mod tests;
mod types;

pub use layout_debug_report::*;
pub use layout_debug_warning::*;
pub use solved_node::*;
pub use solved_tree::*;
pub use types::*;
