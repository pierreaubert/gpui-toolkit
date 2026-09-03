//! Theme definition with serialization support
//!
//! Provides a serializable theme structure that can be exported to JSON or Rust code.

pub use gpui_ui_kit::Color;

mod accent_palette;
mod accessibility_palette;
mod built_in_theme_preset;
mod color_group;
mod community_theme_bundle;
mod community_theme_manifest;
mod contrast_fix;
mod default;
mod editor_theme;
mod misc;
#[cfg(test)]
mod tests;
mod theme_gallery;
mod theme_mode_preference;
mod theme_schedule;
mod theme_transition;
mod token_export;
mod time_of_day;
mod tui_ansi_palette;
mod tui_theme_preset;
mod types;

pub use accent_palette::*;
pub use accessibility_palette::*;
pub use built_in_theme_preset::*;
pub use color_group::*;
pub use community_theme_bundle::*;
pub use community_theme_manifest::*;
pub use contrast_fix::*;
pub use editor_theme::*;
pub use misc::*;
pub use theme_gallery::*;
pub use theme_mode_preference::*;
pub use theme_schedule::*;
pub use theme_transition::*;
pub use token_export::*;
pub use time_of_day::*;
pub use tui_ansi_palette::*;
pub use tui_theme_preset::*;
pub use types::*;
