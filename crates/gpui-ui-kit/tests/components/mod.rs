//! Component unit tests for GPUI UI Kit
//!
//! This module provides unit tests for all UI components that verify
//! component creation, configuration, and API correctness without
//! requiring a GPUI window.
//!
//! Test organization:
//! - Each component has its own test file
//! - Tests verify builder patterns work correctly
//! - Tests verify all variants and sizes are accessible
//! - Tests verify event handlers can be attached

// Form Controls
mod button_test;
mod checkbox_test;
mod input_test;
mod select_test;
mod slider_test;
mod toggle_test;

// Display Components
mod accordion_test;
mod alert_test;
mod avatar_test;
mod badge_test;
mod card_test;
mod dialog_test;
mod icon_button_test;

// Navigation Components
mod breadcrumbs_test;
mod button_set_test;
mod menu_test;
mod tabs_test;

// Layout Components
mod pane_divider_test;

// Feedback Components
mod progress_test;
mod spinner_test;
mod text_test;
mod toast_test;
mod tooltip_test;

// Theme
mod number_input_test;
mod theme_test;
