//! Integration tests for horizontal Slider component
//!
//! Tests the slider component including:
//! - Basic rendering with different sizes
//! - Value changes via scroll wheel
//! - Value changes via click and drag
//! - Keyboard navigation (arrows)
//! - Disabled state
//! - Value clamping at bounds
//! - Callbacks: on_change

mod slider_disabled_view;
mod slider_percentage_view;
mod slider_scroll_wheel_view;
mod slider_test_view;
mod slider_value_change_view;
mod test;
