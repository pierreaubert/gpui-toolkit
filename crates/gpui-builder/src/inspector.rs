//! Layout inspection records for developer tooling.
//!
//! The core solver stays platform agnostic. This module exposes owned, stable
//! records that debug overlays, showcase apps, and snapshot tests can consume
//! without depending on borrowed declaration lifetimes or solver internals.

mod format;
mod inspect;
mod layout_inspection;
mod layout_inspection_node;
mod misc;
mod option;
mod sizing_inspection;
mod solved_inspection;
mod solved_inspection_node;
#[cfg(test)]
mod tests;
mod types;

pub use inspect::*;
pub use layout_inspection::*;
pub use layout_inspection_node::*;
pub use sizing_inspection::*;
pub use solved_inspection::*;
pub use solved_inspection_node::*;
pub use types::*;
