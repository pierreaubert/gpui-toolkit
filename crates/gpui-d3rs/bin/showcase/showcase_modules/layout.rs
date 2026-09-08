//! Shared responsive layout helpers for showcase demos.
//!
//! Demos that pair a fixed-pixel plot with a fixed-pixel side menu in one
//! flex row must size the plot from the leftover content width — otherwise
//! the plot claims the full content width and pushes the menu off-screen.
//! `content_width` is recomputed from the window bounds on every render, so
//! sizing through [`plot_width`] keeps demos live on window resize.

/// Width of the standard demo side menu (px): every `w(px(280.0))` controls
/// column in the showcase.
pub const SIDE_MENU_WIDTH: f32 = 280.0;
/// Horizontal gap between a plot and its side menu (`gap_8` = 2 rem, px).
pub const SIDE_MENU_GAP: f32 = 32.0;
/// Minimum usable plot width (px) before the row must scroll instead.
pub const MIN_PLOT_WIDTH: f32 = 320.0;

/// Plot width for a row shared with a fixed side menu of `menu_width` px:
/// whatever the content has left after menu + gap, clamped to a minimum so
/// the menu is always visible.
pub fn plot_width(content_width: f32, menu_width: f32) -> f32 {
    (content_width - menu_width - SIDE_MENU_GAP).max(MIN_PLOT_WIDTH)
}
