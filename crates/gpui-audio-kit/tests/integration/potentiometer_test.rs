//! Integration tests for Potentiometer component
//!
//! Tests the potentiometer (rotary knob) component including:
//! - Basic rendering with different sizes
//! - Value changes via scroll wheel
//! - Value changes via click (increment behavior)
//! - Double-click to reset
//! - Selected state
//! - Linear vs Logarithmic scales
//! - Disabled state
//! - Callbacks: on_change, on_select, on_reset, on_drag_start

#[path = "potentiometer_test/disabled_pot_test_view.rs"]
mod disabled_pot_test_view;
#[path = "potentiometer_test/pot_drag_test_view.rs"]
mod pot_drag_test_view;
#[path = "potentiometer_test/pot_keyboard_view.rs"]
mod pot_keyboard_view;
#[path = "potentiometer_test/pot_log_scale_view.rs"]
mod pot_log_scale_view;
#[path = "potentiometer_test/pot_percentage_view.rs"]
mod pot_percentage_view;
#[path = "potentiometer_test/pot_reset_test_view.rs"]
mod pot_reset_test_view;
#[path = "potentiometer_test/pot_scroll_wheel_view.rs"]
mod pot_scroll_wheel_view;
#[path = "potentiometer_test/pot_select_test_view.rs"]
mod pot_select_test_view;
#[path = "potentiometer_test/pot_value_change_test_view.rs"]
mod pot_value_change_test_view;
#[path = "potentiometer_test/potentiometer_test_view.rs"]
mod potentiometer_test_view;
#[path = "potentiometer_test/test.rs"]
mod test;
