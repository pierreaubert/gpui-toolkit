//! Layout declaration validation.
//!
//! The solver is intentionally permissive and deterministic. This module adds
//! a separate lint pass for examples, tests, and CI so suspicious declarations
//! can be caught before a layout is solved.

mod layout_issue_severity;
mod layout_validation_report;
mod misc;
mod push;
#[cfg(test)]
mod tests;
mod types;
mod validate;

pub use layout_issue_severity::*;
pub use layout_validation_report::*;
pub use types::*;
pub use validate::*;
