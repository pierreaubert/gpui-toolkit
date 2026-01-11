//! Integration test utilities for GPUI UI Kit
//!
//! This module provides integration tests for all UI components
//! that verify they can be rendered in actual GPUI windows.

// Component integration tests - Form Controls
mod button_set_test;
mod button_test;
mod checkbox_test;
mod color_picker_test;
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
mod menu_test;
mod tabs_test;
mod wizard_test;

// Component integration tests - Layout Components
mod pane_divider_test;
mod stack_test;

// Component integration tests - Audio/Control Components
mod potentiometer_test;
mod vertical_slider_test;
mod volume_knob_test;

// Component integration tests - Feedback Components
mod progress_test;
mod spinner_test;
mod text_test;
mod toast_test;
mod tooltip_test;

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // If this compiles, the module structure works
    }
}
