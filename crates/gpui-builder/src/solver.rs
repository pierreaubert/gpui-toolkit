//! Layout Constraint Solver
//!
//! Pure function that resolves a `LayoutNode` tree into a `SolvedNode` tree
//! with concrete pixel sizes. No framework dependencies, no side effects.
//!
//! Algorithm (per container, recursive):
//! 1. Resolve axis (check `auto_axis` against width/height ratio)
//! 2. Apply user collapse preferences
//! 3. Allocate main-axis space:
//!    a. Sum Fixed children + divider space
//!    b. Reserve minimums for Fractional/Flex children
//!    c. If minimums exceed remaining → priority-based collapse (lowest first)
//!    d. Distribute remaining space
//! 4. Determine display tiers for each slot
//! 5. Recurse into container children

mod child_info;
mod misc;
mod resolve;
mod solve;
#[cfg(test)]
mod tests;
mod types;

pub use misc::TextMeasureCache;
pub use solve::*;
