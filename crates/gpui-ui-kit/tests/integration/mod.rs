//! Integration test utilities for GPUI UI Kit
//!
//! This module provides integration tests for all UI components
//! that verify they can be rendered in actual GPUI windows.

// Component integration tests - Form Controls
mod button_set_test;
mod button_test;
mod checkbox_test;
mod color_picker_test;
#[allow(clippy::arc_with_non_send_sync)]
mod input_test;
mod number_input_test;
mod select_test;
mod slider_test;
mod toggle_test;

// Component integration tests - Display Components
mod alert_test;
mod avatar_test;
mod badge_test;
mod card_test;
mod dialog_test;
mod icon_button_test;

// Component integration tests - Navigation Components
mod accordion_test;
mod breadcrumbs_test;
mod context_menu_test;
mod menu_test;
mod tabs_test;
mod wizard_test;

// Component integration tests - Layout Components
mod pane_divider_drag_test;
mod pane_divider_test;
mod sidebar_test;
mod stack_test;
mod status_bar_test;

// Component integration tests - Feedback Components
mod confirm_dialog_test;
mod empty_state_test;
mod progress_test;
mod spinner_test;
mod text_test;
mod toast_test;
mod tooltip_test;

// Component integration tests - Input Components
mod keyboard_shortcut_label_test;
mod popover_test;
mod search_bar_test;

// Component integration tests - Data Display
mod qr_test;

// Tier 2 integration tests
mod image_view_test;
mod loading_overlay_test;
mod settings_form_test;
mod split_pane_test;
mod step_indicator_test;

// Tier 3 integration tests
mod command_palette_test;
mod drag_list_test;
mod notification_test;
mod tag_test;
mod toolbar_test;
mod tree_view_test;

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // If this compiles, the module structure works
    }
}
