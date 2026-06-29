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

use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState};
use crate::theme::ThemeExt;
use gpui::prelude::{
    FluentBuilder, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, AppContext, Bounds, ClipboardItem, Context, DispatchPhase, Element, ElementId,
    Entity, FocusHandle, FontWeight, GlobalElementId, InspectorElementId, KeyDownEvent, LayoutId,
    MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Render, ScrollDelta, ScrollWheelEvent,
    SharedString, Subscription, WeakEntity, Window, div, px, rgba,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    static NUMBER_INPUT_FOCUS_HANDLES: RefCell<HashMap<ElementId, FocusHandle>> = RefCell::new(HashMap::new());
}
thread_local! {
    static NUMBER_INPUT_EDIT_STATES: RefCell<HashMap<ElementId, Rc<RefCell<NumberEditState>>>> = RefCell::new(HashMap::new());
}
thread_local! {
    static NUMBER_INPUT_FOCUS_SUBS: RefCell<HashMap<ElementId, Subscription>> = RefCell::new(HashMap::new());
}
thread_local! {
    // Cached render entities so repeated renders reuse the same GPUI entity.
    // Stored as weak references so the entities can be dropped when no longer
    // referenced by the element tree, avoiding leaked-handle panics in tests.
    static NUMBER_INPUT_ENTITIES: RefCell<HashMap<ElementId, WeakEntity<NumberInputEntity>>> =
        RefCell::new(HashMap::new());
}

mod misc;
mod number_edit_state;
mod number_input_size;
mod types;

use misc::keystroke_to_char;
pub use misc::{cleanup_number_input_state, is_number_input_editing};
use number_edit_state::NumberEditState;
pub use number_input_size::NumberInputSize;
pub use types::NumberInputTheme;

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
    on_change: Option<Rc<dyn Fn(f64, &mut Window, &mut App) + 'static>>,
    focus_handle: Option<FocusHandle>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
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
            focus_handle: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the focus handle (optional - one is created internally if not provided)
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set the current value
    ///
    /// NaN values are clamped to the minimum bound.
    pub fn value(mut self, value: f64) -> Self {
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
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Set an explicit ARIA label
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Spinbutton)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Format value for display (test helper).
    #[cfg(test)]
    fn format_value_str(value: f64, decimals: usize, unit: Option<&SharedString>) -> String {
        let formatted = format!("{:.prec$}", value, prec = decimals);
        if let Some(unit) = unit {
            format!("{} {}", formatted, unit)
        } else {
            formatted
        }
    }

    /// Parse a string to a value, removing unit suffix.
    fn parse_value_str(text: &str, unit: Option<&SharedString>, min: f64, max: f64) -> Option<f64> {
        let text = if let Some(unit) = unit {
            text.trim().trim_end_matches(unit.as_ref()).trim()
        } else {
            text.trim()
        };

        text.parse::<f64>().ok().map(|v| v.clamp(min, max))
    }
}

impl NumberInputEntity {
    fn emit_change(&mut self, value: f64, window: &mut Window, cx: &mut Context<Self>) {
        self.props.value = value;
        let handler = self.props.on_change.clone();
        if let Some(handler) = handler {
            handler(value, window, cx);
        }
        cx.notify();
        window.refresh();
    }

    fn handle_dec_click(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        window.blur();
        let new_value = (self.props.value - self.props.step).clamp(self.props.min, self.props.max);
        self.emit_change(new_value, window, _cx);
    }

    fn handle_inc_click(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        window.blur();
        let new_value = (self.props.value + self.props.step).clamp(self.props.min, self.props.max);
        self.emit_change(new_value, window, _cx);
    }

    fn handle_value_click(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // The formatted value is recomputed on each render and cached in the
        // edit state, so read it from there.
        let formatted_value = self.edit_state.borrow_mut().format_value_str(
            self.props.value,
            self.props.decimals,
            self.props.unit.as_ref(),
        );

        window.focus(&self.focus_handle, _cx);

        let mut state = self.edit_state.borrow_mut();

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

        if !state.editing {
            *state = NumberEditState::new(&formatted_value);
        } else {
            state.text_selected = false;
        }
        drop(state);
        window.refresh();
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        _cx.stop_propagation();

        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let cmd = event.keystroke.modifiers.platform;
        let alt = event.keystroke.modifiers.alt;

        let mut state = self.edit_state.borrow_mut();

        if state.editing {
            if matches!(key, "up" | "down") {
                let new_value = if key == "up" {
                    (self.props.value + self.props.step).clamp(self.props.min, self.props.max)
                } else {
                    (self.props.value - self.props.step).clamp(self.props.min, self.props.max)
                };
                drop(state);
                self.emit_change(new_value, window, _cx);
                return;
            }

            if cmd || (ctrl && matches!(key, "c" | "x" | "v" | "a")) {
                match key {
                    "a" => {
                        state.select_all();
                        drop(state);
                        window.refresh();
                        return;
                    }
                    "c" => {
                        if let Some(selected) = state.get_selected_text() {
                            drop(state);
                            _cx.write_to_clipboard(ClipboardItem::new_string(selected));
                        }
                        return;
                    }
                    "x" => {
                        if let Some(selected) = state.get_selected_text() {
                            _cx.write_to_clipboard(ClipboardItem::new_string(selected));
                            state.delete_selected();
                            drop(state);
                            window.refresh();
                        }
                        return;
                    }
                    "v" => {
                        if let Some(clipboard) = _cx.read_from_clipboard()
                            && let Some(paste_text) = clipboard.text()
                        {
                            state.insert_str(&paste_text);
                            drop(state);
                            window.refresh();
                        }
                        return;
                    }
                    _ => {}
                }
            }

            if alt {
                match key {
                    "backspace" => {
                        state.kill_word_backward();
                        drop(state);
                        window.refresh();
                        return;
                    }
                    "d" => {
                        state.kill_word_forward();
                        drop(state);
                        window.refresh();
                        return;
                    }
                    _ => {}
                }
            }

            if ctrl {
                match key {
                    "a" => state.move_to_start(),
                    "e" => state.move_to_end(),
                    "k" => state.kill_to_end(),
                    "u" => state.kill_to_start(),
                    "w" => state.kill_word_backward(),
                    "h" => state.do_backspace(),
                    "d" => state.do_delete(),
                    "f" => state.move_right(),
                    "b" => state.move_left(),
                    "y" => {
                        if let Some(clipboard) = _cx.read_from_clipboard()
                            && let Some(paste_text) = clipboard.text()
                        {
                            state.insert_str(&paste_text);
                        }
                    }
                    _ => {}
                }
                drop(state);
                window.refresh();
                return;
            }

            match key {
                "enter" => {
                    let parsed = NumberInput::parse_value_str(
                        &state.text,
                        state.last_unit(),
                        self.props.min,
                        self.props.max,
                    );
                    state.editing = false;
                    state.text.clear();
                    state.text_selected = false;
                    drop(state);

                    window.blur();

                    if let Some(value) = parsed {
                        self.emit_change(value, window, _cx);
                    }
                }
                "escape" => {
                    state.editing = false;
                    state.text.clear();
                    state.text_selected = false;
                    drop(state);
                    window.blur();
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
                    if let Some(ch) = keystroke_to_char(&event.keystroke) {
                        state.insert_char(ch);
                        drop(state);
                        window.refresh();
                    }
                }
            }
        } else {
            let new_value = match key {
                "up" | "right" => {
                    Some((self.props.value + self.props.step).clamp(self.props.min, self.props.max))
                }
                "down" | "left" => {
                    Some((self.props.value - self.props.step).clamp(self.props.min, self.props.max))
                }
                _ => None,
            };
            drop(state);

            if let Some(value) = new_value {
                self.emit_change(value, window, _cx);
            }
        }
    }

    fn handle_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();

        let delta_y: f32 = match event.delta {
            ScrollDelta::Pixels(point) => point.y.into(),
            ScrollDelta::Lines(point) => point.y,
        };

        if delta_y.abs() < 0.0001 {
            return;
        }

        let direction = if delta_y < 0.0 { 1.0 } else { -1.0 };
        let new_value =
            (self.props.value + self.props.step * direction).clamp(self.props.min, self.props.max);
        self.emit_change(new_value, window, cx);
    }

    fn handle_blur(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        let mut state = self.edit_state.borrow_mut();
        if state.editing {
            let parsed = NumberInput::parse_value_str(
                &state.text,
                state.last_unit(),
                self.props.min,
                self.props.max,
            );
            state.editing = false;
            state.text.clear();
            state.text_selected = false;
            drop(state);

            if let Some(value) = parsed {
                self.emit_change(value, window, _cx);
            }
        }
    }

    fn set_hovered_dec(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if self.hovered_dec != hovered {
            self.hovered_dec = hovered;
            cx.notify();
        }
    }

    fn set_hovered_inc(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if self.hovered_inc != hovered {
            self.hovered_inc = hovered;
            cx.notify();
        }
    }
}

impl Render for NumberInputEntity {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let props = &self.props;

        // Register in accessibility tree
        let effective_label = props
            .aria_label
            .clone()
            .or_else(|| self.label.clone())
            .unwrap_or_default();
        cx.register_accessible(AccessibilityNode {
            element_id: props.id.clone(),
            label: effective_label,
            props: AriaProps::with_role(props.aria_role.unwrap_or(AriaRole::Spinbutton))
                .maybe_state(props.disabled, AriaState::Disabled)
                .value_range(props.value, props.min, props.max),
        });

        let global_theme = cx.theme();
        let default_theme = NumberInputTheme::from(global_theme);
        let theme = props.theme.as_ref().unwrap_or(&default_theme);

        let height = props.size.height();
        let button_width = props.size.button_width();
        let padding = props.size.padding();
        let disabled = props.disabled;
        let current_value = props.value;
        let decimals = props.decimals;
        let unit = &props.unit;

        // Format the value once and reuse it for display and click-to-edit state.
        let formatted_value =
            self.edit_state
                .borrow_mut()
                .format_value_str(current_value, decimals, unit.as_ref());

        let state = self.edit_state.borrow();
        let editing = state.editing;
        let text_selected = state.text_selected;
        let edit_text: SharedString = if editing {
            state.text.clone().into()
        } else {
            formatted_value.clone()
        };
        let cursor_pos = state.cursor;
        drop(state);

        let dec_id = ElementId::from((props.id.clone(), "dec"));
        let value_id = ElementId::from((props.id.clone(), "value"));
        let inc_id = ElementId::from((props.id.clone(), "inc"));
        let input_debug_id = props.id.to_string();
        let dec_debug_id = dec_id.to_string();
        let value_debug_id = value_id.to_string();
        let inc_debug_id = inc_id.to_string();

        let mut container = div().flex().flex_col().gap_1();

        // Label (cached in the entity, updated only when the prop changes)
        if let Some(label) = &self.label {
            container = container.child(
                div()
                    .text_sm()
                    .text_color(theme.label)
                    .font_weight(FontWeight::MEDIUM)
                    .child(label.clone()),
            );
        }

        // Input row: [−] [value] [+]
        let mut input_row = div()
            .id(props.id.clone())
            .debug_selector(move || input_debug_id)
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

        if let Some(width) = props.width {
            input_row = input_row.w(px(width));
        }

        if disabled {
            input_row = input_row.opacity(theme.disabled_opacity);
        } else {
            input_row = input_row.on_scroll_wheel(cx.listener(
                |this, event: &ScrollWheelEvent, window, cx| {
                    this.handle_scroll_wheel(event, window, cx);
                },
            ));
        }

        let button_bg = theme.button_bg;
        let button_hover = theme.button_hover;
        let button_active = theme.button_active;
        let button_text = theme.button_text;
        let text_color = theme.text;

        // Decrement button (−)
        let mut dec_button = div()
            .id(dec_id)
            .debug_selector(move || dec_debug_id)
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
                .when(self.hovered_dec, |s| s.bg(button_hover))
                .active(|s| s.bg(button_active))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _event: &MouseUpEvent, window, cx| {
                        this.handle_dec_click(window, cx);
                    }),
                )
                .on_hover(cx.listener(
                    |this: &mut NumberInputEntity, hovered: &bool, _window, cx| {
                        this.set_hovered_dec(*hovered, cx);
                    },
                ));
        } else {
            dec_button = dec_button.cursor_not_allowed();
        }

        input_row = input_row.child(dec_button);

        // Value display / edit field
        let (value_bg, value_text_color) = if editing && text_selected {
            (Some(theme.button_active), rgba(0xffffffff))
        } else {
            (None, text_color)
        };

        let display_element: AnyElement = if editing && !text_selected {
            let char_width = 8.0_f32;
            let cursor_left = char_width * cursor_pos as f32;

            div()
                .relative()
                .flex()
                .items_center()
                .child(edit_text.clone())
                .child(
                    div()
                        .absolute()
                        .left(px(cursor_left))
                        .top_0()
                        .bottom_0()
                        .w(px(1.0))
                        .bg(text_color),
                )
                .into_any_element()
        } else {
            div().child(edit_text.clone()).into_any_element()
        };

        let mut value_field = div()
            .id(value_id)
            .debug_selector(move || value_debug_id)
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .h_full()
            .px(px(padding))
            .text_color(value_text_color)
            .track_focus(&self.focus_handle)
            .focusable()
            .child(display_element);

        if let Some(bg) = value_bg {
            value_field = value_field.bg(bg);
        }

        value_field = value_field.text_size(px(props.size.font_size()));

        if !disabled {
            value_field = value_field
                .cursor_text()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseDownEvent, window, cx| {
                        this.handle_value_click(event, window, cx);
                    }),
                )
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    this.handle_key_down(event, window, cx);
                }));
        }

        input_row = input_row.child(value_field);

        // Increment button (+)
        let mut inc_button = div()
            .id(inc_id)
            .debug_selector(move || inc_debug_id)
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
                .when(self.hovered_inc, |s| s.bg(button_hover))
                .active(|s| s.bg(button_active))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _event: &MouseUpEvent, window, cx| {
                        this.handle_inc_click(window, cx);
                    }),
                )
                .on_hover(cx.listener(
                    |this: &mut NumberInputEntity, hovered: &bool, _window, cx| {
                        this.set_hovered_inc(*hovered, cx);
                    },
                ));
        } else {
            inc_button = inc_button.cursor_not_allowed();
        }

        input_row = input_row.child(inc_button);

        container.child(input_row)
    }
}

/// Internal entity that renders a [`NumberInput`] with stable identity across frames.
pub struct NumberInputEntity {
    props: NumberInput,
    focus_handle: FocusHandle,
    edit_state: Rc<RefCell<NumberEditState>>,
    hovered_dec: bool,
    hovered_inc: bool,
    label: Option<SharedString>,
}

struct NumberInputElement {
    child: AnyElement,
    focus_handle: FocusHandle,
    entity: Entity<NumberInputEntity>,
}

impl Element for NumberInputElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        window.set_focus_handle(&self.focus_handle, cx);
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let entity = self.entity.clone();
        let focus_handle = self.focus_handle.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase == DispatchPhase::Capture
                && focus_handle.is_focused(window)
                && !bounds.contains(&event.position)
            {
                entity.update(cx, |model, cx| {
                    model.handle_blur(window, cx);
                });
            }
        });
        self.child.paint(window, cx);
    }
}

impl IntoElement for NumberInputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl RenderOnce for NumberInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let focus_handle = self.focus_handle.clone().unwrap_or_else(|| {
            NUMBER_INPUT_FOCUS_HANDLES.with(|handles| {
                let mut handles = handles.borrow_mut();
                handles
                    .entry(id.clone())
                    .or_insert_with(|| cx.focus_handle())
                    .clone()
            })
        });
        let edit_state = NUMBER_INPUT_EDIT_STATES.with(|states| {
            let mut states = states.borrow_mut();
            states
                .entry(id.clone())
                .or_insert_with(|| Rc::new(RefCell::new(NumberEditState::default())))
                .clone()
        });

        let entity: Entity<NumberInputEntity> = NUMBER_INPUT_ENTITIES.with(|map| {
            let mut map = map.borrow_mut();
            if let Some(weak) = map.get(&id)
                && let Some(entity) = weak.upgrade()
            {
                return entity;
            }
            let entity = cx.new(|_cx| NumberInputEntity {
                props: NumberInput::new(id.clone()),
                focus_handle: focus_handle.clone(),
                edit_state: edit_state.clone(),
                hovered_dec: false,
                hovered_inc: false,
                label: None,
            });
            map.insert(id.clone(), entity.downgrade());
            entity
        });

        // Register a focus-out subscription once per element id. The
        // subscription weakly references the entity so it can call the current
        // blur handler without capturing a fresh closure each render.
        NUMBER_INPUT_FOCUS_SUBS.with(|subs| {
            let mut subs = subs.borrow_mut();
            if !subs.contains_key(&id) {
                let entity_weak = entity.downgrade();
                let sub = window.on_focus_out(&focus_handle, cx, move |_event, window, cx| {
                    if let Some(entity) = entity_weak.upgrade() {
                        entity.update(cx, |model, cx| {
                            model.handle_blur(window, cx);
                        });
                    }
                });
                subs.insert(id.clone(), sub);
            }
        });

        entity.update(cx, |model, _cx| {
            if model.props.label != self.label {
                model.label = self.label.clone();
            }
            model.focus_handle = focus_handle.clone();
            model.edit_state = edit_state;
            model.props = self;
        });
        NumberInputElement {
            child: entity.clone().into_any_element(),
            focus_handle,
            entity,
        }
    }
}

#[cfg(test)]
mod tests;
