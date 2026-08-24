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
mod interaction_test;
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
mod table_test;

// Navigation Components
mod breadcrumbs_test;
mod button_set_test;
mod context_menu_test;
mod menu_test;
mod tabs_test;

// Layout Components
mod pane_divider_test;
mod sidebar_test;
mod stack_test;
mod status_bar_test;

// Color Components
mod color_picker_test;

// Wizard
mod wizard_test;

// Feedback Components
mod progress_test;
mod spinner_test;
mod text_test;
mod toast_test;
mod tooltip_test;

// Overlay Components
mod confirm_dialog_test;
mod popover_test;

// Search & Input
mod search_bar_test;

// Data Display
mod empty_state_test;
mod keyboard_shortcut_label_test;
mod qr_test;

// Theme
mod number_input_test;
mod theme_test;

// Tier 2 Components
mod image_view_test;
mod loading_overlay_test;
mod settings_form_test;
mod split_pane_test;
mod step_indicator_test;

// Thinking Orb (engine golden-vector parity)
mod thinking_orb_parity_test;
// Thinking Orb (API/behavior)
mod thinking_orb_test;

// Tier 3 Components
mod command_palette_test;
mod drag_list_test;
mod notification_test;
mod tag_test;
mod toolbar_test;
mod tree_view_test;
