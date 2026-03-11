//! AutoEQ Parameter Form
//!
//! A reusable form component for AutoEQ optimization parameters.
//! Used by Room EQ, Speaker EQ, Headphone EQ, and Group optimization screens.

// Allow complex callback types - common in UI code
#![allow(clippy::type_complexity)]
// Allow to_* methods that take self by reference - matches GPUI patterns
#![allow(clippy::wrong_self_convention)]

pub mod config;
pub mod constants;
mod form;
mod render;
pub mod theme;
pub mod ui_state;

// Re-export all public types
pub use config::{AutoEqConfig, ParamLimits};
pub use constants::*;
pub use form::{AutoEqForm, AutoEqLayoutMode};
pub use theme::AutoEqFormTheme;
pub use ui_state::AutoEqFormUiState;
