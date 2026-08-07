//! Potentiometer (rotary knob) component for audio plugin parameters
//!
//! A circular knob with:
//! - Selection highlighting for plugin parameter editing
//! - Drag support with vertical mouse movement (via on_drag_start handler)
//! - Scroll wheel adjustment (Shift for fine control: 0.5% vs 5%)
//! - Double-click to reset to default
//! - Keyboard navigation (when focused via click):
//!   - Arrow Up/Right: increase value (5%)
//!   - Arrow Down/Left: decrease value (5%)
//!   - Page Up: increase value (10%)
//!   - Page Down: decrease value (10%)
//!   - Escape: reset to default
//! - Value display with units
//! - Keyboard shortcut hints
//! - Rotating indicator dot
//! - Tick marks with major (labeled) and minor (unlabeled) ticks

use super::interactions::{
    InteractionConfig, clear_drag_state, get_drag_state, handle_drag, handle_keyboard,
    handle_scroll, store_drag_state, value_tracker,
};
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState};
use crate::audio_accessibility::{
    AudioAccessibilitySummary, normalized, range_description, value_text,
};
use crate::theme::ThemeExt;
use gpui::prelude::*;
use gpui::*;

mod knob_arc_element;
mod potentiometer_size;
mod tick_element;
mod types;

pub use potentiometer_size::*;
pub use types::*;

use knob_arc_element::KnobArcElement;
use tick_element::{PotentiometerTickLinesElement, get_tick_geometry};

/// A potentiometer (rotary knob) component for audio plugin parameters
#[derive(IntoElement)]
pub struct Potentiometer {
    id: ElementId,
    value: f64,
    min: f64,
    max: f64,
    unit: SharedString,
    label: Option<SharedString>,
    shortcut_key: Option<char>,
    size: PotentiometerSize,
    scale: PotentiometerScale,
    selected: bool,
    disabled: bool,
    theme: Option<PotentiometerTheme>,
    /// Override accent color for the value arc (e.g., plugin-specific color)
    accent_color: Option<Rgba>,
    /// Platform design tokens for arc geometry and sizing.
    design_tokens: crate::audio_design_tokens::AudioDesignTokens,
    on_change: Option<Box<dyn Fn(f64, &mut Window, &mut App) + 'static>>,
    on_commit: Option<Box<dyn Fn(f64, &mut Window, &mut App) + 'static>>,
    on_drag_start: Option<Box<dyn Fn(f32, f64, &mut Window, &mut App) + 'static>>,
    on_select: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    on_reset: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    focus_handle: Option<FocusHandle>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
    /// Cached formatted label with keyboard shortcut indicator.
    formatted_label: SharedString,
    /// Cached formatted value display (without unit).
    formatted_value_only: SharedString,
}

impl Potentiometer {
    /// Create a new potentiometer with the given ID
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            unit: "".into(),
            label: None,
            shortcut_key: None,
            size: PotentiometerSize::default(),
            scale: PotentiometerScale::default(),
            selected: false,
            disabled: false,
            theme: None,
            accent_color: None,
            design_tokens: Default::default(),
            on_change: None,
            on_commit: None,
            on_drag_start: None,
            on_select: None,
            on_reset: None,
            focus_handle: None,
            aria_label: None,
            aria_role: None,
            formatted_label: SharedString::default(),
            formatted_value_only: SharedString::default(),
        }
    }

    /// Convert a value to normalized position [0, 1] based on scale type
    fn value_to_normalized(&self, value: f64) -> f64 {
        self.scale.value_to_normalized(value, self.min, self.max)
    }

    /// Set the current value (clamped to min/max during render)
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self.formatted_value_only = self.format_value_only();
        self
    }

    /// Set the minimum value
    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self.formatted_value_only = self.format_value_only();
        self
    }

    /// Set the maximum value
    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self.formatted_value_only = self.format_value_only();
        self
    }

    /// Set the unit label (e.g., "dB", "Hz", "%", ":1")
    pub fn unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = unit.into();
        self.formatted_value_only = self.format_value_only();
        self
    }

    /// Set the display label
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self.formatted_label = self.format_label();
        self
    }

    /// Set the keyboard shortcut key for the label
    pub fn shortcut_key(mut self, key: char) -> Self {
        self.shortcut_key = Some(key);
        self.formatted_label = self.format_label();
        self
    }

    /// Set the potentiometer size
    pub fn size(mut self, size: PotentiometerSize) -> Self {
        self.size = size;
        self
    }

    /// Set the value scale type (linear or logarithmic)
    ///
    /// Use `Logarithmic` for frequency parameters (e.g., 20Hz to 20kHz)
    /// where equal visual distances should represent equal ratios.
    ///
    /// Note: For logarithmic scale, min must be > 0.
    pub fn scale(mut self, scale: PotentiometerScale) -> Self {
        self.scale = scale;
        self
    }

    /// Set selected state (for plugin parameter editing)
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set theme colors
    pub fn theme(mut self, theme: PotentiometerTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set a custom accent color for the value arc (overrides theme accent).
    ///
    /// Use this to match the plugin's type color (e.g., blue for EQ, red for compressor).
    pub fn accent_color(mut self, color: Rgba) -> Self {
        self.accent_color = Some(color);
        self
    }

    /// Set platform design tokens for arc geometry and sizing.
    pub fn design_tokens(mut self, tokens: crate::audio_design_tokens::AudioDesignTokens) -> Self {
        self.design_tokens = tokens;
        self
    }

    /// Set platform design defaults through the shared design system.
    pub fn design(mut self, design: impl Into<std::sync::Arc<gpui_design::DesignSystem>>) -> Self {
        let design = design.into();
        self.design_tokens = crate::audio_design_tokens::AudioDesignTokens::from(design.as_ref());
        self
    }

    /// Set value change handler (called on scroll wheel and mouse click)
    ///
    /// When only `on_change` is provided (without `on_select` or `on_drag_start`),
    /// clicking the potentiometer will increment the value by 10% and wrap around at max.
    /// Scrolling will adjust the value by 5% increments.
    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Set a semantic commit handler. Dragging emits once on release; keyboard,
    /// scroll, and click-step interactions emit after their preview change.
    pub fn on_commit(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_commit = Some(Box::new(handler));
        self
    }

    /// Set drag start handler (called on mouse down with y position and current value)
    pub fn on_drag_start(
        mut self,
        handler: impl Fn(f32, f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_drag_start = Some(Box::new(handler));
        self
    }

    /// Set select handler (called on click to select this parameter)
    pub fn on_select(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }

    /// Set reset handler (called on double-click)
    pub fn on_reset(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_reset = Some(Box::new(handler));
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

    /// Return non-rendering accessibility metadata for this audio parameter.
    pub fn accessibility_summary(&self) -> AudioAccessibilitySummary {
        let label = self
            .aria_label
            .clone()
            .or_else(|| self.label.clone())
            .unwrap_or_else(|| "Potentiometer".into());
        let value = self.value.clamp(self.min, self.max);
        let value_text = value_text(value, &self.unit);
        let description = range_description(
            "rotary slider",
            &label,
            &value_text,
            self.min,
            self.max,
            self.disabled,
        );

        AudioAccessibilitySummary {
            control_type: "potentiometer",
            label,
            role: self.aria_role.unwrap_or(AriaRole::Slider),
            value_now: Some(value),
            value_min: Some(self.min),
            value_max: Some(self.max),
            value_text: Some(value_text),
            unit: (!self.unit.is_empty()).then(|| self.unit.clone()),
            normalized: Some(normalized(value, self.min, self.max, self.scale)),
            scale: Some(self.scale),
            selected: self.selected,
            disabled: self.disabled,
            muted: false,
            peak_value: None,
            description,
        }
    }

    /// Format the label with keyboard shortcut indicator
    fn format_label(&self) -> SharedString {
        let label = self.label.as_ref().cloned().unwrap_or_default();
        match self.shortcut_key {
            Some(key) => {
                let key_lower = key.to_ascii_lowercase();
                let mut match_pos = None;
                for (pos, ch) in label.char_indices() {
                    if ch.to_ascii_lowercase() == key_lower {
                        match_pos = Some(pos);
                        break;
                    }
                }
                if let Some(pos) = match_pos {
                    let matched_char = label[pos..].chars().next().unwrap();
                    let after_pos = pos + matched_char.len_utf8();
                    SharedString::new(format!(
                        "{}[{}]{}",
                        &label[..pos],
                        matched_char.to_ascii_uppercase(),
                        &label[after_pos..]
                    ))
                } else {
                    SharedString::new(format!("[{}] {}", key.to_ascii_uppercase(), label))
                }
            }
            None => label,
        }
    }

    /// Format the value display (with unit suffix)
    /// Note: Currently unused, kept for potential future use
    #[allow(dead_code)]
    fn format_value(&self) -> SharedString {
        let value = self.value.clamp(self.min, self.max);
        let unit = self.unit.as_ref();
        SharedString::new(if unit == ":1" {
            format!("{:.1}{}", value, unit)
        } else if unit == "%" {
            // Compute percentage relative to the range (min=0%, max=100%)
            let pct = if self.max > self.min {
                ((value - self.min) / (self.max - self.min)) * 100.0
            } else {
                0.0
            };
            format!("{:.0}{}", pct, unit)
        } else if unit == "Hz" {
            format!("{:.0} {}", value, unit)
        } else if unit.is_empty() {
            format!("{:.1}", value)
        } else {
            format!("{:.1} {}", value, unit)
        })
    }

    /// Format the value display (without unit, for center display)
    fn format_value_only(&self) -> SharedString {
        let value = self.value.clamp(self.min, self.max);
        let unit = self.unit.as_ref();
        SharedString::new(if unit == ":1" {
            format!("{:.1}", value)
        } else if unit == "%" {
            // Compute percentage relative to the range (min=0%, max=100%)
            let pct = if self.max > self.min {
                ((value - self.min) / (self.max - self.min)) * 100.0
            } else {
                0.0
            };
            format!("{:.0}", pct)
        } else if unit == "Hz" {
            format!("{:.0}", value)
        } else {
            // Default: show one decimal place
            format!("{:.1}", value)
        })
    }
}

impl RenderOnce for Potentiometer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Register in accessibility tree
        let effective_label = self
            .aria_label
            .clone()
            .or_else(|| self.label.clone())
            .unwrap_or_default();
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: effective_label,
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Slider))
                .value_range(self.value, self.min, self.max)
                .maybe_state(self.disabled, AriaState::Disabled),
        });

        let global_theme = cx.theme();
        let default_theme = PotentiometerTheme::from(global_theme.as_ref());
        let theme = self.theme.clone().unwrap_or(default_theme);
        let selected = self.selected;
        let disabled = self.disabled;

        // Use scale-aware normalization for indicator position
        let normalized = self.value_to_normalized(self.value) as f32;

        // Calculate angle for indicator with dead zone at bottom
        let start_rad: f32 = self.design_tokens.knob_arc_start_deg.to_radians();
        let end_rad: f32 = (self.design_tokens.knob_arc_start_deg
            + self.design_tokens.knob_arc_sweep_deg)
            .to_radians();
        let angle_rad = start_rad + (end_rad - start_rad) * normalized;

        let knob_size = self.size.knob_size();
        let radius = self.size.indicator_radius();
        let center = knob_size / 2.0;
        // Make indicator larger for Lg size to be more visible
        let indicator_size = match self.size {
            PotentiometerSize::Xs => {
                if selected {
                    5.0
                } else {
                    3.0
                }
            }
            PotentiometerSize::Sm => {
                if selected {
                    6.0
                } else {
                    4.0
                }
            }
            PotentiometerSize::Md => {
                if selected {
                    6.0
                } else {
                    4.0
                }
            }
            PotentiometerSize::Lg => {
                if selected {
                    10.0
                } else {
                    8.0
                }
            }
        };

        let x = center + radius * angle_rad.cos() - (indicator_size / 2.0);
        let y = center + radius * angle_rad.sin() - (indicator_size / 2.0);

        let formatted_label = self.formatted_label;
        let value_str_only = self.formatted_value_only;
        let unit_str = self.unit.clone();
        let min_width = self.size.min_width();

        // Colors based on selection state
        let bg_color = if selected {
            theme.accent_muted
        } else {
            theme.surface
        };
        let border_color = if selected { theme.accent } else { theme.border };
        let knob_bg = if selected {
            theme.surface_hover
        } else {
            theme.knob_bg
        };
        let label_color = if selected {
            theme.accent
        } else {
            theme.text_secondary
        };
        let value_color = if selected {
            theme.text_on_accent
        } else {
            theme.text_primary
        };
        // For Lg size or when selected, use accent color for better visibility
        let indicator_color = if matches!(self.size, PotentiometerSize::Lg) || selected {
            theme.accent
        } else {
            theme.text_muted
        };

        // Capture values for closures
        let value = self.value;
        let min = self.min;
        let max = self.max;
        let scale = self.scale;

        // Shared current value tracker and interaction config
        let current_value = value_tracker(value);
        // Potentiometer uses rotational config (drag distance = knob_size for full range)
        let interaction_config = InteractionConfig::rotational(min, max, scale, knob_size);
        let drag_key = self.id.clone();

        // Layout style: boxed (chassis around the whole knob) vs underlined
        // (title above with a thin rule, no surrounding chassis).
        let underlined = self.design_tokens.knob_label_style
            == crate::audio_design_tokens::AudioDesignTokens::LABEL_UNDERLINED;

        let mut container = div()
            .id(self.id)
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .min_w(px(min_width));

        if underlined {
            // No chassis fill or border — just the title rule + knob below.
            // Selection is communicated via the title color and weight (set
            // further down) rather than a surrounding box.
        } else {
            container = container
                .p_2()
                .rounded_lg()
                .bg(bg_color)
                .border_2()
                .border_color(border_color);
        }

        // Track focus if handle provided
        // Both track_focus (for focus observation) and focusable (for key events) are needed
        if let Some(ref focus_handle) = self.focus_handle {
            container = container.track_focus(focus_handle).focusable();
        }

        // Add shadow when selected (chassis only — underlined style stays flat).
        if selected && !underlined {
            container = container.shadow_md();
        }

        // Hover effect — only apply chassis-style hover when boxed; the
        // underlined variant relies on indicator/title color changes instead.
        if !underlined {
            let hover_border = theme.accent;
            let hover_bg = theme.surface_hover;
            container = container.hover(|s| s.border_color(hover_border).bg(hover_bg));
        }

        // Cursor
        if disabled {
            container = container.cursor_not_allowed().opacity(0.5);
        } else {
            container = container.cursor_ns_resize();
        }

        // Event handlers
        if !disabled {
            // Wrap on_change in Rc if it exists, so we can use it in multiple handlers
            let on_change_rc = self.on_change.map(|handler| std::rc::Rc::new(handler));
            let on_commit_rc = self.on_commit.map(|handler| std::rc::Rc::new(handler));
            let on_reset_rc = self.on_reset.map(|handler| std::rc::Rc::new(handler));

            // Mouse down - focus, select, and optionally start drag
            let on_select = self.on_select;
            let on_drag_start = self.on_drag_start;
            let on_change_click = on_change_rc.clone();
            let drag_key_down = drag_key.clone();
            let current_value_down = current_value.clone();
            let focus_handle_click = self.focus_handle.clone();

            container = container.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                cx.stop_propagation();
                // Always focus for keyboard navigation
                if let Some(ref fh) = focus_handle_click {
                    fh.focus(window, cx);
                }

                // Handle Selection
                if let Some(ref handler) = on_select {
                    handler(window, cx);
                }

                // Handle Drag or Click-Step
                if let Some(ref handler) = on_drag_start {
                    handler(event.position.y.into(), value, window, cx);
                } else if on_change_click.is_some() {
                    store_drag_state(
                        drag_key_down.clone(),
                        event.position.y.into(),
                        current_value_down.get(),
                    );
                }
            });

            // Native delta drag; a release without movement retains the legacy click-step.
            if let Some(ref handler) = on_change_rc {
                let drag_handler = handler.clone();
                let drag_key_move = drag_key.clone();
                let current_value_drag = current_value.clone();
                let config_drag = interaction_config.clone();
                container = container.on_mouse_move(move |event, window, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let Some(state) = get_drag_state(&drag_key_move)
                    {
                        let position: f32 = event.position.y.into();
                        if let Some(new_value) = handle_drag(position, &state, &config_drag) {
                            current_value_drag.set(new_value);
                            drag_handler(new_value, window, cx);
                        }
                    }
                });
                let release_change = handler.clone();
                let release_commit = on_commit_rc.clone();
                let drag_key_up = drag_key.clone();
                let current_value_up = current_value.clone();
                container = container.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    if let Some(state) = get_drag_state(&drag_key_up) {
                        let mut final_value = current_value_up.get();
                        if final_value == state.start_value {
                            final_value = scale.step_value(final_value, min, max, 1.0, 0.1);
                            current_value_up.set(final_value);
                            release_change(final_value, window, cx);
                        }
                        if let Some(ref commit) = release_commit {
                            commit(final_value, window, cx);
                        }
                    }
                    clear_drag_state(drag_key_up.clone());
                });
            }

            // Double-click - reset
            if let Some(ref reset_rc) = on_reset_rc {
                let reset_handler = reset_rc.clone();
                container = container.on_click(move |event, window, cx| {
                    if event.click_count() == 2 {
                        reset_handler(window, cx);
                    }
                });
            }

            // Keyboard navigation - register when focused (works on focus, not selection)
            // Register if either on_change or on_reset is provided
            if on_change_rc.is_some() || on_reset_rc.is_some() {
                let handler_key = on_change_rc.clone();
                let reset_key = on_reset_rc.clone();
                let current_value_key = current_value.clone();
                let config_key = interaction_config.clone();
                let commit_key = on_commit_rc.clone();
                container = container.on_key_down(move |event, window, cx| {
                    cx.stop_propagation();
                    let key = event.keystroke.key.as_str();
                    if key == "escape" {
                        if let Some(ref reset_handler) = reset_key {
                            reset_handler(window, cx);
                        }
                    } else if let Some(ref handler) = handler_key
                        && let Some(new_value) = handle_keyboard(
                            key,
                            &event.keystroke.modifiers,
                            current_value_key.get(),
                            &config_key,
                        )
                    {
                        current_value_key.set(new_value);
                        handler(new_value, window, cx);
                        if let Some(ref commit) = commit_key {
                            commit(new_value, window, cx);
                        }
                    }
                });
            }

            // Scroll wheel - adjust value
            if let Some(handler_rc) = on_change_rc {
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
                        handler_rc(new_value, window, cx);
                        if let Some(ref commit) = commit_scroll {
                            commit(new_value, window, cx);
                        }
                    }
                });
            }

            // Focus on mouse enter - keyboard follows hover like scroll wheel
            let focus_handle_hover = self.focus_handle.clone();
            container = container.on_mouse_move(move |event, window, cx| {
                if let Some(ref fh) = focus_handle_hover
                    && !fh.is_focused(window)
                    && event.pressed_button.is_none()
                {
                    fh.focus(window, cx);
                }
            });
        }

        // Label with keyboard shortcut.
        //
        // Underlined variant adds a thin rule beneath the title and a small
        // gap, mimicking the "title above + horizontal bar" mockup. When the
        // caller passes an empty label (e.g. the Xone view, where the cell
        // around the knob already supplies the parameter name as a header),
        // we skip the *entire* title+rule block so no vertical space is
        // reserved for an empty row of text.
        let has_label = !formatted_label.is_empty();
        if has_label {
            let label_text = div()
                .text_xs()
                .font_weight(if selected {
                    FontWeight::BOLD
                } else {
                    FontWeight::SEMIBOLD
                })
                .text_color(label_color)
                .text_center()
                .child(formatted_label);

            if underlined {
                let rule_color = if selected { theme.accent } else { theme.border };
                container = container.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_1()
                        .min_w(px(min_width))
                        .child(label_text)
                        // Thin underline rule — width keyed off the knob
                        // container so the rule lines up with the knob's
                        // bounding box.
                        .child(div().h(px(1.0)).w(px(min_width * 0.85)).bg(rule_color)),
                );
            } else {
                container = container.child(label_text);
            }
        }

        // Knob graphic with ticks. Horizontal labels still need room for the
        // widest tick text, but the top/bottom gutter can be much tighter:
        // just enough for the tick label's own line height plus a small gap.
        let label_line_h = 10.0_f32;
        let tick_label_pad = 1.5_f32;
        let tick_inner_radius = knob_size / 2.0; // Start at knob edge
        let major_tick_outer_radius = tick_inner_radius + 8.0; // Major ticks
        let minor_tick_outer_radius = tick_inner_radius + 5.0; // Minor ticks (shorter)
        let label_radius = major_tick_outer_radius + 8.0; // Labels outside ticks
        let horizontal_label_gutter = 44.0;
        let vertical_label_gutter = (label_radius - center + label_line_h + tick_label_pad).ceil();
        let container_width = knob_size + (horizontal_label_gutter * 2.0);
        let container_height = knob_size + (vertical_label_gutter * 2.0);
        let mut knob_container = div()
            .w(px(container_width))
            .h(px(container_height))
            .relative();

        // Create tick colors - use accent-based colors for visibility
        let major_tick_color = {
            let a = theme.accent;
            Rgba {
                r: a.r,
                g: a.g,
                b: a.b,
                a: if selected { 0.8 } else { 0.5 },
            }
        };
        let minor_tick_color = {
            let a = theme.accent;
            Rgba {
                r: a.r,
                g: a.g,
                b: a.b,
                a: if selected { 0.4 } else { 0.25 },
            }
        };

        // Cache tick geometry/labels by (min, max, scale, size, unit) and paint
        // all tick lines in a single custom element instead of one div per dot.
        let tick_geometry = get_tick_geometry(
            min,
            max,
            scale,
            self.size,
            &self.unit,
            self.design_tokens.knob_arc_start_deg,
            self.design_tokens.knob_arc_sweep_deg,
            knob_size,
            center,
            horizontal_label_gutter,
            vertical_label_gutter,
            container_width,
            container_height,
            major_tick_outer_radius,
            minor_tick_outer_radius,
            tick_inner_radius,
            label_radius,
        );

        knob_container = knob_container.child(PotentiometerTickLinesElement {
            container_width,
            container_height,
            major_tick_color,
            minor_tick_color,
            ticks: tick_geometry.ticks.clone(),
            major_tick_width: tick_geometry.major_tick_width,
            minor_tick_width: tick_geometry.minor_tick_width,
        });

        // Add cached tick labels as div children.
        let char_w = 5.4_f32;
        let line_h = label_line_h;
        let dead_zone = 0.30_f32;
        let pad = tick_label_pad;
        for (label_text, label_x, label_y) in tick_geometry.labels.iter() {
            let tick_angle = {
                let dx = label_x - tick_geometry.knob_offset_x - center;
                let dy = label_y - tick_geometry.knob_offset_y - center;
                dy.atan2(dx)
            };
            let text_w = (label_text.len() as f32) * char_w;
            let cos_a = tick_angle.cos();
            let sin_a = tick_angle.sin();

            let dx = if cos_a > dead_zone {
                pad
            } else if cos_a < -dead_zone {
                -text_w - pad
            } else {
                -text_w / 2.0
            };
            let dy = if sin_a > dead_zone {
                pad
            } else if sin_a < -dead_zone {
                -line_h - pad
            } else {
                -line_h / 2.0
            };

            knob_container = knob_container.child(
                div()
                    .absolute()
                    .left(px(label_x + dx))
                    .top(px(label_y + dy))
                    .text_size(px(9.0))
                    .text_color(major_tick_color)
                    .child(label_text.clone()),
            );
        }

        // Value arc — painted behind tick marks, around the knob
        let arc_color = self.accent_color.unwrap_or(theme.accent);
        let size_idx = match self.size {
            PotentiometerSize::Xs => 0,
            PotentiometerSize::Sm => 1,
            PotentiometerSize::Md => 2,
            PotentiometerSize::Lg => 3,
        };
        let arc_width = self.design_tokens.knob_arc_widths[size_idx];
        let track_arc_width = self.design_tokens.knob_arc_track_widths[size_idx].max(0.0);
        knob_container = knob_container.child(
            div().absolute().inset_0().child(KnobArcElement {
                container_width,
                container_height,
                knob_offset_x: tick_geometry.knob_offset_x,
                knob_offset_y: tick_geometry.knob_offset_y,
                knob_size,
                normalized,
                arc_color,
                arc_width,
                track_arc_width,
                arc_glow: self.design_tokens.knob_arc_glow,
                arc_start_rad: self.design_tokens.knob_arc_start_deg.to_radians(),
                arc_end_rad: (self.design_tokens.knob_arc_start_deg
                    + self.design_tokens.knob_arc_sweep_deg)
                    .to_radians(),
                arc_segments: self.design_tokens.knob_arc_segments,
            }),
        );

        // Knob circle (offset to center in larger container)
        // Use major_tick_color for border to match ticks and labels
        let knob_border = self.design_tokens.knob_border_width;
        let mut knob = div()
            .absolute()
            .left(px(tick_geometry.knob_offset_x))
            .top(px(tick_geometry.knob_offset_y))
            .w(px(knob_size))
            .h(px(knob_size))
            .rounded_full()
            .bg(knob_bg)
            .border(px(knob_border))
            .border_color(major_tick_color);

        if selected {
            knob = knob.shadow_sm();
            // Arc indicator when selected
            knob = knob.child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded_full()
                    .border_2()
                    .border_color(theme.accent_muted),
            );
        }

        // Indicator marker — shape configured by `knob_indicator_style`. The
        // dot rect is positioned at (x, y); for non-dot shapes we keep the
        // same anchor and width so existing geometry math stays meaningful.
        let mut indicator = div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(indicator_size))
            .h(px(indicator_size))
            .bg(indicator_color);

        match self.design_tokens.knob_indicator_style {
            crate::audio_design_tokens::AudioDesignTokens::INDICATOR_TICK => {
                // Radial tick: stretch toward the rim, narrow tangentially.
                // We approximate the radial direction by always extending the
                // marker outward from center; for the dead-zone-bottom layout
                // a simple square stretched 1.5x reads correctly at the
                // common quadrants and stays inside the bounding box.
                let tick_len = indicator_size * 1.6;
                let tick_thick = (indicator_size * 0.55).max(2.0);
                let tick_x = center + radius * angle_rad.cos() - (tick_thick / 2.0);
                let tick_y = center + radius * angle_rad.sin() - (tick_len / 2.0);
                indicator = div()
                    .absolute()
                    .left(px(tick_x))
                    .top(px(tick_y))
                    .w(px(tick_thick))
                    .h(px(tick_len))
                    .bg(indicator_color)
                    .rounded_sm();
            }
            crate::audio_design_tokens::AudioDesignTokens::INDICATOR_ARROW => {
                // Arrow approximation: small triangular cap rendered as a
                // tilted square (rotated 45° via overflow trick is not
                // available without transforms in GPUI's stable surface, so
                // approximate with a smaller, brighter rounded square at the
                // arc tip).
                let arrow_size = indicator_size * 0.85;
                let arrow_x = center + radius * angle_rad.cos() - (arrow_size / 2.0);
                let arrow_y = center + radius * angle_rad.sin() - (arrow_size / 2.0);
                indicator = div()
                    .absolute()
                    .left(px(arrow_x))
                    .top(px(arrow_y))
                    .w(px(arrow_size))
                    .h(px(arrow_size))
                    .bg(indicator_color)
                    .rounded_sm();
            }
            // INDICATOR_DOT (or any unknown — fall through to dot).
            _ => {
                indicator = indicator.rounded_full();
            }
        }

        // Add shiny shadow for Lg size and selected state
        indicator = match self.size {
            PotentiometerSize::Lg => indicator.shadow_md(), // Always shiny for Lg
            _ => indicator.when(selected, |d| d.shadow_sm()),
        };

        knob = knob.child(indicator);

        // Current value in center of knob
        let mut value_display = div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .font_weight(FontWeight::BOLD)
            .text_color(if selected { theme.accent } else { value_color });

        // Increase font size for large potentiometer
        value_display = match self.size {
            PotentiometerSize::Xs => value_display.text_xs(),
            PotentiometerSize::Sm => value_display.text_xs(),
            PotentiometerSize::Md => value_display.text_xs(),
            PotentiometerSize::Lg => value_display.text_sm(),
        };

        knob = knob.child(value_display.child(value_str_only));

        knob_container = knob_container.child(knob);

        // Unit label at 6 o'clock position (270° standard = 90° screen, bottom center)
        // Position it at the same radius as the tick labels
        if !unit_str.is_empty() {
            let unit_angle = std::f32::consts::PI * 0.5; // 90° in screen coordinates (6 o'clock)
            let unit_x = tick_geometry.knob_offset_x + center + label_radius * unit_angle.cos();
            let unit_y = tick_geometry.knob_offset_y + center + label_radius * unit_angle.sin();

            // Calculate approximate centering offset based on typical unit string lengths
            // "%" is 1 char, "Hz" is 2 chars, "dB" is 2 chars
            // At text_xs (12px), approximate char width is ~7px
            let estimated_width = unit_str.len() as f32 * 7.0;
            let center_offset_x = estimated_width / 2.0;

            knob_container = knob_container.child(
                div()
                    .absolute()
                    .left(px(unit_x - center_offset_x))
                    .top(px(unit_y - 14.0)) // Move up (was -6.0, now -6-sizeoffont to be closer to circle)
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if selected {
                        theme.accent
                    } else {
                        theme.text_secondary
                    })
                    .child(unit_str),
            );
        }

        container.child(knob_container)
    }
}

#[cfg(test)]
mod tests {
    use super::Potentiometer;
    use crate::{AudioScale as Scale, PotentiometerTheme};

    #[test]
    fn format_label_wraps_shortcut_key() {
        let pot = Potentiometer::new("test")
            .label("Frequency")
            .shortcut_key('f');
        assert_eq!(pot.format_label(), "[F]requency");

        let pot_unmatched = Potentiometer::new("test").label("Gain").shortcut_key('z');
        assert_eq!(pot_unmatched.format_label(), "[Z] Gain");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn format_value_only_respects_units() {
        let pot_pct = Potentiometer::new("test")
            .value(75.0)
            .min(0.0)
            .max(100.0)
            .unit("%");
        assert_eq!(pot_pct.format_value_only(), "75");

        let pot_hz = Potentiometer::new("test")
            .value(1000.0)
            .min(20.0)
            .max(20000.0)
            .unit("Hz");
        assert_eq!(pot_hz.format_value_only(), "1000");

        let pot_default = Potentiometer::new("test").value(3.14);
        assert_eq!(pot_default.format_value_only(), "3.1");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn format_value_includes_units() {
        let pot_pct = Potentiometer::new("test")
            .value(75.0)
            .min(0.0)
            .max(100.0)
            .unit("%");
        assert_eq!(pot_pct.format_value(), "75%");

        let pot_hz = Potentiometer::new("test")
            .value(1000.0)
            .min(20.0)
            .max(20000.0)
            .unit("Hz");
        assert_eq!(pot_hz.format_value(), "1000 Hz");

        let pot_ratio = Potentiometer::new("test")
            .value(4.0)
            .min(1.0)
            .max(20.0)
            .unit(":1");
        assert_eq!(pot_ratio.format_value(), "4.0:1");

        let pot_default = Potentiometer::new("test").value(3.14);
        assert_eq!(pot_default.format_value(), "3.1");
    }

    #[test]
    fn value_to_normalized_respects_scale() {
        let linear = Potentiometer::new("test")
            .value(50.0)
            .min(0.0)
            .max(100.0)
            .scale(Scale::Linear);
        assert!((linear.value_to_normalized(50.0) - 0.5).abs() < 1e-9);

        let log = Potentiometer::new("test")
            .value(1000.0)
            .min(20.0)
            .max(20000.0)
            .scale(Scale::Logarithmic);
        let norm = log.value_to_normalized(1000.0);
        assert!(norm > 0.0 && norm < 1.0);
    }

    #[test]
    fn accessibility_summary_describes_parameter_range() {
        let summary = Potentiometer::new("freq")
            .label("Frequency")
            .value(1000.0)
            .min(20.0)
            .max(20_000.0)
            .unit("Hz")
            .scale(Scale::Logarithmic)
            .selected(true)
            .accessibility_summary();

        assert_eq!(summary.control_type, "potentiometer");
        assert_eq!(summary.label, "Frequency");
        assert_eq!(summary.role, crate::accessibility::AriaRole::Slider);
        assert_eq!(summary.value_now, Some(1000.0));
        assert_eq!(summary.value_min, Some(20.0));
        assert_eq!(summary.value_max, Some(20_000.0));
        assert_eq!(summary.value_text, Some("1000 Hz".into()));
        assert_eq!(summary.scale, Some(Scale::Logarithmic));
        assert!(summary.selected);
        assert!(summary.description.contains("rotary slider"));
    }

    #[test]
    fn builder_setters_chain() {
        let _pot = Potentiometer::new("test")
            .value(50.0)
            .min(0.0)
            .max(100.0)
            .unit("%")
            .label("Gain")
            .shortcut_key('g')
            .size(crate::PotentiometerSize::Lg)
            .scale(Scale::Linear)
            .selected(true)
            .disabled(false)
            .on_change(|_val, _window, _cx| {})
            .on_commit(|_val, _window, _cx| {})
            .on_drag_start(|_pos, _val, _window, _cx| {})
            .on_select(|_window, _cx| {})
            .on_reset(|_window, _cx| {});
    }

    #[test]
    fn extended_builder_setters_chain() {
        let theme = PotentiometerTheme {
            surface: gpui::rgba(0x1a1a1aff),
            surface_hover: gpui::rgba(0x2a2a2aff),
            knob_bg: gpui::rgba(0x333333ff),
            accent: gpui::rgba(0xff6600ff),
            accent_muted: gpui::rgba(0xff660033),
            border: gpui::rgba(0x444444ff),
            text_secondary: gpui::rgba(0xaaaaaaff),
            text_primary: gpui::rgba(0xffffffff),
            text_muted: gpui::rgba(0x888888ff),
            text_on_accent: gpui::rgba(0xffffffff),
            background_secondary: gpui::rgba(0x222222ff),
        };
        let design = gpui_design::DesignSystem::neutral();

        let _pot = Potentiometer::new("extended")
            .theme(theme)
            .accent_color(gpui::rgba(0x00ff00ff))
            .design_tokens(crate::AudioDesignTokens::default())
            .design(design)
            .aria_label("Gain knob")
            .aria_role(crate::accessibility::AriaRole::Slider);
    }
}
