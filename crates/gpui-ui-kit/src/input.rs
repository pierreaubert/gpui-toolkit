//! Input component
//!
//! Text input field with optional label, placeholder, and validation.
//!
//! Features:
//! - Full keyboard text editing support (self-contained)
//! - Click to focus and start editing
//! - Enter to confirm, Escape to cancel
//! - Cursor navigation and text selection
//! - Mouse drag to select text, double-click to select all
//! - Clipboard support: Cmd+C (copy), Cmd+X (cut), Cmd+V (paste), Cmd+A (select all)
//! - Emacs-style keybindings (Ctrl+A/E/K/U/W/H/D/F/B)
//! - Disabled and readonly states
//!
//! # Simple Usage
//!
//! The Input component handles all focus and keyboard events internally.
//! Just provide callbacks for changes:
//!
//! ```ignore
//! Input::new("my-input")
//!     .value(current_value)
//!     .placeholder("Enter text...")
//!     .on_change(|new_value, _window, _cx| {
//!         println!("Value changed to: {}", new_value);
//!     })
//!     .on_text_change(|text, _window, _cx| {
//!         // Called on every keystroke (optional, for live updates)
//!         println!("Current text: {}", text);
//!     })
//! ```
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
//! 2. Calling `cleanup_input_state(id)` when components are removed
//!
//! ## Cleanup Function
//!
//! To manually clean up state for a removed element:
//! ```rust,ignore
//! cleanup_input_state(&element_id);
//! ```

use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{
    FluentBuilder, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, AppContext, Bounds, ClipboardItem, Context, DispatchPhase, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, FocusHandle, FontWeight, GlobalElementId,
    InspectorElementId, KeyDownEvent, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Render, Rgba, SharedString, Subscription, UTF16Selection, WeakEntity,
    Window, div, px,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

thread_local! {
    static FOCUS_HANDLES: RefCell<HashMap<ElementId, FocusHandle>> = RefCell::new(HashMap::new());
}
thread_local! {
    static EDIT_STATES: RefCell<HashMap<ElementId, Rc<RefCell<EditState>>>> = RefCell::new(HashMap::new());
}
thread_local! {
    static TEXT_ORIGINS: RefCell<HashMap<ElementId, f32>> = RefCell::new(HashMap::new());
}
thread_local! {
    static FOCUS_SUBS: RefCell<HashMap<ElementId, Subscription>> = RefCell::new(HashMap::new());
}
thread_local! {
    // Cached render entities so repeated renders reuse the same GPUI entity.
    // Stored as weak references so the entities can be dropped when no longer
    // referenced by the element tree, avoiding leaked-handle panics in tests.
    static INPUT_ENTITIES: RefCell<HashMap<ElementId, WeakEntity<InputEntity>>> =
        RefCell::new(HashMap::new());
}

mod cleanup;
#[cfg(not(feature = "bench"))]
mod edit_state;
#[cfg(feature = "bench")]
pub mod edit_state;
mod input_size;
mod misc;
mod types;

pub use cleanup::{cleanup_input_state, cleanup_stale_input_states};
use edit_state::EditState;
pub use input_size::InputSize;
use misc::keystroke_to_char;
pub use misc::{clear_all_input_states, input_state_count, is_input_editing};
pub use types::{InputTheme, InputVariant};

/// The current native text selection, expressed in Unicode scalar indices.
///
/// Callers receive only positions, never the selected text. This makes the
/// callback safe to use with password inputs and protocol traces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputSelection {
    pub start: usize,
    pub end: usize,
    pub reversed: bool,
}

/// A text input component with full keyboard editing support
///
/// The Input handles all focus and keyboard events internally.
/// Parent components only need to provide callbacks for value changes.
pub struct Input {
    id: ElementId,
    value: SharedString,
    placeholder: Option<SharedString>,
    label: Option<SharedString>,
    size: InputSize,
    variant: InputVariant,
    disabled: bool,
    readonly: bool,
    password: bool,
    error: Option<SharedString>,
    icon_left: Option<SharedString>,
    icon_right: Option<SharedString>,
    bg_color: Option<Rgba>,
    text_color: Option<Rgba>,
    border_color: Option<Rgba>,
    placeholder_color: Option<Rgba>,
    /// Called when value is confirmed (Enter pressed)
    on_change: Option<Rc<dyn Fn(&str, &mut Window, &mut App) + 'static>>,
    /// Called when editing starts (click on input)
    on_edit_start: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    /// Called when editing ends (Enter = Some(value), Escape = None)
    on_edit_end: Option<Rc<dyn Fn(Option<String>, &mut Window, &mut App) + 'static>>,
    /// Called on every text change during editing (for live updates)
    on_text_change: Option<Rc<dyn Fn(&str, &mut Window, &mut App) + 'static>>,
    /// Called when the cursor or selected range changes. The callback exposes
    /// positions only, never the selected value.
    on_selection_change: Option<Rc<dyn Fn(InputSelection, &mut Window, &mut App) + 'static>>,
    /// Focus handle for this input
    focus_handle: Option<FocusHandle>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Input");
        debug
            .field("id", &self.id)
            .field("placeholder", &self.placeholder)
            .field("label", &self.label)
            .field("size", &self.size)
            .field("variant", &self.variant)
            .field("disabled", &self.disabled)
            .field("readonly", &self.readonly)
            .field("password", &self.password);
        if self.password {
            debug
                .field("value", &"<redacted>")
                .field("value_char_count", &self.value.chars().count());
        } else {
            debug.field("value", &self.value);
        }
        debug.finish_non_exhaustive()
    }
}

impl Input {
    /// Create a new input
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: "".into(),
            placeholder: None,
            label: None,
            size: InputSize::default(),
            variant: InputVariant::default(),
            disabled: false,
            readonly: false,
            password: false,
            error: None,
            icon_left: None,
            icon_right: None,
            bg_color: None,
            text_color: None,
            border_color: None,
            placeholder_color: None,
            on_change: None,
            on_edit_start: None,
            on_edit_end: None,
            on_text_change: None,
            on_selection_change: None,
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

    /// Set the input value
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    /// Set placeholder text
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set label text
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set input size
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    /// Set input variant
    pub fn variant(mut self, variant: InputVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set readonly state
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Mask the rendered value while preserving normal text editing and change
    /// callbacks. The native accessibility value is masked as well.
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Set error message
    pub fn error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Set left icon
    pub fn icon_left(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon_left = Some(icon.into());
        self
    }

    /// Set right icon
    pub fn icon_right(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon_right = Some(icon.into());
        self
    }

    /// Set background color
    pub fn bg_color(mut self, color: impl Into<Rgba>) -> Self {
        self.bg_color = Some(color.into());
        self
    }

    /// Set text color
    pub fn text_color(mut self, color: impl Into<Rgba>) -> Self {
        self.text_color = Some(color.into());
        self
    }

    /// Set border color
    pub fn border_color(mut self, color: impl Into<Rgba>) -> Self {
        self.border_color = Some(color.into());
        self
    }

    /// Set placeholder color
    pub fn placeholder_color(mut self, color: impl Into<Rgba>) -> Self {
        self.placeholder_color = Some(color.into());
        self
    }

    /// Set change handler (called when input value is confirmed with Enter)
    pub fn on_change(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Set edit start handler (called when user clicks on input to edit)
    pub fn on_edit_start(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_edit_start = Some(Rc::new(handler));
        self
    }

    /// Set edit end handler (called when user confirms or cancels edit)
    /// The `Option<String>` is `Some(value)` if confirmed, `None` if cancelled
    pub fn on_edit_end(
        mut self,
        handler: impl Fn(Option<String>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_edit_end = Some(Rc::new(handler));
        self
    }

    /// Set text change handler (called on every keystroke during editing)
    pub fn on_text_change(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_text_change = Some(Rc::new(handler));
        self
    }

    /// Set a selection-change handler. Positions use Unicode scalar indices
    /// and are suitable for serializing into application events.
    pub fn on_selection_change(
        mut self,
        handler: impl Fn(InputSelection, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_selection_change = Some(Rc::new(handler));
        self
    }

    /// Set an explicit ARIA label
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Textbox)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }
}

impl InputEntity {
    fn cached_password_mask(
        cache: &RefCell<Option<(usize, SharedString)>>,
        source: &str,
    ) -> SharedString {
        let char_count = source.chars().count();
        if let Some((cached_char_count, mask)) = cache.borrow().as_ref()
            && *cached_char_count == char_count
        {
            return mask.clone();
        }
        let mask: SharedString = "•".repeat(char_count).into();
        // Deliberately retain only the public character count and mask. A
        // password must not be duplicated into a render/accessibility cache.
        *cache.borrow_mut() = Some((char_count, mask.clone()));
        mask
    }

    fn char_to_utf16(text: &str, char_idx: usize) -> usize {
        text.chars().take(char_idx).map(char::len_utf16).sum()
    }

    fn utf16_to_char(text: &str, utf16_idx: usize) -> usize {
        let mut utf16_pos = 0;
        for (char_idx, ch) in text.chars().enumerate() {
            let next = utf16_pos + ch.len_utf16();
            if utf16_idx < next {
                return char_idx;
            }
            utf16_pos = next;
        }
        text.chars().count()
    }

    /// Return the two UTF-8 byte boundaries in one character scan. Editing
    /// rendering needs several character-indexed pieces per frame; deriving
    /// both boundaries together avoids the old repeated O(n) `nth` walks.
    fn char_range_byte_offsets(text: &str, start: usize, end: usize) -> (usize, usize) {
        let mut start_byte = text.len();
        let mut end_byte = text.len();
        for (char_index, (byte_index, _)) in text.char_indices().enumerate() {
            if char_index == start {
                start_byte = byte_index;
            }
            if char_index == end {
                end_byte = byte_index;
                break;
            }
        }
        (start_byte, end_byte)
    }

    fn replace_char_range(state: &mut EditState, range: Range<usize>, text: &str) {
        state.begin_text_edit();
        let char_len = state.text.chars().count();
        let start = range.start.min(char_len);
        let end = range.end.min(char_len).max(start);
        let start_byte = state
            .text
            .char_indices()
            .nth(start)
            .map(|(idx, _)| idx)
            .unwrap_or(state.text.len());
        let end_byte = state
            .text
            .char_indices()
            .nth(end)
            .map(|(idx, _)| idx)
            .unwrap_or(state.text.len());

        state.text.replace_range(start_byte..end_byte, text);
        state.cursor = start + text.chars().count();
        state.clear_selection();
    }

    fn current_selected_char_range(state: &EditState) -> Range<usize> {
        if let Some((start, end)) = state.selection_range() {
            start..end
        } else {
            state.cursor..state.cursor
        }
    }

    fn ensure_editing_state(&mut self) {
        let mut state = self.edit_state.borrow_mut();
        if !state.editing {
            *state = EditState::new(self.props.value.as_ref());
            state.clear_selection();
        }
    }

    fn emit_text_change(&self, text: &str, window: &mut Window, cx: &mut App) {
        if let Some(ref handler) = self.props.on_text_change {
            handler(text, window, cx);
        }
    }

    fn selection_from_state(state: &EditState) -> InputSelection {
        let range = Self::current_selected_char_range(state);
        InputSelection {
            start: range.start,
            end: range.end,
            reversed: state
                .selection_anchor
                .is_some_and(|anchor| state.cursor < anchor),
        }
    }

    fn emit_selection_change(&self, selection: InputSelection, window: &mut Window, cx: &mut App) {
        if let Some(ref handler) = self.props.on_selection_change {
            handler(selection, window, cx);
        }
    }

    fn commit_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut state = self.edit_state.borrow_mut();
        if !state.editing {
            return;
        }

        let text = state.finish_edit();
        drop(state);

        if let Some(ref handler) = self.props.on_change {
            handler(&text, window, cx);
        }
        if let Some(ref handler) = self.props.on_edit_end {
            handler(Some(text), window, cx);
        }
        window.refresh();
    }

    fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Focus the input
        window.focus(&self.focus_handle, cx);

        let mut state = self.edit_state.borrow_mut();

        // Ensure editing state is initialised
        if !state.editing {
            *state = EditState::new(self.props.value.as_ref());
        }

        // Double-click: select all text
        if event.click_count == 2 {
            state.select_all();
            let selection = Self::selection_from_state(&state);
            drop(state);
            self.emit_selection_change(selection, window, cx);
            window.refresh();
            return;
        }

        // Calculate cursor position from click.
        let edit_text = state.text.clone();
        let text_len = edit_text.chars().count();
        let char_width = 8.0_f32;
        let click_x: f32 = event.position.x.into();
        let id = self.props.id.clone();

        let stored_origin = TEXT_ORIGINS.with(|o| o.borrow().get(&id).copied());
        let char_pos_f = click_x / char_width;
        let origin = stored_origin.unwrap_or_else(|| {
            let cp = char_pos_f.round().min(text_len as f32);
            (click_x - cp * char_width).max(0.0)
        });
        TEXT_ORIGINS.with(|o| {
            o.borrow_mut().insert(id, origin);
        });

        let char_pos = (((click_x - origin) / char_width).round() as usize).min(text_len);

        let was_editing = state.editing;
        state.editing = true;
        state.start_selection(char_pos);
        let selection = Self::selection_from_state(&state);
        drop(state);

        if !was_editing && let Some(ref handler) = self.props.on_edit_start {
            handler(window, cx);
        }
        self.emit_selection_change(selection, window, cx);
        window.refresh();
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let mut state = self.edit_state.borrow_mut();
        if state.is_dragging && state.editing {
            let edit_text = state.text.clone();
            let text_len = edit_text.chars().count();
            let char_width = 8.0_f32;
            let move_x: f32 = event.position.x.into();
            let id = self.props.id.clone();
            let origin = TEXT_ORIGINS.with(|o| o.borrow().get(&id).copied().unwrap_or(0.0));
            let char_pos = (((move_x - origin) / char_width).round() as usize).min(text_len);
            state.update_selection(char_pos);
            let selection = Self::selection_from_state(&state);
            drop(state);
            self.emit_selection_change(selection, window, _cx);
            window.refresh();
        }
    }

    fn handle_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let mut state = self.edit_state.borrow_mut();
        if state.is_dragging {
            state.end_selection();
            let selection = Self::selection_from_state(&state);
            drop(state);
            self.emit_selection_change(selection, window, _cx);
            window.refresh();
        }
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.focus_handle.is_focused(window) && !self.edit_state.borrow().editing {
            return;
        }
        cx.stop_propagation();

        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let cmd = event.keystroke.modifiers.platform;
        let alt = event.keystroke.modifiers.alt;
        let shift = event.keystroke.modifiers.shift;

        let mut state = self.edit_state.borrow_mut();
        if !state.editing {
            state.text = self.props.value.to_string();
            state.editing = true;
            state.cursor = state.text.chars().count();
            state.clear_selection();
        }

        // Platform modifier handles clipboard shortcuts and select-all. Keep
        // ctrl+a available for the Emacs start-of-line binding below.
        if cmd || (ctrl && matches!(key, "c" | "x" | "v" | "z" | "y")) {
            match key {
                "c" => {
                    if self.props.password {
                        return;
                    }
                    if let Some(selected) = state.get_selected_text() {
                        drop(state);
                        cx.write_to_clipboard(ClipboardItem::new_string(selected));
                    }
                    return;
                }
                "x" => {
                    if self.props.password {
                        return;
                    }
                    if let Some(selected) = state.get_selected_text() {
                        cx.write_to_clipboard(ClipboardItem::new_string(selected));
                        state.delete_selection();
                        if let Some(ref handler) = self.props.on_text_change {
                            handler(&state.text, window, cx);
                        }
                        drop(state);
                        window.refresh();
                    }
                    return;
                }
                "v" => {
                    if let Some(clipboard) = cx.read_from_clipboard()
                        && let Some(paste_text) = clipboard.text()
                    {
                        state.insert_text(&paste_text);
                        if let Some(ref handler) = self.props.on_text_change {
                            handler(&state.text, window, cx);
                        }
                        drop(state);
                        window.refresh();
                    }
                    return;
                }
                "a" => {
                    state.select_all();
                    let selection = Self::selection_from_state(&state);
                    drop(state);
                    self.emit_selection_change(selection, window, cx);
                    window.refresh();
                    return;
                }
                "z" => {
                    let changed = if shift { state.redo() } else { state.undo() };
                    if changed {
                        let selection = Self::selection_from_state(&state);
                        self.emit_text_change(&state.text, window, cx);
                        drop(state);
                        self.emit_selection_change(selection, window, cx);
                        window.refresh();
                    }
                    return;
                }
                "y" if ctrl => {
                    if state.redo() {
                        let selection = Self::selection_from_state(&state);
                        self.emit_text_change(&state.text, window, cx);
                        drop(state);
                        self.emit_selection_change(selection, window, cx);
                        window.refresh();
                    }
                    return;
                }
                _ => {}
            }
        }

        // cmd+left/right — line start/end (macOS); cmd+shift extends selection
        if cmd && matches!(key, "left" | "right") {
            if shift {
                match key {
                    "left" => state.extend_to_start(),
                    "right" => state.extend_to_end(),
                    _ => {}
                }
            } else {
                match key {
                    "left" => state.move_to_start(),
                    "right" => state.move_to_end(),
                    _ => {}
                }
            }
            let selection = Self::selection_from_state(&state);
            drop(state);
            self.emit_selection_change(selection, window, cx);
            window.refresh();
            return;
        }

        // alt+left/right — word jump; alt+shift extends selection
        if alt && matches!(key, "left" | "right") {
            if shift {
                match key {
                    "left" => state.extend_word_backward(),
                    "right" => state.extend_word_forward(),
                    _ => {}
                }
            } else {
                match key {
                    "left" => state.move_word_backward(),
                    "right" => state.move_word_forward(),
                    _ => {}
                }
            }
            let selection = Self::selection_from_state(&state);
            drop(state);
            self.emit_selection_change(selection, window, cx);
            window.refresh();
            return;
        }

        // alt+backspace / alt+d — kill word
        if alt {
            match key {
                "backspace" => {
                    state.kill_word_backward();
                    if let Some(ref handler) = self.props.on_text_change {
                        handler(&state.text, window, cx);
                    }
                    drop(state);
                    window.refresh();
                    return;
                }
                "d" => {
                    state.kill_word_forward();
                    if let Some(ref handler) = self.props.on_text_change {
                        handler(&state.text, window, cx);
                    }
                    drop(state);
                    window.refresh();
                    return;
                }
                _ => {}
            }
        }

        // Emacs ctrl bindings
        if ctrl {
            match key {
                "a" => state.move_to_start(),
                "e" => state.move_to_end(),
                "k" => state.kill_to_end(),
                "u" => state.kill_to_start(),
                "w" => state.kill_word_backward(),
                "h" => state.do_backspace(),
                "d" => state.do_delete(),
                "f" => state.move_forward(),
                "b" => state.move_backward(),
                "left" => {
                    if shift {
                        state.extend_word_backward();
                    } else {
                        state.move_word_backward();
                    }
                }
                "right" => {
                    if shift {
                        state.extend_word_forward();
                    } else {
                        state.move_word_forward();
                    }
                }
                "y" => {
                    if let Some(clipboard) = cx.read_from_clipboard()
                        && let Some(paste_text) = clipboard.text()
                    {
                        state.insert_text(&paste_text);
                    }
                }
                _ => {}
            }
            if let Some(ref handler) = self.props.on_text_change {
                handler(&state.text, window, cx);
            }
            drop(state);
            window.refresh();
            return;
        }

        match key {
            "enter" => {
                drop(state);
                self.commit_edit(window, cx);
                window.blur();
            }
            "escape" => {
                state.abandon_edit();
                drop(state);
                window.blur();
                if let Some(ref handler) = self.props.on_edit_end {
                    handler(None, window, cx);
                }
            }
            "backspace" => {
                state.do_backspace();
                if let Some(ref handler) = self.props.on_text_change {
                    handler(&state.text, window, cx);
                }
                drop(state);
                window.refresh();
            }
            "delete" => {
                state.do_delete();
                if let Some(ref handler) = self.props.on_text_change {
                    handler(&state.text, window, cx);
                }
                drop(state);
                window.refresh();
            }
            "left" => {
                if shift {
                    state.extend_backward();
                } else {
                    state.move_backward();
                }
                let selection = Self::selection_from_state(&state);
                drop(state);
                self.emit_selection_change(selection, window, cx);
                window.refresh();
            }
            "right" => {
                if shift {
                    state.extend_forward();
                } else {
                    state.move_forward();
                }
                let selection = Self::selection_from_state(&state);
                drop(state);
                self.emit_selection_change(selection, window, cx);
                window.refresh();
            }
            "home" => {
                if shift {
                    state.extend_to_start();
                } else {
                    state.move_to_start();
                }
                let selection = Self::selection_from_state(&state);
                drop(state);
                self.emit_selection_change(selection, window, cx);
                window.refresh();
            }
            "end" => {
                if shift {
                    state.extend_to_end();
                } else {
                    state.move_to_end();
                }
                let selection = Self::selection_from_state(&state);
                drop(state);
                self.emit_selection_change(selection, window, cx);
                window.refresh();
            }
            _ => {
                if let Some(ch) = keystroke_to_char(&event.keystroke) {
                    state.insert_char(ch);
                    if let Some(ref handler) = self.props.on_text_change {
                        handler(&state.text, window, cx);
                    }
                    drop(state);
                    window.refresh();
                }
            }
        }
    }

    fn set_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if self.hovered != hovered {
            self.hovered = hovered;
            cx.notify();
        }
    }
}

impl EntityInputHandler for InputEntity {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let state = self.edit_state.borrow();
        let start = Self::utf16_to_char(&state.text, range.start);
        let end = Self::utf16_to_char(&state.text, range.end);
        let start_byte = state
            .text
            .char_indices()
            .nth(start)
            .map(|(idx, _)| idx)
            .unwrap_or(state.text.len());
        let end_byte = state
            .text
            .char_indices()
            .nth(end)
            .map(|(idx, _)| idx)
            .unwrap_or(state.text.len());
        *adjusted_range =
            Some(Self::char_to_utf16(&state.text, start)..Self::char_to_utf16(&state.text, end));
        Some(state.text[start_byte..end_byte].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if self.props.disabled || self.props.readonly {
            return None;
        }

        self.ensure_editing_state();
        let state = self.edit_state.borrow();
        let range = Self::current_selected_char_range(&state);
        Some(UTF16Selection {
            range: Self::char_to_utf16(&state.text, range.start)
                ..Self::char_to_utf16(&state.text, range.end),
            reversed: state
                .selection_anchor
                .is_some_and(|anchor| state.cursor < anchor),
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.props.disabled || self.props.readonly {
            return;
        }

        self.ensure_editing_state();
        let mut state = self.edit_state.borrow_mut();
        let range = range
            .map(|range| {
                Self::utf16_to_char(&state.text, range.start)
                    ..Self::utf16_to_char(&state.text, range.end)
            })
            .unwrap_or_else(|| Self::current_selected_char_range(&state));
        Self::replace_char_range(&mut state, range, text);
        let selection = Self::selection_from_state(&state);
        self.emit_text_change(&state.text, window, cx);
        drop(state);
        self.emit_selection_change(selection, window, cx);
        window.refresh();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.props.disabled || self.props.readonly {
            return;
        }

        self.replace_text_in_range(range, new_text, window, cx);
        if let Some(selected_range) = new_selected_range {
            let mut state = self.edit_state.borrow_mut();
            let start = Self::utf16_to_char(&state.text, selected_range.start);
            let end = Self::utf16_to_char(&state.text, selected_range.end);
            state.selection_anchor = Some(start);
            state.cursor = end;
            let selection = Self::selection_from_state(&state);
            drop(state);
            self.emit_selection_change(selection, window, cx);
        }
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let state = self.edit_state.borrow();
        let start = Self::utf16_to_char(&state.text, range_utf16.start);
        let end = Self::utf16_to_char(&state.text, range_utf16.end).max(start);
        let char_width = 8.0_f32;
        Some(Bounds {
            origin: gpui::point(
                element_bounds.origin.x + px(start as f32 * char_width),
                element_bounds.origin.y,
            ),
            size: gpui::size(
                px((end - start).max(1) as f32 * char_width),
                element_bounds.size.height,
            ),
        })
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let state = self.edit_state.borrow();
        let char_width = 8.0_f32;
        let x: f32 = point.x.into();
        Some(((x / char_width).round().max(0.0) as usize).min(state.text.chars().count()))
    }

    fn accepts_text_input(&self, _window: &mut Window, _cx: &mut Context<Self>) -> bool {
        !self.props.disabled && !self.props.readonly
    }
}

struct InputElement {
    child: AnyElement,
    focus_handle: FocusHandle,
    entity: Entity<InputEntity>,
}

impl Element for InputElement {
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
                    model.commit_edit(window, cx);
                });
            }
        });
        window.handle_input(
            &self.focus_handle,
            ElementInputHandler::new(bounds, self.entity.clone()),
            cx,
        );
        self.child.paint(window, cx);
    }
}

impl IntoElement for InputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Render for InputEntity {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let props = &self.props;

        // Build the effective label from references to avoid cloning through
        // the whole fallback chain on every render.
        let effective_label: Option<&SharedString> = props
            .aria_label
            .as_ref()
            .or(props.label.as_ref())
            .or(props.placeholder.as_ref());
        let native_label = effective_label.cloned().unwrap_or_default();
        let native_props = AriaProps::with_role(props.aria_role.unwrap_or(AriaRole::Textbox))
            .maybe_state(props.disabled, AriaState::Disabled)
            .value_text(if props.password {
                Self::cached_password_mask(&self.aria_password_mask, props.value.as_ref())
            } else {
                props.value.clone()
            });
        cx.register_accessible(AccessibilityNode {
            element_id: props.id.clone(),
            label: effective_label.cloned().unwrap_or_default(),
            props: AriaProps::with_role(props.aria_role.unwrap_or(AriaRole::Textbox))
                .maybe_state(props.disabled, AriaState::Disabled),
        });

        let global_theme = cx.theme();
        let theme = InputTheme::from(global_theme.as_ref());

        let (py, _text_size_class) = match props.size {
            InputSize::Xs => (px(2.0), "text_xs"),
            InputSize::Sm => (px(4.0), "text_xs"),
            InputSize::Md => (px(8.0), "text_sm"),
            InputSize::Lg => (px(12.0), "text_base"),
        };

        let has_error = props.error.is_some();
        let disabled = props.disabled;
        let readonly = props.readonly;

        // Determine editing state from focus
        let is_focused = self.focus_handle.is_focused(window);
        let editing = is_focused && !disabled && !readonly;

        // Get display state from edit_state
        let state = self.edit_state.borrow();
        let selection_anchor = if editing {
            state.selection_anchor
        } else {
            None
        };
        let cursor_pos = state.cursor;
        let edit_text: Option<SharedString> = if editing && state.editing {
            Some(state.text.clone().into())
        } else {
            None
        };
        drop(state);

        let border_color = if has_error {
            theme.error
        } else if editing {
            theme.border_focus
        } else {
            props.border_color.unwrap_or(theme.border)
        };

        let mut container = div().flex().flex_col().gap_1();

        // Label
        if let Some(label) = &props.label {
            container = container.child(
                div()
                    .font_family(global_theme.font_family.clone())
                    .text_sm()
                    .text_color(theme.label)
                    .font_weight(FontWeight::MEDIUM)
                    .child(label.clone()),
            );
        }

        let field_id = ElementId::from((props.id.clone(), "field"));
        let input_debug_id = props.id.to_string();

        // Input wrapper
        let mut input_wrapper = div()
            .id(props.id.clone())
            .debug_selector(move || input_debug_id)
            .font_family(global_theme.font_family.clone())
            .track_focus(&self.focus_handle)
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py(py)
            .rounded_md()
            .border_1()
            .border_color(border_color)
            .focusable();

        // Apply variant styling
        match props.variant {
            InputVariant::Default => {
                input_wrapper = input_wrapper.bg(props.bg_color.unwrap_or(theme.background));
            }
            InputVariant::Filled => {
                input_wrapper = input_wrapper
                    .bg(props.bg_color.unwrap_or(theme.filled_bg))
                    .border_color(theme.transparent);
            }
            InputVariant::Flushed => {
                input_wrapper = input_wrapper
                    .bg(theme.transparent)
                    .border_0()
                    .border_b_1()
                    .border_color(border_color)
                    .rounded_none();
            }
        }

        let border_hover = theme.border_hover;
        if disabled {
            input_wrapper = input_wrapper.opacity(0.5).cursor_not_allowed();
        } else if !readonly {
            input_wrapper = input_wrapper
                .cursor_text()
                .when(self.hovered, |s| s.border_color(border_hover));
        }

        let placeholder_color = props.placeholder_color.unwrap_or(theme.placeholder);
        let text_color = props.text_color.unwrap_or(theme.text);
        let selection_bg = theme.selection_bg;
        let cursor_color = theme.cursor;

        // Add event handlers
        if !disabled && !readonly {
            input_wrapper = input_wrapper
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseDownEvent, window, cx| {
                        this.handle_mouse_down(event, window, cx);
                    }),
                )
                .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                    this.handle_mouse_move(event, window, cx);
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseUpEvent, window, cx| {
                        this.handle_mouse_up(event, window, cx);
                    }),
                )
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    this.handle_key_down(event, window, cx);
                }))
                .on_hover(
                    cx.listener(|this: &mut InputEntity, hovered: &bool, _window, cx| {
                        this.set_hovered(*hovered, cx);
                    }),
                );
        }

        // Left icon
        if let Some(icon) = &props.icon_left {
            input_wrapper =
                input_wrapper.child(div().text_color(placeholder_color).child(icon.clone()));
        }

        // Determine display text. Keep it as a `SharedString` when it comes from
        // props so we don't allocate a new `String` on every render.
        let editing_text = edit_text.is_some();
        let clear_display_text: SharedString = if let Some(text) = edit_text {
            text
        } else if props.value.is_empty() {
            props.placeholder.clone().unwrap_or_default()
        } else {
            props.value.clone()
        };
        let display_text: SharedString =
            if props.password && (editing_text || !props.value.is_empty()) {
                Self::cached_password_mask(&self.display_password_mask, clear_display_text.as_ref())
            } else {
                clear_display_text
            };

        // Build the text element with partial selection support
        let mut text_el = div().id(field_id).flex_1().flex().items_center();

        text_el = match props.size {
            InputSize::Xs => text_el.text_xs(),
            InputSize::Sm => text_el.text_xs(),
            InputSize::Md => text_el.text_sm(),
            InputSize::Lg => text_el,
        };

        if editing {
            let len = display_text.chars().count();
            let cursor_pos = cursor_pos.min(len);
            let selection_anchor = selection_anchor.map(|a| a.min(len));

            let (sel_start, sel_end) = if let Some(anchor) = selection_anchor {
                (cursor_pos.min(anchor), cursor_pos.max(anchor))
            } else {
                (cursor_pos, cursor_pos)
            };

            text_el = text_el.text_color(text_color).whitespace_nowrap();

            if sel_start != sel_end {
                let text = display_text.as_ref();
                let (sel_start_byte, sel_end_byte) =
                    Self::char_range_byte_offsets(text, sel_start, sel_end);
                let before: SharedString = text[..sel_start_byte].into();
                let selected: SharedString = text[sel_start_byte..sel_end_byte].into();
                let after: SharedString = text[sel_end_byte..].into();

                if !before.is_empty() {
                    text_el = text_el.child(before);
                }
                text_el = text_el.child(div().bg(selection_bg).child(selected));
                if !after.is_empty() {
                    text_el = text_el.child(after);
                }
            } else {
                let text = display_text.as_ref();
                let (cursor_byte, _) = Self::char_range_byte_offsets(text, cursor_pos, len);
                let before: SharedString = text[..cursor_byte].into();
                let after: SharedString = text[cursor_byte..].into();
                let cursor_debug_id = format!("{}-cursor", props.id);
                let cursor_height = match props.size {
                    InputSize::Xs => px(14.0),
                    InputSize::Sm => px(14.0),
                    InputSize::Md => px(20.0),
                    InputSize::Lg => px(24.0),
                };

                if !before.is_empty() {
                    text_el = text_el.child(before);
                }
                text_el = text_el.child(
                    div()
                        .debug_selector(move || cursor_debug_id)
                        .flex_none()
                        .w(px(1.5))
                        .h(cursor_height)
                        .bg(cursor_color),
                );
                if !after.is_empty() {
                    text_el = text_el.child(after);
                }
            }
        } else if props.value.is_empty() {
            text_el = text_el.text_color(placeholder_color).child(display_text);
        } else {
            text_el = text_el.text_color(text_color).child(display_text);
        }

        input_wrapper = input_wrapper.child(text_el);

        // Right icon
        if let Some(icon) = &props.icon_right {
            input_wrapper =
                input_wrapper.child(div().text_color(placeholder_color).child(icon.clone()));
        }

        container = container.child(apply_native_accessibility(
            input_wrapper,
            native_label,
            &native_props,
        ));

        // Error message
        if let Some(error) = &props.error {
            container =
                container.child(div().text_xs().text_color(theme.error).child(error.clone()));
        }

        container
    }
}

/// Internal entity that renders an [`Input`] with stable identity across frames.
pub struct InputEntity {
    props: Input,
    focus_handle: FocusHandle,
    edit_state: Rc<RefCell<EditState>>,
    hovered: bool,
    aria_password_mask: RefCell<Option<(usize, SharedString)>>,
    display_password_mask: RefCell<Option<(usize, SharedString)>>,
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let focus_handle = self.focus_handle.clone().unwrap_or_else(|| {
            FOCUS_HANDLES.with(|handles| {
                let mut handles = handles.borrow_mut();
                handles
                    .entry(id.clone())
                    .or_insert_with(|| cx.focus_handle())
                    .clone()
            })
        });
        let edit_state = EDIT_STATES.with(|states| {
            let mut states = states.borrow_mut();
            states
                .entry(id.clone())
                .or_insert_with(|| Rc::new(RefCell::new(EditState::default())))
                .clone()
        });

        let entity: Entity<InputEntity> = INPUT_ENTITIES.with(|map| {
            let mut map = map.borrow_mut();
            if let Some(weak) = map.get(&id)
                && let Some(entity) = weak.upgrade()
            {
                return entity;
            }
            let entity = cx.new(|_cx| InputEntity {
                props: Input::new(id.clone()),
                focus_handle: focus_handle.clone(),
                edit_state: edit_state.clone(),
                hovered: false,
                aria_password_mask: RefCell::new(None),
                display_password_mask: RefCell::new(None),
            });
            map.insert(id.clone(), entity.downgrade());
            entity
        });

        FOCUS_SUBS.with(|subs| {
            let mut subs = subs.borrow_mut();
            if !subs.contains_key(&id) {
                let entity_weak = entity.downgrade();
                let sub = window.on_focus_out(&focus_handle, cx, move |_event, window, cx| {
                    if let Some(entity) = entity_weak.upgrade() {
                        entity.update(cx, |model, cx| {
                            model.commit_edit(window, cx);
                        });
                    }
                });
                subs.insert(id.clone(), sub);
            }
        });

        entity.update(cx, |model, _cx| {
            model.props = self;
            // Keep the persistent focus handle/edit state in sync with any
            // explicit ones provided on the builder.
            model.focus_handle = focus_handle.clone();
            model.edit_state = edit_state;
        });
        InputElement {
            child: entity.clone().into_any_element(),
            focus_handle,
            entity,
        }
    }
}

impl IntoElement for Input {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests;
