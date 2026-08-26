//! VolumeKnob - A circular volume knob with path-painted fill indicator
//!
//! A visual volume control with:
//! - Path-painted circular fill that rises from bottom
//! - Drag support with vertical mouse movement
//! - Scroll wheel adjustment (Shift for fine control: 0.5% vs 5%)
//! - Double-click to toggle mute
//! - Keyboard support (requires focus - click to focus):
//!   - Arrow Up/Right: increase volume (5%)
//!   - Arrow Down/Left: decrease volume (5%)
//!   - Page Up: increase volume (10%)
//!   - Page Down: decrease volume (10%)
//!   - M key: toggle mute
//!   - Media keys: AudioVolumeUp/Down/Mute (F12/F11/F10)
//! - Mute state support
//! - Customizable colors and theme support

use super::interactions::{
    InteractionConfig, clear_drag_state, drag_has_moved, get_drag_state, handle_drag,
    handle_keyboard, handle_scroll, mark_drag_moved, store_drag_state, value_tracker,
};
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState};
use crate::audio_accessibility::{
    AudioAccessibilitySummary, normalized, range_description, value_text,
};
use crate::scale::Scale;
use crate::theme::ThemeExt;
use d3rs::render2d::{Renderer2D, VelloBackend};
use gpui::*;

#[cfg(test)]
mod tests;
mod types;
mod volume_knob_fill_element;

pub use types::*;

use volume_knob_fill_element::VolumeKnobFillElement;

/// A circular volume knob with fill indicator.
#[derive(IntoElement)]
pub struct VolumeKnob {
    id: ElementId,
    value: f32,
    label: SharedString,
    size: DefiniteLength,
    muted: bool,
    disabled: bool,
    /// Optional theme (uses global theme if not set)
    theme: Option<VolumeKnobTheme>,
    /// Override: accent color
    accent_color: Option<Rgba>,
    /// Override: muted color
    muted_color: Option<Rgba>,
    /// Override: background color
    bg_color: Option<Rgba>,
    /// Override: text color
    text_color: Option<Rgba>,
    on_change: Option<Box<dyn Fn(f32, &mut Window, &mut App) + 'static>>,
    on_commit: Option<Box<dyn Fn(f32, &mut Window, &mut App) + 'static>>,
    on_mute_toggle: Option<Box<dyn Fn(bool, &mut Window, &mut App) + 'static>>,
    focus_handle: Option<FocusHandle>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
    renderer_2d: Renderer2D,
    vello_backend: VelloBackend,
}

impl VolumeKnob {
    /// Construct a knob with an ID stable at this call site.
    ///
    /// Repeated knobs emitted from one call site, such as a loop, must provide
    /// their own distinct stable ID with [`Self::id`].
    #[track_caller]
    pub fn new() -> Self {
        Self {
            id: ElementId::CodeLocation(*std::panic::Location::caller()),
            value: 0.0,
            label: "".into(),
            size: px(40.0).into(),
            muted: false,
            disabled: false,
            theme: None,
            accent_color: None,
            muted_color: None,
            bg_color: None,
            text_color: None,
            on_change: None,
            on_commit: None,
            on_mute_toggle: None,
            focus_handle: None,
            aria_label: None,
            aria_role: None,
            renderer_2d: Renderer2D::default(),
            vello_backend: VelloBackend::default(),
        }
    }

    /// Set the theme
    pub fn theme(mut self, theme: VolumeKnobTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Select the high-level 2D renderer for custom-painted knob visuals.
    pub fn renderer_2d(mut self, renderer: Renderer2D) -> Self {
        self.renderer_2d = renderer;
        self
    }

    /// Select the Vello WGPU/CPU backend.
    pub fn vello_backend(mut self, backend: VelloBackend) -> Self {
        self.vello_backend = backend;
        self
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn size(mut self, size: impl Into<DefiniteLength>) -> Self {
        self.size = size.into();
        self
    }

    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    /// Disable all pointer and keyboard interaction.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Override accent color (ring and fill when active)
    pub fn accent_color(mut self, color: impl Into<Rgba>) -> Self {
        self.accent_color = Some(color.into());
        self
    }

    /// Override muted color
    pub fn muted_color(mut self, color: impl Into<Rgba>) -> Self {
        self.muted_color = Some(color.into());
        self
    }

    /// Override background color
    pub fn bg_color(mut self, color: impl Into<Rgba>) -> Self {
        self.bg_color = Some(color.into());
        self
    }

    /// Override text color
    pub fn text_color(mut self, color: impl Into<Rgba>) -> Self {
        self.text_color = Some(color.into());
        self
    }

    /// Set value change handler (called on scroll wheel)
    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Set a semantic commit handler fired on drag release or after a discrete change.
    pub fn on_commit(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_commit = Some(Box::new(handler));
        self
    }

    /// Set mute toggle handler (called on double-click)
    pub fn on_mute_toggle(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mute_toggle = Some(Box::new(handler));
        self
    }

    /// Set the focus handle for keyboard navigation
    pub fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle);
        self
    }

    /// Set an explicit ARIA label
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Slider)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// The value presented by the control, after applying mute and range
    /// semantics. Keep visual rendering and accessibility in lockstep.
    fn effective_value(&self) -> f64 {
        if self.muted {
            0.0
        } else {
            self.value.clamp(0.0, 1.0) as f64
        }
    }

    fn should_commit_drag(start_value: f64, final_value: f64, moved: bool) -> bool {
        moved && (final_value - start_value).abs() > f64::EPSILON
    }

    /// Return non-rendering accessibility metadata for this volume control.
    pub fn accessibility_summary(&self) -> AudioAccessibilitySummary {
        let label = self
            .aria_label
            .clone()
            .filter(|label| !label.is_empty())
            .unwrap_or_else(|| {
                if self.label.is_empty() {
                    "Volume".into()
                } else {
                    self.label.clone()
                }
            });
        let value = self.effective_value();
        let unit: SharedString = "%".into();
        let value_text = value_text(value * 100.0, &unit);
        let mut description = range_description(
            "volume knob",
            &label,
            &value_text,
            0.0,
            100.0,
            self.disabled,
        );
        if self.muted {
            description = SharedString::new(format!("{description} Muted."));
        }

        AudioAccessibilitySummary {
            control_type: "volume_knob",
            label,
            role: self.aria_role.unwrap_or(AriaRole::Slider),
            value_now: Some(value),
            value_min: Some(0.0),
            value_max: Some(1.0),
            value_text: Some(value_text),
            unit: Some(unit),
            normalized: Some(normalized(value, 0.0, 1.0, Scale::Linear)),
            scale: Some(Scale::Linear),
            selected: false,
            disabled: self.disabled,
            muted: self.muted,
            peak_value: None,
            description,
        }
    }
}

impl Default for VolumeKnob {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for VolumeKnob {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let effective_value = self.effective_value();

        // Register in accessibility tree
        let effective_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| self.label.clone());
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: effective_label,
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Slider))
                .value_range(effective_value, 0.0, 1.0)
                .maybe_state(self.disabled, AriaState::Disabled),
        });

        // Resolve DefiniteLength to Pixels using window's rem_size
        let resolved_size = match self.size {
            DefiniteLength::Absolute(abs) => match abs {
                AbsoluteLength::Pixels(px_val) => px_val,
                AbsoluteLength::Rems(rem_val) => {
                    let rem_px: f32 = window.rem_size().into();
                    px(rem_val.0 * rem_px)
                }
            },
            DefiniteLength::Fraction(_) => px(40.0), // fallback
        };

        // Get theme: use explicit theme, or derive from global theme
        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| VolumeKnobTheme::from(global_theme.as_ref()));

        // Apply color overrides or use theme defaults
        let accent_color = self.accent_color.unwrap_or(theme.accent);
        let muted_color = self.muted_color.unwrap_or(theme.muted);
        let bg_color = self.bg_color.unwrap_or(theme.background);
        let text_color = self.text_color.unwrap_or(theme.text);

        let display_value = effective_value as f32;
        let ring_color = if self.muted {
            muted_color
        } else {
            accent_color
        };
        let text_color_final = if self.muted { muted_color } else { text_color };

        // Make fill color slightly lighter than the background
        let fill_color = if self.muted {
            muted_color
        } else {
            // Lighten the background color by converting to Hsla, increasing lightness,
            // then converting back to Rgba
            let mut lighter: Hsla = bg_color.into();
            lighter.l = (lighter.l + 0.15).min(1.0);
            lighter.into()
        };

        // Capture values for closures
        let current_muted = self.muted;
        let knob_size_f32 = resolved_size.to_f64() as f32;

        // Shared current value tracker and interaction config (with media keys enabled)
        let current_value = value_tracker(self.value as f64);
        let interaction_config =
            InteractionConfig::rotational(0.0, 1.0, Scale::Linear, knob_size_f32).with_media_keys();
        let drag_key = self.id.clone();
        let disabled = self.disabled;

        let mut container = div()
            .id(self.id)
            .relative()
            .w(resolved_size)
            .h(resolved_size)
            .cursor_pointer();

        if disabled {
            container = container.cursor_not_allowed().opacity(0.5);
        }

        if let Some(ref focus_handle) = self.focus_handle {
            container = container.track_focus(focus_handle).focusable();
        }

        // Convert handlers to Rc for sharing between closures
        let on_change_rc = if disabled {
            None
        } else {
            self.on_change.map(std::rc::Rc::new)
        };
        let on_commit_rc = if disabled {
            None
        } else {
            self.on_commit.map(std::rc::Rc::new)
        };
        let on_mute_rc = if disabled {
            None
        } else {
            self.on_mute_toggle.map(std::rc::Rc::new)
        };

        // Mouse down: focus for keyboard navigation AND capture the drag
        // origin so on_mouse_move can compute a delta. Storing the click
        // Y position is what makes the drag delta-based instead of
        // interpreting raw window-space Y as knob-local progress.
        if !disabled && (self.focus_handle.is_some() || on_change_rc.is_some()) {
            let focus_handle_click = self.focus_handle.clone();
            let drag_key_down = drag_key.clone();
            let current_value_at_press = current_value.clone();
            let has_change_handler = on_change_rc.is_some();
            container = container.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                cx.stop_propagation();
                if let Some(ref fh) = focus_handle_click {
                    fh.focus(window, cx);
                }
                if has_change_handler {
                    let click_y: f32 = event.position.y.into();
                    store_drag_state(drag_key_down.clone(), click_y, current_value_at_press.get());
                }
            });
        }

        // Scroll wheel - adjust value (shift for fine-grained control)
        if let Some(ref change_handler) = on_change_rc {
            let scroll_handler = change_handler.clone();
            let current_value_scroll = current_value.clone();
            let config_scroll = interaction_config.clone();
            let commit_scroll = on_commit_rc.clone();
            container = container.on_scroll_wheel(move |event, window, cx| {
                cx.stop_propagation();
                let val = current_value_scroll.get();
                if let Some(new_value) =
                    handle_scroll(&event.delta, &event.modifiers, val, &config_scroll)
                {
                    current_value_scroll.set(new_value);
                    scroll_handler(new_value as f32, window, cx);
                    if let Some(ref commit) = commit_scroll {
                        commit(new_value as f32, window, cx);
                    }
                }
            });
        }

        // Drag support
        if !disabled {
            let drag_handler = on_change_rc.clone();
            let drag_key_move = drag_key.clone();
            let current_value_drag = current_value.clone();
            let config_drag = interaction_config.clone();

            container = container.on_mouse_move(move |event, window, cx| {
                if event.pressed_button == Some(MouseButton::Left) {
                    // Delta-based drag keyed off the position captured at
                    // mouse_down — see store_drag_state above.
                    if let Some(ref handler) = drag_handler
                        && let Some(state) = get_drag_state(&drag_key_move)
                    {
                        let current_y: f32 = event.position.y.into();
                        if (current_y - state.start_pos).abs() > f32::EPSILON {
                            mark_drag_moved(&drag_key_move);
                        }
                        if let Some(new_value) = handle_drag(current_y, &state, &config_drag) {
                            current_value_drag.set(new_value);
                            handler(new_value as f32, window, cx);
                        }
                    }
                }
            });
        }

        // Mouse up — clear drag state for the next drag.
        if !disabled && on_change_rc.is_some() {
            let drag_key_up = drag_key.clone();
            let commit_drag = on_commit_rc.clone();
            let current_value_up = current_value.clone();
            container = container.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                if let Some(state) = get_drag_state(&drag_key_up) {
                    let final_value = current_value_up.get();
                    if Self::should_commit_drag(
                        state.start_value,
                        final_value,
                        drag_has_moved(&drag_key_up),
                    ) && let Some(ref commit) = commit_drag
                    {
                        commit(final_value as f32, window, cx);
                    }
                }
                clear_drag_state(drag_key_up.clone());
            });

            // This capture-phase callback covers release after the pointer
            // leaves the knob. It updates the final delta, emits one commit
            // when the value changed, and always clears retained drag state.
            let drag_handler_out = on_change_rc.clone();
            let drag_key_up_out = drag_key.clone();
            let commit_drag_out = on_commit_rc.clone();
            let current_value_up_out = current_value.clone();
            let config_up_out = interaction_config.clone();
            container = container.on_mouse_up_out(MouseButton::Left, move |event, window, cx| {
                if let Some(state) = get_drag_state(&drag_key_up_out) {
                    let position: f32 = event.position.y.into();
                    if (position - state.start_pos).abs() > f32::EPSILON {
                        mark_drag_moved(&drag_key_up_out);
                    }
                    let previous_value = current_value_up_out.get();
                    let final_value =
                        handle_drag(position, &state, &config_up_out).unwrap_or(previous_value);
                    current_value_up_out.set(final_value);
                    if Self::should_commit_drag(
                        state.start_value,
                        final_value,
                        drag_has_moved(&drag_key_up_out),
                    ) {
                        if final_value != previous_value
                            && let Some(ref handler) = drag_handler_out
                        {
                            handler(final_value as f32, window, cx);
                        }
                        if let Some(ref commit) = commit_drag_out {
                            commit(final_value as f32, window, cx);
                        }
                    }
                }
                clear_drag_state(drag_key_up_out.clone());
            });
        }

        // Double-click - toggle mute
        if !disabled && let Some(ref mute_handler) = on_mute_rc {
            let click_mute = mute_handler.clone();
            container = container.on_click(move |event, window, cx| {
                if event.click_count() == 2 {
                    click_mute(!current_muted, window, cx);
                }
            });
        }

        // Keyboard support (including media keys for volume control)
        if !disabled && (on_change_rc.is_some() || on_mute_rc.is_some()) {
            let key_change = on_change_rc.clone();
            let key_mute = on_mute_rc.clone();
            let current_value_key = current_value.clone();
            let config_key = interaction_config.clone();
            let commit_key = on_commit_rc.clone();

            container = container.on_key_down(move |event, window, cx| {
                cx.stop_propagation();
                let key = event.keystroke.key.as_str();

                // Handle mute keys specially
                if matches!(key, "m" | "audiomute" | "audiovolumemute" | "f10") {
                    if let Some(ref handler) = key_mute {
                        handler(!current_muted, window, cx);
                    }
                } else if let Some(ref handler) = key_change {
                    // Use shared keyboard handler for value changes
                    if let Some(new_value) = handle_keyboard(
                        key,
                        &event.keystroke.modifiers,
                        current_value_key.get(),
                        &config_key,
                    ) {
                        current_value_key.set(new_value);
                        handler(new_value as f32, window, cx);
                        if let Some(ref commit) = commit_key {
                            commit(new_value as f32, window, cx);
                        }
                    }
                }
            });
        }

        container
            // Custom painted fill element
            .child(
                div().absolute().inset_0().child(
                    VolumeKnobFillElement::new(
                        ElementId::named_usize("volume-knob-fill", 0),
                        resolved_size,
                        display_value,
                        bg_color,
                        fill_color,
                        ring_color,
                    )
                    .renderer_2d(self.renderer_2d)
                    .vello_backend(self.vello_backend),
                ),
            )
            // Label text in center
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(text_color_final)
                    .child(self.label),
            )
    }
}
