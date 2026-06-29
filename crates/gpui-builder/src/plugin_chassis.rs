//! Plugin chassis layout descriptors.
//!
//! Owned, platform-agnostic data structures describing the visual structure
//! of an audio plugin's UI chassis: a header strip, a horizontal row of
//! sections (each with an eyebrow / title / caption + a stack of rows), and
//! an optional footer.
//!
//! These descriptors are consumed by the app-gpui renderer to paint the
//! plugin UI using the active `PluginTheme`. The renderer walks
//! [`ChassisLayout::sections`] in order and uses [`ChassisLayout::solve`] to
//! determine which sections fit at the available width and which collapse.
//!
//! No rendering code lives here. No GPUI dependencies.

mod chassis_layout;
mod row_spec;
mod solved_chassis;
#[cfg(test)]
mod tests;
mod types;

pub use chassis_layout::*;
pub use row_spec::*;
pub use solved_chassis::*;
pub use types::*;
