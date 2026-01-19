//! NumberInput component for numeric value entry
//!
//! A numeric input field with:
//! - Increment/decrement buttons (+ and -)
//! - Direct text editing of the value (click on value to edit)
//! - Keyboard navigation:
//!   - Arrow Up/Right: increase value
//!   - Arrow Down/Left: decrease value
//!   - Enter: confirm edit
//!   - Escape: cancel edit
//! - Scroll wheel adjustment
//! - Configurable step size, min/max bounds
//! - Value formatting (decimals, units)
//!
//! The component handles its own editing state internally - just provide
//! an `on_change` callback to receive value updates.
//!
//! # Thread-Local State Pattern
//!
//! This component uses `thread_local!` storage to persist focus handles and
//! edit state across renders. This is necessary because GPUI's `RenderOnce`
//! components are recreated on each render, but we need state to persist:
//!
//! - **Focus handles**: Must be the same instance across renders or focus is lost
//! - **Edit state**: Cursor position, text, and selection must persist during editing
//!
//! ## Memory Considerations
//!
//! The thread-local `HashMap` entries grow as new element IDs are used and are
//! never automatically cleaned up. For most applications this is fine because:
//! - Element IDs are typically static or part of a bounded set
//! - The stored data is small (FocusHandle, EditState)
//!
//! If you have dynamic element IDs (e.g., from a virtualized list), consider:
//! 1. Using a stable ID scheme that reuses IDs
//! 2. Calling `cleanup_number_input_state(id)` when components are removed
//!
//! ## Cleanup Function
//!
//! To manually clean up state for a removed element:
//! ```rust,ignore
//! cleanup_number_input_state(&element_id);
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// Thread-local registry for focus handles, keyed by element ID.
thread_local! {
    static NUMBER_INPUT_FOCUS_HANDLES: RefCell<HashMap<ElementId, FocusHandle>> = RefCell::new(HashMap::new());
}

// Thread-local registry for edit state, keyed by element ID.
thread_local! {
    static NUMBER_INPUT_EDIT_STATES: RefCell<HashMap<ElementId, Rc<RefCell<NumberEditState>>>> = RefCell::new(HashMap::new());
}

/// Clean up thread-local state for a NumberInput element.
///
/// Call this when removing a NumberInput with a dynamic element ID to prevent
/// memory leaks. For static element IDs, cleanup is not necessary.
///
/// # Example
/// ```rust,ignore
/// // When removing a dynamically-created NumberInput
/// cleanup_number_input_state(&ElementId::Name(format!("input-{}", item_id).into()));
/// ```
pub fn cleanup_number_input_state(id: &ElementId) {
    NUMBER_INPUT_FOCUS_HANDLES.with(|handles| {
        handles.borrow_mut().remove(id);
    });
    NUMBER_INPUT_EDIT_STATES.with(|states| {
        states.borrow_mut().remove(id);
    });
}

/// Internal editing state for the number input
#[derive(Clone, Default)]
struct NumberEditState {
    /// Whether currently editing
    editing: bool,
    /// Current edit text
    text: String,
    /// Cursor position (character index)
    cursor: usize,
    /// Whether all text is selected
    text_selected: bool,
}

impl NumberEditState {
    fn new(value: &str) -> Self {
        Self {
            editing: true,
            text: value.to_string(),
            cursor: value.chars().count(),
            text_selected: true,
        }
    }

    fn select_all(&mut self) {
        self.text_selected = true;
        self.cursor = self.text.chars().count();
    }

    fn do_backspace(&mut self) {
        if self.text_selected {
            self.text.clear();
            self.cursor = 0;
            self.text_selected = false;
        } else if self.cursor > 0 {
            // Find byte position of character before cursor
            // Since we only allow ASCII input, cursor == byte position
            // but we handle it correctly for safety
            let byte_pos = self
                .text
                .char_indices()
                .nth(self.cursor - 1)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let next_byte = self
                .text
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.text.len());
            self.text.replace_range(byte_pos..next_byte, "");
            self.cursor -= 1;
        }
    }

    fn do_delete(&mut self) {
        if self.text_selected {
            self.text.clear();
            self.cursor = 0;
            self.text_selected = false;
        } else {
            let len = self.text.chars().count();
            if self.cursor < len {
                // Find byte positions for character at cursor
                let byte_pos = self
                    .text
                    .char_indices()
                    .nth(self.cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(self.text.len());
                let next_byte = self
                    .text
                    .char_indices()
                    .nth(self.cursor + 1)
                    .map(|(i, _)| i)
                    .unwrap_or(self.text.len());
                self.text.replace_range(byte_pos..next_byte, "");
            }
        }
    }

    fn insert_char(&mut self, ch: char) {
        // Only allow valid numeric characters (all ASCII, so 1 byte each)
        if !ch.is_ascii_digit() && ch != '.' && ch != '-' && ch != '+' {
            return;
        }

        if self.text_selected {
            self.text.clear();
            self.cursor = 0;
            self.text_selected = false;
        }

        // Find byte position for insertion
        let byte_pos = self
            .text
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len());
        self.text.insert(byte_pos, ch);
        self.cursor += 1;
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        self.text_selected = false;
    }

    fn move_right(&mut self) {
        let len = self.text.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
        self.text_selected = false;
    }

    fn move_to_start(&mut self) {
        self.cursor = 0;
        self.text_selected = false;
    }

    fn move_to_end(&mut self) {
        self.cursor = self.text.chars().count();
        self.text_selected = false;
    }
}

/// Theme colors for number input styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct NumberInputTheme {
    /// Background color
    #[theme(default = 0x1e1e1eff, from = background)]
    pub background: Rgba,
    /// Text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text: Rgba,
    /// Button background
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub button_bg: Rgba,
    /// Button hover background
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub button_hover: Rgba,
    /// Button active (pressed) background
    #[theme(default = 0x007accff, from = accent)]
    pub button_active: Rgba,
    /// Button text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub button_text: Rgba,
    /// Border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Border focus color
    #[theme(default = 0x007accff, from = accent)]
    pub border_focus: Rgba,
    /// Label color
    #[theme(default = 0xaaaaaaff, from = text_secondary)]
    pub label: Rgba,
    /// Disabled opacity
    #[theme(default_f32 = 0.5, from_expr = "0.5")]
    pub disabled_opacity: f32,
}

/// Number input size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumberInputSize {
    /// Small size
    Sm,
    /// Medium size (default)
    #[default]
    Md,
    /// Large size
    Lg,
}

impl From<crate::ComponentSize> for NumberInputSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

impl NumberInputSize {
    fn height(&self) -> f32 {
        match self {
            Self::Sm => 24.0,
            Self::Md => 32.0,
            Self::Lg => 40.0,
        }
    }

    fn button_width(&self) -> f32 {
        match self {
            Self::Sm => 20.0,
            Self::Md => 28.0,
            Self::Lg => 36.0,
        }
    }

    fn font_size(&self) -> f32 {
        match self {
            Self::Sm => 11.0,
            Self::Md => 13.0,
            Self::Lg => 15.0,
        }
    }

    fn padding(&self) -> f32 {
        match self {
            Self::Sm => 4.0,
            Self::Md => 8.0,
            Self::Lg => 12.0,
        }
    }
}

/// A numeric input component with increment/decrement buttons
///
/// The component handles its own editing state internally. Just provide
/// an `on_change` callback to receive value updates.
#[derive(IntoElement)]
pub struct NumberInput {
    id: ElementId,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    decimals: usize,
    unit: Option<SharedString>,
    label: Option<SharedString>,
    size: NumberInputSize,
    width: Option<f32>,
    disabled: bool,
    theme: Option<NumberInputTheme>,
    on_change: Option<Box<dyn Fn(f64, &mut Window, &mut App) + 'static>>,
}

impl NumberInput {
    /// Create a new number input with the given ID
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: 0.0,
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            step: 1.0,
            decimals: 0,
            unit: None,
            label: None,
            size: NumberInputSize::default(),
            width: None,
            disabled: false,
            theme: None,
            on_change: None,
        }
    }

    /// Set the current value
    ///
    /// NaN values are clamped to the minimum bound.
    pub fn value(mut self, value: f64) -> Self {
        // Handle NaN by falling back to min (or 0 if min is infinite)
        let value = if value.is_nan() {
            if self.min.is_finite() {
                self.min
            } else if self.max.is_finite() {
                self.max
            } else {
                0.0
            }
        } else {
            value
        };
        self.value = value.clamp(self.min, self.max);
        self
    }

    /// Set the minimum value
    ///
    /// # Panics
    /// Panics if min is NaN
    pub fn min(mut self, min: f64) -> Self {
        assert!(!min.is_nan(), "NumberInput min cannot be NaN");
        self.min = min;
        self
    }

    /// Set the maximum value
    ///
    /// # Panics
    /// Panics if max is NaN
    pub fn max(mut self, max: f64) -> Self {
        assert!(!max.is_nan(), "NumberInput max cannot be NaN");
        self.max = max;
        self
    }

    /// Set both min and max values at once
    ///
    /// # Panics
    /// Panics if min > max or if either value is NaN
    pub fn range(mut self, min: f64, max: f64) -> Self {
        assert!(!min.is_nan(), "NumberInput min cannot be NaN");
        assert!(!max.is_nan(), "NumberInput max cannot be NaN");
        assert!(
            min <= max,
            "NumberInput range invalid: min ({}) > max ({})",
            min,
            max
        );
        self.min = min;
        self.max = max;
        self
    }

    /// Set the step size for increment/decrement
    ///
    /// # Panics
    /// Panics if step is not positive or is NaN
    pub fn step(mut self, step: f64) -> Self {
        assert!(
            step > 0.0 && !step.is_nan(),
            "NumberInput step must be positive, got: {}",
            step
        );
        self.step = step;
        self
    }

    /// Set the number of decimal places to display
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Set the unit suffix (e.g., "Hz", "dB", "%")
    pub fn unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Set the label
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the size variant
    pub fn size(mut self, size: NumberInputSize) -> Self {
        self.size = size;
        self
    }

    /// Set fixed width (optional)
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the theme
    pub fn theme(mut self, theme: NumberInputTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set value change handler (called on button click, scroll, keyboard, or text edit confirm)
    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Format value for display
    fn format_value_str(value: f64, decimals: usize, unit: Option<&SharedString>) -> String {
        let formatted = format!("{:.prec$}", value, prec = decimals);
        if let Some(unit) = unit {
            format!("{} {}", formatted, unit)
        } else {
            formatted
        }
    }

    /// Parse a string to a value, removing unit suffix
    fn parse_value_str(text: &str, unit: Option<&SharedString>, min: f64, max: f64) -> Option<f64> {
        let text = if let Some(unit) = unit {
            text.trim().trim_end_matches(unit.as_ref()).trim()
        } else {
            text.trim()
        };

        text.parse::<f64>().ok().map(|v| v.clamp(min, max))
    }
}

impl RenderOnce for NumberInput {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let default_theme = NumberInputTheme::from(&global_theme);
        let theme = self.theme.clone().unwrap_or(default_theme);

        let height = self.size.height();
        let button_width = self.size.button_width();
        let padding = self.size.padding();
        let disabled = self.disabled;
        let current_value = self.value;
        let min = self.min;
        let max = self.max;
        let step = self.step;
        let decimals = self.decimals;
        let unit_clone = self.unit.clone();

        // Get or create focus handle for this element
        let focus_handle = NUMBER_INPUT_FOCUS_HANDLES.with(|handles| {
            let mut handles = handles.borrow_mut();
            handles
                .entry(self.id.clone())
                .or_insert_with(|| cx.focus_handle())
                .clone()
        });

        // Get or create edit state for this element
        let edit_state = NUMBER_INPUT_EDIT_STATES.with(|states| {
            let mut states = states.borrow_mut();
            states
                .entry(self.id.clone())
                .or_insert_with(|| Rc::new(RefCell::new(NumberEditState::default())))
                .clone()
        });

        // Check if we're focused - editing is only active when focused
        let is_focused = focus_handle.is_focused(_window);

        // If we were editing but lost focus, confirm the edit
        {
            let mut state = edit_state.borrow_mut();
            if state.editing && !is_focused {
                // Parse and confirm the value on focus loss
                if let Some(value) =
                    Self::parse_value_str(&state.text, self.unit.as_ref(), min, max)
                    && let Some(ref handler) = self.on_change
                {
                    handler(value, _window, cx);
                }
                // Clear editing state
                state.editing = false;
                state.text.clear();
                state.text_selected = false;
            }
        }

        // Read current edit state
        let state = edit_state.borrow();
        let editing = state.editing && is_focused; // Only edit when focused
        let text_selected = state.text_selected;
        let edit_text = if editing {
            state.text.clone()
        } else {
            Self::format_value_str(current_value, decimals, unit_clone.as_ref())
        };
        let cursor_pos = state.cursor;
        drop(state);

        // Create unique child IDs based on parent ID
        let parent_id = format!("{:?}", self.id);
        let dec_id = ElementId::Name(SharedString::from(format!("{}-dec", parent_id)));
        let value_id = ElementId::Name(SharedString::from(format!("{}-value", parent_id)));
        let inc_id = ElementId::Name(SharedString::from(format!("{}-inc", parent_id)));

        // Wrap handler in Rc for sharing
        let on_change_rc = self.on_change.map(Rc::new);

        let mut container = div().flex().flex_col().gap_1();

        // Label
        if let Some(label) = self.label {
            container = container.child(
                div()
                    .text_sm()
                    .text_color(theme.label)
                    .font_weight(FontWeight::MEDIUM)
                    .child(label),
            );
        }

        // Input row: [−] [value] [+]
        let mut input_row = div()
            .id(self.id.clone())
            .flex()
            .items_center()
            .h(px(height))
            .rounded_md()
            .border_1()
            .border_color(if editing {
                theme.border_focus
            } else {
                theme.border
            })
            .bg(theme.background)
            .overflow_hidden();

        if let Some(width) = self.width {
            input_row = input_row.w(px(width));
        }

        if disabled {
            input_row = input_row.opacity(theme.disabled_opacity);
        }

        // Decrement button (−)
        let button_bg = theme.button_bg;
        let button_hover = theme.button_hover;
        let button_active = theme.button_active;
        let button_text = theme.button_text;
        let text_color = theme.text;

        let mut dec_button = div()
            .id(dec_id)
            .flex()
            .items_center()
            .justify_center()
            .w(px(button_width))
            .h_full()
            .bg(button_bg)
            .text_color(button_text)
            .font_weight(FontWeight::BOLD)
            .child("−");

        if !disabled {
            dec_button = dec_button
                .cursor_pointer()
                .hover(move |s| s.bg(button_hover))
                .active(move |s| s.bg(button_active));

            if let Some(ref handler_rc) = on_change_rc {
                let handler = handler_rc.clone();
                dec_button = dec_button.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    let new_value = (current_value - step).clamp(min, max);
                    handler(new_value, window, cx);
                });
            }
        } else {
            dec_button = dec_button.cursor_not_allowed();
        }

        input_row = input_row.child(dec_button);

        // Value display / edit field
        // Visual selection highlight: when text_selected is true, show accent background
        let (value_bg, value_text_color) = if editing && text_selected {
            (Some(theme.button_active), rgba(0xffffffff))
        } else {
            (None, text_color)
        };

        // Build display with cursor if editing and not all selected
        let display_element: AnyElement = if editing && !text_selected {
            // Show text with cursor
            let chars: Vec<char> = edit_text.chars().collect();
            let before: String = chars[..cursor_pos].iter().collect();
            let after: String = chars[cursor_pos..].iter().collect();

            div()
                .flex()
                .items_center()
                .child(before)
                .child(
                    div()
                        .w(px(1.0))
                        .h(px(self.size.font_size() + 2.0))
                        .bg(text_color),
                )
                .child(after)
                .into_any_element()
        } else {
            div().child(edit_text.clone()).into_any_element()
        };

        let mut value_field = div()
            .id(value_id)
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .h_full()
            .px(px(padding))
            .text_color(value_text_color)
            .track_focus(&focus_handle)
            .focusable()
            .child(display_element);

        // Apply selection background if selected
        if let Some(bg) = value_bg {
            value_field = value_field.bg(bg);
        }

        // Apply font size
        value_field = value_field.text_size(px(self.size.font_size()));

        if !disabled {
            // Click to start editing / focus
            let edit_state_for_click = edit_state.clone();
            let focus_handle_for_click = focus_handle.clone();
            let formatted_value =
                Self::format_value_str(current_value, decimals, unit_clone.as_ref());

            value_field = value_field.cursor_text().on_mouse_down(
                MouseButton::Left,
                move |event, window, cx| {
                    // Focus the input
                    window.focus(&focus_handle_for_click, cx);

                    let mut state = edit_state_for_click.borrow_mut();

                    // Double-click: select all
                    if event.click_count == 2 {
                        if state.editing {
                            state.select_all();
                        } else {
                            *state = NumberEditState::new(&formatted_value);
                        }
                        drop(state);
                        window.refresh();
                        return;
                    }

                    // Single click: start editing if not already
                    if !state.editing {
                        *state = NumberEditState::new(&formatted_value);
                    } else {
                        // Clear selection on single click while editing
                        state.text_selected = false;
                    }
                },
            );

            // Keyboard handling
            let edit_state_for_key = edit_state.clone();
            let on_change_key = on_change_rc.clone();
            let unit_for_key = unit_clone.clone();

            value_field = value_field.on_key_down(move |event, window, cx| {
                let mut state = edit_state_for_key.borrow_mut();

                if state.editing {
                    match event.keystroke.key.as_str() {
                        "enter" => {
                            // Confirm edit - parse and call on_change
                            let parsed =
                                Self::parse_value_str(&state.text, unit_for_key.as_ref(), min, max);
                            state.editing = false;
                            state.text.clear();
                            state.text_selected = false;
                            drop(state);

                            if let Some(ref handler) = on_change_key
                                && let Some(value) = parsed
                            {
                                handler(value, window, cx);
                            }
                            window.refresh();
                        }
                        "escape" => {
                            // Cancel edit - restore original value
                            state.editing = false;
                            state.text.clear();
                            state.text_selected = false;
                            drop(state);
                            window.refresh();
                        }
                        "backspace" => {
                            state.do_backspace();
                            drop(state);
                            window.refresh();
                        }
                        "delete" => {
                            state.do_delete();
                            drop(state);
                            window.refresh();
                        }
                        "left" => {
                            state.move_left();
                            drop(state);
                            window.refresh();
                        }
                        "right" => {
                            state.move_right();
                            drop(state);
                            window.refresh();
                        }
                        "home" => {
                            state.move_to_start();
                            drop(state);
                            window.refresh();
                        }
                        "end" => {
                            state.move_to_end();
                            drop(state);
                            window.refresh();
                        }
                        _ => {
                            // Character input - use key_char for actual text characters
                            if let Some(text) = event.keystroke.key_char.as_ref()
                                && let Some(ch) = text.chars().next()
                            {
                                state.insert_char(ch);
                                drop(state);
                                window.refresh();
                            }
                        }
                    }
                } else {
                    // Non-editing mode - arrow keys adjust value
                    let new_value = match event.keystroke.key.as_str() {
                        "up" | "right" => Some((current_value + step).clamp(min, max)),
                        "down" | "left" => Some((current_value - step).clamp(min, max)),
                        _ => None,
                    };
                    drop(state);

                    if let Some(v) = new_value
                        && let Some(ref handler) = on_change_key
                    {
                        handler(v, window, cx);
                    }
                }
            });
        }

        input_row = input_row.child(value_field);

        // Increment button (+)
        let mut inc_button = div()
            .id(inc_id)
            .flex()
            .items_center()
            .justify_center()
            .w(px(button_width))
            .h_full()
            .bg(button_bg)
            .text_color(button_text)
            .font_weight(FontWeight::BOLD)
            .child("+");

        if !disabled {
            inc_button = inc_button
                .cursor_pointer()
                .hover(move |s| s.bg(button_hover))
                .active(move |s| s.bg(button_active));

            if let Some(ref handler_rc) = on_change_rc {
                let handler = handler_rc.clone();
                inc_button = inc_button.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    let new_value = (current_value + step).clamp(min, max);
                    handler(new_value, window, cx);
                });
            }
        } else {
            inc_button = inc_button.cursor_not_allowed();
        }

        input_row = input_row.child(inc_button);

        // Note: Scroll wheel handling removed to allow page scrolling.
        // Use +/- buttons or keyboard to adjust value.

        container.child(input_row)
    }
}
