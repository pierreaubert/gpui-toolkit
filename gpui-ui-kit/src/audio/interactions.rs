//! Shared interaction handling for audio control components
//!
//! This module provides common interaction patterns for sliders, knobs, and potentiometers:
//! - Keyboard navigation (arrows, page up/down, home/end, escape)
//! - Mouse drag with delta-based value changes
//! - Scroll wheel adjustment (with shift for fine control)
//! - Double-click to reset
//!
//! The drag state is stored in thread-local storage to survive component re-renders
//! triggered by `cx.notify()`.

use crate::scale::Scale;
use gpui::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Drag state that persists across re-renders
#[derive(Clone, Copy, Debug)]
pub struct DragState {
    pub start_pos: f32,   // Starting position (y for vertical, x for horizontal)
    pub start_value: f64, // Value when drag started
}

thread_local! {
    static DRAG_STATES: RefCell<HashMap<String, DragState>> = RefCell::new(HashMap::new());
}

/// Store drag state for an element (call on mouse_down)
pub fn store_drag_state(element_key: &str, start_pos: f32, start_value: f64) {
    DRAG_STATES.with(|states| {
        states.borrow_mut().insert(
            element_key.to_string(),
            DragState {
                start_pos,
                start_value,
            },
        );
    });
}

/// Get drag state for an element (call on mouse_move)
pub fn get_drag_state(element_key: &str) -> Option<DragState> {
    DRAG_STATES.with(|states| states.borrow().get(element_key).copied())
}

/// Clear drag state for an element (call on mouse_up)
pub fn clear_drag_state(element_key: &str) {
    DRAG_STATES.with(|states| {
        states.borrow_mut().remove(element_key);
    });
}

/// Orientation for drag calculations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragOrientation {
    /// Vertical drag (sliders) - up increases value
    Vertical,
    /// Horizontal drag - right increases value
    Horizontal,
    /// Circular/rotational drag (knobs) - up or right increases value
    Rotational,
}

/// Configuration for interaction handlers
#[derive(Clone)]
pub struct InteractionConfig {
    pub min: f64,
    pub max: f64,
    pub scale: Scale,
    pub orientation: DragOrientation,
    pub track_size: f32, // Height for vertical, width for horizontal
    /// Enable media key support (for volume controls)
    pub media_keys: bool,
}

impl InteractionConfig {
    pub fn vertical(min: f64, max: f64, scale: Scale, track_height: f32) -> Self {
        Self {
            min,
            max,
            scale,
            orientation: DragOrientation::Vertical,
            track_size: track_height,
            media_keys: false,
        }
    }

    pub fn horizontal(min: f64, max: f64, scale: Scale, track_width: f32) -> Self {
        Self {
            min,
            max,
            scale,
            orientation: DragOrientation::Horizontal,
            track_size: track_width,
            media_keys: false,
        }
    }

    pub fn rotational(min: f64, max: f64, scale: Scale, drag_distance: f32) -> Self {
        Self {
            min,
            max,
            scale,
            orientation: DragOrientation::Rotational,
            track_size: drag_distance,
            media_keys: false,
        }
    }

    pub fn with_media_keys(mut self) -> Self {
        self.media_keys = true;
        self
    }
}

/// Shared value tracker for event handlers
///
/// This is needed because events may fire faster than re-renders,
/// and multiple handlers need to see the same up-to-date value.
pub type ValueTracker = Rc<std::cell::Cell<f64>>;

/// Create a new value tracker initialized with a value
pub fn value_tracker(initial: f64) -> ValueTracker {
    Rc::new(std::cell::Cell::new(initial))
}

/// Handle keyboard events for value adjustment
///
/// Returns the new value if the key was handled, None otherwise.
pub fn handle_keyboard(
    key: &str,
    modifiers: &Modifiers,
    current_value: f64,
    config: &InteractionConfig,
) -> Option<f64> {
    let scale = config.scale;
    let min = config.min;
    let max = config.max;

    // Determine step size based on modifiers
    // Shift = Fine (1%)
    // Ctrl/Cmd = Large (10%)
    // Default = Normal (5%)
    let step_size = if modifiers.shift {
        0.01
    } else if modifiers.control || modifiers.platform {
        0.10
    } else {
        0.05
    };

    match key {
        // Standard navigation keys
        "up" | "right" => Some(scale.step_value(current_value, min, max, 1.0, step_size)),
        "down" | "left" => Some(scale.step_value(current_value, min, max, -1.0, step_size)),
        "pageup" => Some(scale.step_value(current_value, min, max, 1.0, 0.10)),
        "pagedown" => Some(scale.step_value(current_value, min, max, -1.0, 0.10)),
        "home" => Some(min),
        "end" => Some(max),
        _ => {
            // Media keys (only if enabled)
            if config.media_keys {
                match key {
                    "audiomute" => None, // Handled separately by mute toggle
                    "audiolowervolume" => {
                        Some(scale.step_value(current_value, min, max, -1.0, 0.05))
                    }
                    "audioraisevolume" => {
                        Some(scale.step_value(current_value, min, max, 1.0, 0.05))
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
    }
}

/// Handle scroll wheel events for value adjustment
///
/// Returns the new value based on scroll delta.
/// Shift modifier enables fine control (0.5% vs 5%).
pub fn handle_scroll(
    delta: &ScrollDelta,
    modifiers: &Modifiers,
    current_value: f64,
    config: &InteractionConfig,
) -> Option<f64> {
    let (delta_x, delta_y): (f32, f32) = match delta {
        ScrollDelta::Pixels(point) => (point.x.into(), point.y.into()),
        ScrollDelta::Lines(point) => (point.x, point.y),
    };

    // Determine primary scroll direction based on orientation
    let scroll_delta = match config.orientation {
        DragOrientation::Vertical | DragOrientation::Rotational => {
            if delta_y.abs() > 0.0001 {
                delta_y
            } else if delta_x.abs() > 0.0001 {
                delta_x
            } else {
                return None;
            }
        }
        DragOrientation::Horizontal => {
            if delta_x.abs() > 0.0001 {
                -delta_x // Positive x = right = increase
            } else if delta_y.abs() > 0.0001 {
                delta_y
            } else {
                return None;
            }
        }
    };

    // Scroll up/left = negative delta = increase value
    let direction = if scroll_delta < 0.0 { 1.0 } else { -1.0 };
    let step_size = if modifiers.shift { 0.005 } else { 0.05 };

    Some(
        config
            .scale
            .step_value(current_value, config.min, config.max, direction, step_size),
    )
}

/// Handle drag movement for value adjustment
///
/// Returns the new value based on drag delta from start position.
pub fn handle_drag(
    current_pos: f32,
    drag_state: &DragState,
    config: &InteractionConfig,
) -> Option<f64> {
    let delta = match config.orientation {
        DragOrientation::Vertical => {
            // Vertical: dragging up (negative delta) increases value
            drag_state.start_pos - current_pos
        }
        DragOrientation::Horizontal => {
            // Horizontal: dragging right (positive delta) increases value
            current_pos - drag_state.start_pos
        }
        DragOrientation::Rotational => {
            // Rotational: up or right increases (use vertical movement primarily)
            drag_state.start_pos - current_pos
        }
    };

    // Minimum movement threshold to avoid spurious updates on click
    if delta.abs() < 2.0 {
        return None;
    }

    // Map pixel delta to normalized change
    let delta_norm = (delta / config.track_size) as f64;

    Some(config.scale.step_value(
        drag_state.start_value,
        config.min,
        config.max,
        delta_norm,
        1.0,
    ))
}
