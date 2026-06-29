//! Platform-Adaptive Design System
//!
//! Defines shape, spacing, interaction, and animation rules that vary per
//! platform (Apple HIG/Liquid Glass, Material Design 3, Windows Fluent,
//! GNOME Adwaita, KDE Breeze) and opt-in product languages such as IBM Carbon
//! while the Theme system handles colors independently. The two layers are
//! independently combinable: any color theme works with any design system.
//!
//! This module contains only data types — no rendering code, no framework deps.
//! Platform renderers consume it alongside Theme colors.

mod animation_rules;
mod audio_control_rules;
mod corner_radii;
mod design_conformance_case;
mod design_conformance_matrix;
mod design_conformance_report;
mod design_ext;
mod design_language;
mod design_platform;
mod design_system;
mod design_system_state;
mod design_token;
mod design_token_export;
mod elevation_rules;
mod finite;
mod interaction_rules;
mod layout_thresholds;
mod spacing_rules;
#[cfg(test)]
mod tests;
mod types;
mod typography_rules;

pub use animation_rules::*;
pub use audio_control_rules::*;
pub use corner_radii::*;
pub use design_conformance_case::*;
pub use design_conformance_matrix::*;
pub use design_conformance_report::*;
pub use design_ext::*;
pub use design_language::*;
pub use design_platform::*;
pub use design_system::*;
pub use design_system_state::*;
pub use design_token::*;
pub use design_token_export::*;
pub use elevation_rules::*;
pub use interaction_rules::*;
pub use layout_thresholds::*;
pub use spacing_rules::*;
pub use types::*;
pub use typography_rules::*;
