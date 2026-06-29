//! Layout story catalog primitives.
//!
//! Stories give examples, docs, tests, and future showcase tooling a shared way
//! to name layout trees and solve them across standard viewport scenarios.

mod layout_scenario;
mod layout_story;
mod layout_story_catalog;
mod misc;
mod solved_layout_scenario;
#[cfg(test)]
mod tests;
mod types;

pub use layout_scenario::*;
pub use layout_story::*;
pub use layout_story_catalog::*;
pub use solved_layout_scenario::*;
pub use types::*;
