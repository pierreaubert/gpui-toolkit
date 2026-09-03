//! Vertical Slider component for audio plugin parameters
//!
//! A vertical slider with:
//! - Selection highlighting for plugin parameter editing
//! - Drag support: click and drag vertically to adjust value (delta-based)
//! - Scroll wheel adjustment (Shift for fine control: 0.5% vs 5%)
//! - Double-click to reset to default
//! - Keyboard navigation (when focused via click):
//!   - Arrow Up/Right: increase value (5%)
//!   - Arrow Down/Left: decrease value (5%)
//!   - Page Up: increase value (10%)
//!   - Page Down: decrease value (10%)
//!   - Home: set to minimum
//!   - End: set to maximum
//!   - Escape: reset to default
//! - Value display with units
//! - Keyboard shortcut hints
//! - Linear or logarithmic scale

use super::interactions::{
    InteractionConfig, ValueTracker, clear_drag_state, get_drag_state, handle_drag,
    handle_keyboard, handle_scroll, store_drag_state, value_tracker,
};
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState};
use crate::audio_accessibility::{
    AudioAccessibilitySummary, normalized, range_description, value_text,
};
use crate::scale::Scale;
use crate::theme::ThemeExt;
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static VERTICAL_SLIDER_FOCUS_HANDLES: RefCell<HashMap<ElementId, FocusHandle>> =
        RefCell::new(HashMap::new());
}

const VERTICAL_SLIDER_FOCUS_HANDLE_CAPACITY: usize = 256;

mod calculate;
mod misc;
mod types;
mod vertical_slider_size;

pub use types::*;
pub use vertical_slider_size::*;

use calculate::calculate_ticks;
use misc::format_value_abbrev;

/// A vertical slider component for audio plugin parameters
#[derive(IntoElement)]
pub struct VerticalSlider {
    id: ElementId,
    track_id: ElementId,
    value: f64,
    min: f64,
    max: f64,
    unit: SharedString,
    label: Option<SharedString>,
    shortcut_key: Option<char>,
    size: VerticalSliderSize,
    scale: Scale,
    custom_height: Option<f32>,
    show_ticks: bool,
    selected: bool,
    disabled: bool,
    /// Optional peak marker value (for audio peak indicators)
    peak: Option<f64>,
    theme: Option<VerticalSliderTheme>,
    /// Platform design tokens for track sizing.
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
    /// Cached formatted value display.
    formatted_value: SharedString,
    /// Cached min/max scale markers (no-ticks branch), refreshed by setters.
    formatted_min: SharedString,
    formatted_max: SharedString,
}

impl VerticalSlider {
    /// Create a new vertical slider with the given ID
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        let track_id = ElementId::Name(SharedString::from(format!("{}-track", id)));
        Self {
            id,
            track_id,
            value: 0.0,
            min: 0.0,
            max: 100.0,
            unit: "".into(),
            label: None,
            shortcut_key: None,
            size: VerticalSliderSize::default(),
            scale: Scale::default(),
            custom_height: None,
            show_ticks: false,
            selected: false,
            disabled: false,
            peak: None,
            theme: None,
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
            formatted_value: SharedString::default(),
            formatted_min: format_value_abbrev(0.0),
            formatted_max: format_value_abbrev(100.0),
        }
    }

    /// Convert a value to normalized position [0, 1] based on scale type
    fn value_to_normalized(&self, value: f64) -> f64 {
        self.scale.value_to_normalized(value, self.min, self.max)
    }

    /// Set the current value
    /// Note: The value is stored as-is and clamped at render time
    /// after min/max are known
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self.formatted_value = self.format_value();
        self
    }

    /// Set the minimum value
    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self.formatted_value = self.format_value();
        self.formatted_min = format_value_abbrev(min);
        self
    }

    /// Set the maximum value
    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self.formatted_value = self.format_value();
        self.formatted_max = format_value_abbrev(max);
        self
    }

    /// Set the unit label (e.g., "dB", "Hz", "%", ":1")
    pub fn unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = unit.into();
        self.formatted_value = self.format_value();
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

    /// Set the slider size
    pub fn size(mut self, size: VerticalSliderSize) -> Self {
        self.size = size;
        self
    }

    /// Set the value scale type (linear or logarithmic)
    ///
    /// Use `Logarithmic` for frequency parameters (e.g., 20Hz to 20kHz)
    /// where equal visual distances should represent equal ratios.
    ///
    /// Note: For logarithmic scale, min must be > 0.
    pub fn scale(mut self, scale: Scale) -> Self {
        self.scale = scale;
        self
    }

    /// Set a custom track height in pixels (overrides size preset)
    pub fn height(mut self, height: f32) -> Self {
        self.custom_height = Some(height);
        self
    }

    /// Enable tick marks along the track
    pub fn with_ticks(mut self) -> Self {
        self.show_ticks = true;
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

    /// Set an optional peak marker value
    ///
    /// When set, displays a thick horizontal line at the peak position.
    /// Useful for audio applications to show peak levels.
    /// The peak value should be in the same range as min/max.
    pub fn peak(mut self, peak: Option<f64>) -> Self {
        self.peak = peak;
        self
    }

    /// Set theme colors
    pub fn theme(mut self, theme: VerticalSliderTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set platform design tokens for track sizing.
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

    /// Set value change handler (called on scroll wheel)
    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Set a semantic commit handler fired on drag release or after a discrete change.
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
            .unwrap_or_else(|| "Vertical slider".into());
        let value = self.value.clamp(self.min, self.max);
        let value_text = value_text(value, &self.unit);
        let description = range_description(
            "vertical slider",
            &label,
            &value_text,
            self.min,
            self.max,
            self.disabled,
        );

        AudioAccessibilitySummary {
            control_type: "vertical_slider",
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
            peak_value: self.peak,
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
                if let Some(pos) = match_pos
                    && let Some(matched_char) = label[pos..].chars().next()
                {
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

    /// Format the value display
    fn format_value(&self) -> SharedString {
        let value = self.value.clamp(self.min, self.max);
        let unit = self.unit.as_ref();
        SharedString::new(if unit == ":1" {
            format!("{:.1}{}", value, unit)
        } else if unit == "%" {
            let percentage = if self.max > self.min {
                (value - self.min) / (self.max - self.min) * 100.0
            } else {
                0.0
            };
            format!("{:.0}{}", percentage, unit)
        } else if unit.is_empty() {
            format!("{:.1}", value)
        } else {
            format!("{:.1} {}", value, unit)
        })
    }
}

/// Selection-aware colors resolved once per slider render.
struct SliderPalette {
    bg: Rgba,
    border: Rgba,
    track_border: Rgba,
    label: Rgba,
    value_bg: Rgba,
    value: Rgba,
    track_bg: Rgba,
    thumb: Rgba,
    scale: Rgba,
    hover_border: Rgba,
    hover_bg: Rgba,
    thumb_height: f32,
}

impl SliderPalette {
    fn resolve(theme: &VerticalSliderTheme, selected: bool) -> Self {
        Self {
            bg: if selected {
                theme.accent_muted
            } else {
                theme.surface
            },
            border: if selected { theme.accent } else { theme.border },
            track_border: if selected { theme.accent } else { theme.border },
            label: if selected {
                theme.accent
            } else {
                theme.text_secondary
            },
            value_bg: if selected {
                theme.accent
            } else {
                theme.background_secondary
            },
            value: if selected {
                theme.text_on_accent
            } else {
                theme.text_primary
            },
            track_bg: if selected {
                theme.surface_hover
            } else {
                theme.track_bg
            },
            thumb: if selected {
                theme.text_on_accent
            } else {
                theme.accent
            },
            scale: if selected {
                theme.text_secondary
            } else {
                theme.text_muted
            },
            hover_border: theme.accent,
            hover_bg: theme.surface_hover,
            thumb_height: if selected { 8.0 } else { 6.0 },
        }
    }
}

type SharedChangeHandler = std::rc::Rc<Box<dyn Fn(f64, &mut Window, &mut App) + 'static>>;
type SharedNotifyHandler = std::rc::Rc<Box<dyn Fn(&mut Window, &mut App) + 'static>>;
type SharedDragStartHandler = std::rc::Rc<Box<dyn Fn(f32, f64, &mut Window, &mut App) + 'static>>;

/// Keep a taken handler only when the control is enabled (disabled
/// controls expose no pointer/keyboard wiring).
pub(crate) fn take_if_enabled<T>(enabled: bool, handler: Option<T>) -> Option<T> {
    handler.filter(|_| enabled)
}

/// Owned interaction wiring shared by the container and track builders.
/// Handler slots are `None` when the control is disabled, so wiring helpers
/// stay behavior-identical to the previous inline `if !disabled` gates.
struct SliderHandlers {
    on_change: Option<SharedChangeHandler>,
    on_commit: Option<SharedChangeHandler>,
    on_reset: Option<SharedNotifyHandler>,
    on_select: Option<SharedNotifyHandler>,
    on_drag_start: Option<SharedDragStartHandler>,
    current_value: ValueTracker,
    config: InteractionConfig,
    focus_handle: Option<FocusHandle>,
    element_id: ElementId,
    disabled: bool,
}

impl VerticalSlider {
    fn register_accessible(&self, cx: &mut App) {
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
    }
}

/// Resolve the focus handle: prefer an externally-provided one, else reuse
/// the thread-local registry entry for this element id.
fn resolve_slider_focus(
    element_id: &ElementId,
    external: Option<FocusHandle>,
    cx: &mut App,
) -> FocusHandle {
    external.unwrap_or_else(|| {
        VERTICAL_SLIDER_FOCUS_HANDLES.with(|handles| {
            let mut handles = handles.borrow_mut();
            if let Some(handle) = handles.get(element_id) {
                return handle.clone();
            }
            let handle = cx.focus_handle();
            if handles.len() >= VERTICAL_SLIDER_FOCUS_HANDLE_CAPACITY
                && let Some(evicted_id) = handles.keys().next().cloned()
            {
                handles.remove(&evicted_id);
            }
            handles.insert(element_id.clone(), handle.clone());
            handle
        })
    })
}

/// Boxed/underlined chassis plus focus, shadow, hover, and cursor styling.
struct ChassisSpec {
    id: ElementId,
    min_width: f32,
    underlined: bool,
    selected: bool,
    disabled: bool,
}

impl ChassisSpec {
    fn new(
        id: ElementId,
        min_width: f32,
        underlined: bool,
        selected: bool,
        disabled: bool,
    ) -> Self {
        Self {
            id,
            min_width,
            underlined,
            selected,
            disabled,
        }
    }
}

fn build_chassis(
    spec: &ChassisSpec,
    palette: &SliderPalette,
    focus_handle: &FocusHandle,
) -> Stateful<Div> {
    let mut container = div()
        .id(spec.id.clone())
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .min_w(px(spec.min_width));

    if !spec.underlined {
        container = container
            .p_2()
            .rounded_lg()
            .bg(palette.bg)
            .border_2()
            .border_color(palette.border);
    }

    // Both track_focus (for focus observation) and focusable (for key events)
    // are needed.
    container = container.track_focus(focus_handle).focusable();

    // Add shadow when selected (chassis-only).
    if spec.selected && !spec.underlined {
        container = container.shadow_md();
    }

    // Hover effect — chassis-only.
    if !spec.underlined {
        let hover_border = palette.hover_border;
        let hover_bg = palette.hover_bg;
        container = container.hover(|s| s.border_color(hover_border).bg(hover_bg));
    }

    // Cursor
    if spec.disabled {
        container = container.cursor_not_allowed().opacity(0.5);
    } else {
        container = container.cursor_ns_resize();
    }
    container
}

/// Title block with keyboard-shortcut label. Empty labels collapse the whole
/// block (the Xone hardware view passes "" so the cell owns the title row).
fn build_slider_title(
    formatted_label: SharedString,
    palette: &SliderPalette,
    selected: bool,
    min_width: f32,
    underlined: bool,
    rule_color: Rgba,
) -> Option<Div> {
    if formatted_label.is_empty() {
        return None;
    }
    let label_text = div()
        .text_xs()
        .font_weight(if selected {
            FontWeight::BOLD
        } else {
            FontWeight::SEMIBOLD
        })
        .text_color(palette.label)
        .text_center()
        .child(formatted_label);

    if underlined {
        Some(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .min_w(px(min_width))
                .child(label_text)
                .child(div().h(px(1.0)).w(px(min_width * 0.85)).bg(rule_color)),
        )
    } else {
        Some(label_text)
    }
}

fn build_value_badge(value_bg: Rgba, value_color: Rgba, value_str: SharedString) -> Div {
    div()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(value_bg)
        .text_xs()
        .font_weight(FontWeight::BOLD)
        .text_color(value_color)
        .child(value_str)
}

/// Track dimensions resolved from size tokens and the current value.
struct TrackDims {
    width: f32,
    height: f32,
    normalized: f32,
    peak: Option<f32>,
    corner: f32,
    glow: f32,
}

impl TrackDims {
    fn new(
        width: f32,
        height: f32,
        normalized: f32,
        peak: Option<f32>,
        corner: f32,
        glow: f32,
    ) -> Self {
        Self {
            width,
            height,
            normalized,
            peak,
            corner,
            glow,
        }
    }
}

fn slider_track_layout(
    dims: &TrackDims,
    palette: &SliderPalette,
    accent: Rgba,
    peak_color: Rgba,
    selected: bool,
) -> TrackLayout {
    TrackLayout {
        width: dims.width,
        height: dims.height,
        bg: palette.track_bg,
        border: palette.track_border,
        corner: dims.corner,
        bar_top_px: (dims.normalized * dims.height).max(0.0),
        fill: accent,
        glow: dims.glow,
        thumb: palette.thumb,
        thumb_height: palette.thumb_height,
        // Thumb extends beyond the track edges for a proper grab affordance.
        overhang: 3.0,
        bar_radius: dims.corner,
        peak: dims.peak,
        peak_color,
        selected,
    }
}

/// Geometry + colors for the track, fill, thumb, and peak marker.
struct TrackLayout {
    width: f32,
    height: f32,
    bg: Rgba,
    border: Rgba,
    corner: f32,
    bar_top_px: f32,
    fill: Rgba,
    glow: f32,
    thumb: Rgba,
    thumb_height: f32,
    overhang: f32,
    bar_radius: f32,
    peak: Option<f32>,
    peak_color: Rgba,
    selected: bool,
}

fn build_track_fill(layout: &TrackLayout) -> Div {
    let mut fill = div()
        .absolute()
        .bottom_0()
        .left_0()
        .right_0()
        .h(px(layout.bar_top_px))
        .bg(layout.fill)
        // Top corners stay square (the bar's top edge is the value reading);
        // bottom corners follow the meter corner-radius token.
        .rounded(px(0.0))
        .rounded_b(px(layout.bar_radius));
    if layout.glow > 0.0 {
        // Outer halo: same color as the fill, blurred, zero offset.
        let glow_color = Hsla::from(Rgba {
            r: layout.fill.r,
            g: layout.fill.g,
            b: layout.fill.b,
            a: (layout.fill.a * 0.55 * layout.glow).clamp(0.0, 1.0),
        });
        fill = fill.shadow(vec![BoxShadow {
            color: glow_color,
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(8.0 * layout.glow),
            spread_radius: px(2.0 * layout.glow),
            inset: false,
        }]);
    }
    fill
}

fn build_track_thumb(layout: &TrackLayout) -> Div {
    // Top edge at the value position so the bar's visible end coincides with
    // the tick. The thumb extends past the track sides via `overhang` so it
    // remains a clear grab affordance.
    let thumb_bottom_px = (layout.bar_top_px - layout.thumb_height).max(-layout.thumb_height);
    div()
        .absolute()
        .left(px(-layout.overhang))
        .w(px(layout.width + layout.overhang * 2.0))
        .bottom(px(thumb_bottom_px))
        .h(px(layout.thumb_height))
        .bg(layout.thumb)
        .rounded(px(layout.bar_radius))
        .shadow_sm()
}

fn build_peak_marker(layout: &TrackLayout) -> Option<Div> {
    let peak_pos = layout.peak?;
    let peak_thickness = 3.0_f32;
    let peak_bottom_px = (peak_pos * layout.height) - (peak_thickness / 2.0);
    Some(
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(peak_bottom_px))
            .h(px(peak_thickness))
            .bg(layout.peak_color),
    )
}

fn build_track(track_id: ElementId, layout: &TrackLayout) -> Stateful<Div> {
    let mut track = div()
        .id(track_id)
        .w(px(layout.width))
        .h(px(layout.height))
        .bg(layout.bg)
        .rounded(px(layout.corner))
        .border_2()
        .border_color(layout.border)
        .relative()
        .overflow_hidden()
        .cursor_ns_resize();

    if layout.selected {
        track = track.shadow_sm();
    }

    track = track.child(build_track_fill(layout));
    track = track.child(build_track_thumb(layout));
    if let Some(marker) = build_peak_marker(layout) {
        track = track.child(marker);
    }
    track
}

fn wire_container_handlers(mut container: Stateful<Div>, h: &SliderHandlers) -> Stateful<Div> {
    if h.disabled {
        return container;
    }
    // Mouse down on container - focus, select, and external drag start
    let on_select_container = h.on_select.clone();
    let on_drag_start = h.on_drag_start.clone();
    let current_value_container = h.current_value.clone();
    let focus_handle_container = h.focus_handle.clone();

    container = container.on_mouse_down(MouseButton::Left, move |event, window, cx| {
        // Focus for keyboard navigation (focus follows click)
        if let Some(ref fh) = focus_handle_container {
            fh.focus(window, cx);
        }

        if let Some(ref handler) = on_select_container {
            handler(window, cx);
        }
        if let Some(ref handler) = on_drag_start {
            let val = current_value_container.get();
            handler(event.position.y.into(), val, window, cx);
        }
    });

    // Double-click - reset
    if let Some(ref reset_rc) = h.on_reset {
        let reset_handler = reset_rc.clone();
        container = container.on_click(move |event, window, cx| {
            if event.click_count() == 2 {
                reset_handler(window, cx);
            }
        });
    }

    // Scroll wheel - adjust value (Shift for fine control)
    if let Some(ref handler_rc) = h.on_change {
        let handler_scroll = handler_rc.clone();
        let current_value_scroll = h.current_value.clone();
        let config_scroll = h.config.clone();
        let commit_scroll = h.on_commit.clone();
        container = container.on_scroll_wheel(move |event, window, cx| {
            cx.stop_propagation();
            let val = current_value_scroll.get();
            if let Some(new_value) =
                handle_scroll(&event.delta, &event.modifiers, val, &config_scroll)
            {
                current_value_scroll.set(new_value);
                handler_scroll(new_value, window, cx);
                if let Some(ref commit) = commit_scroll {
                    commit(new_value, window, cx);
                }
            }
        });
    }

    // Keyboard navigation - register on container (which has track_focus)
    if h.on_change.is_some() || h.on_reset.is_some() {
        let handler_key = h.on_change.clone();
        let reset_key = h.on_reset.clone();
        let current_value_key = h.current_value.clone();
        let config_key = h.config.clone();
        let commit_key = h.on_commit.clone();
        container = container.on_key_down(move |event, window, cx| {
            cx.stop_propagation();
            let key = event.keystroke.key.as_str();

            // Escape resets to default
            if key == "escape" {
                if let Some(ref reset_handler) = reset_key {
                    reset_handler(window, cx);
                }
                return;
            }

            // Arrow keys and other navigation
            if let Some(ref handler) = handler_key {
                let val = current_value_key.get();
                if let Some(new_value) =
                    handle_keyboard(key, &event.keystroke.modifiers, val, &config_key)
                {
                    current_value_key.set(new_value);
                    handler(new_value, window, cx);
                    if let Some(ref commit) = commit_key {
                        commit(new_value, window, cx);
                    }
                }
            }
        });
    }
    container
}

fn wire_track_press_handlers(mut track: Stateful<Div>, h: &SliderHandlers) -> Stateful<Div> {
    if h.disabled {
        return track;
    }
    // Create a unique key for this slider's drag state (survives re-renders)
    let drag_key_down = h.element_id.clone();

    // Mouse down - focus, select, and start drag
    let on_select_track = h.on_select.clone();
    let on_drag_start_track = h.on_drag_start.clone();
    let current_value_at_click = h.current_value.clone();
    let has_change_handler = h.on_change.is_some();
    let focus_handle_track = h.focus_handle.clone();
    track = track.on_mouse_down(MouseButton::Left, move |event, window, cx| {
        // The track owns this gesture and stops bubbling after it has
        // invoked the same focus, selection, and drag-start hooks as
        // the surrounding control.

        // Focus for keyboard navigation (focus follows click)
        if let Some(ref fh) = focus_handle_track {
            fh.focus(window, cx);
        }

        // Select the slider (if handler provided)
        if let Some(ref handler) = on_select_track {
            handler(window, cx);
        }

        if let Some(ref handler) = on_drag_start_track {
            handler(
                event.position.y.into(),
                current_value_at_click.get(),
                window,
                cx,
            );
        }

        // Store drag state only if we have a change handler
        if has_change_handler {
            let click_pos: f32 = event.position.y.into();
            store_drag_state(
                drag_key_down.clone(),
                click_pos,
                current_value_at_click.get(),
            );
        }

        cx.stop_propagation();
    });

    // Double-clicking the track resets once rather than bubbling to
    // the container's reset handler too.
    if let Some(ref reset_rc) = h.on_reset {
        let reset_handler = reset_rc.clone();
        track = track.on_click(move |event, window, cx| {
            if event.click_count() == 2 {
                reset_handler(window, cx);
            }
            cx.stop_propagation();
        });
    }
    track
}

fn wire_track_drag_handlers(mut track: Stateful<Div>, h: &SliderHandlers) -> Stateful<Div> {
    if h.disabled {
        return track;
    }
    // Drag and scroll handlers (only if on_change is set)
    if let Some(ref handler_rc) = h.on_change {
        let drag_key_move = h.element_id.clone();
        // Mouse move while pressed - drag to change value
        let handler_drag = handler_rc.clone();
        let current_value_drag = h.current_value.clone();
        let config_drag = h.config.clone();
        track = track.on_mouse_move(move |event, window, cx| {
            if event.pressed_button == Some(MouseButton::Left)
                && let Some(state) = get_drag_state(&drag_key_move)
            {
                let current_pos: f32 = event.position.y.into();
                if let Some(new_value) = handle_drag(current_pos, &state, &config_drag) {
                    current_value_drag.set(new_value);
                    handler_drag(new_value, window, cx);
                }
            }
        });

        // Mouse up - clear drag state
        let drag_key_up = h.element_id.clone();
        let commit_drag = h.on_commit.clone();
        let current_value_up = h.current_value.clone();
        track = track.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
            if get_drag_state(&drag_key_up).is_some()
                && let Some(ref commit) = commit_drag
            {
                commit(current_value_up.get(), window, cx);
            }
            clear_drag_state(drag_key_up.clone());
        });

        // Capture a release beyond the narrow track. Without this,
        // the final automation commit is lost and retained drag state
        // survives until the next press on this slider.
        let handler_drag_out = handler_rc.clone();
        let commit_drag_out = h.on_commit.clone();
        let current_value_up_out = h.current_value.clone();
        let config_up_out = h.config.clone();
        let drag_key_up_out = h.element_id.clone();
        track = track.on_mouse_up_out(MouseButton::Left, move |event, window, cx| {
            if let Some(state) = get_drag_state(&drag_key_up_out) {
                let position: f32 = event.position.y.into();
                let previous_value = current_value_up_out.get();
                let final_value =
                    handle_drag(position, &state, &config_up_out).unwrap_or(previous_value);
                current_value_up_out.set(final_value);
                if final_value != previous_value {
                    handler_drag_out(final_value, window, cx);
                }
                if let Some(ref commit) = commit_drag_out {
                    commit(final_value, window, cx);
                }
            }
            clear_drag_state(drag_key_up_out.clone());
        });

        // Scroll wheel handler on track
        let handler_scroll_track = handler_rc.clone();
        let current_value_track_scroll = h.current_value.clone();
        let config_track_scroll = h.config.clone();
        let commit_track_scroll = h.on_commit.clone();
        track = track.on_scroll_wheel(move |event, window, cx| {
            cx.stop_propagation();
            let val = current_value_track_scroll.get();
            if let Some(new_value) =
                handle_scroll(&event.delta, &event.modifiers, val, &config_track_scroll)
            {
                current_value_track_scroll.set(new_value);
                handler_scroll_track(new_value, window, cx);
                if let Some(ref commit) = commit_track_scroll {
                    commit(new_value, window, cx);
                }
            }
        });
    }
    track
}

/// Track flanked by tick columns, or the bare track plus min/max markers.
fn build_track_with_ticks(
    track: Stateful<Div>,
    ticks: &[TickMark],
    track_height: f32,
    scale_color: Rgba,
) -> Div {
    // Calculate label width for alignment (find widest label)
    let label_width = ticks
        .iter()
        .filter_map(|t| t.label.as_ref())
        .map(|l| l.len())
        .max()
        .unwrap_or(2) as f32
        * 7.0; // Approximate character width

    let tick_mark_width = 6.0_f32; // Major tick width
    let label_tick_gap = 3.0_f32; // Gap between label and tick
    let label_height = 12.0_f32; // Approximate label height for centering

    // Build tick marks container with absolute positioning
    // Height matches track exactly for proper alignment
    let mut ticks_container = div()
        .relative()
        .h(px(track_height))
        .w(px(label_width + label_tick_gap + tick_mark_width));

    for tick in ticks.iter() {
        let pos = tick.normalized_pos as f32;
        let tick_width = if tick.is_major { 6.0 } else { 3.0 };

        // Calculate pixel position from top (inverted: 0=bottom, 1=top)
        // pos=0 should be at bottom (top = track_height - label_height/2)
        // pos=1 should be at top (top = -label_height/2)
        let top_pos = (1.0 - pos) * track_height - label_height / 2.0;

        // Create a tick row positioned from top, centered vertically
        let tick_element = div()
            .absolute()
            .top(px(top_pos))
            .right_0()
            .h(px(label_height))
            .flex()
            .items_center()
            .gap(px(label_tick_gap))
            // Add label for major ticks
            .when(tick.label.is_some(), |d| {
                d.child(
                    div()
                        .text_xs()
                        .text_color(scale_color)
                        .min_w(px(label_width))
                        .text_right()
                        .child(tick.label.clone().unwrap_or_default()),
                )
            })
            // Tick mark
            .child(div().w(px(tick_width)).h(px(1.0)).bg(scale_color));

        ticks_container = ticks_container.child(tick_element);
    }

    // Build right-side tick marks (no labels, just tick marks)
    let mut ticks_right = div().relative().h(px(track_height)).w(px(tick_mark_width));

    for tick in ticks.iter() {
        let pos = tick.normalized_pos as f32;
        let tick_width = if tick.is_major { 6.0 } else { 3.0 };
        let top_pos = (1.0 - pos) * track_height - label_height / 2.0;

        let tick_element = div()
            .absolute()
            .top(px(top_pos))
            .left_0()
            .h(px(label_height))
            .flex()
            .items_center()
            .child(div().w(px(tick_width)).h(px(1.0)).bg(scale_color));

        ticks_right = ticks_right.child(tick_element);
    }

    // Wrap track and ticks in HStack (left ticks - track - right ticks)
    div()
        .flex()
        .items_center()
        .gap(px(2.0))
        .child(ticks_container)
        .child(track)
        .child(ticks_right)
}

fn build_scale_markers(min_label: SharedString, max_label: SharedString, scale_color: Rgba) -> Div {
    // Scale markers (only when not showing ticks) - use abbreviated format
    div()
        .flex()
        .justify_between()
        .w_full()
        .text_xs()
        .text_color(scale_color)
        .child(min_label)
        .child(max_label)
}

impl RenderOnce for VerticalSlider {
    fn render(mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Clamp before formatting and accessibility registration so the
        // displayed value, slider position, and ARIA range remain consistent.
        self.value = self.value.clamp(self.min, self.max);
        self.formatted_value = self.format_value();

        self.register_accessible(cx);

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| VerticalSliderTheme::from(global_theme.as_ref()));
        let selected = self.selected;
        let disabled = self.disabled;

        // Use scale-aware normalization for slider position
        let normalized = self.value_to_normalized(self.value) as f32;

        // Calculate peak normalized position (if peak is set)
        let peak_normalized = self.peak.map(|peak_value| {
            let clamped_peak = peak_value.clamp(self.min, self.max);
            self.value_to_normalized(clamped_peak) as f32
        });

        let formatted_label = self.formatted_label;
        let value_str = self.formatted_value;
        let formatted_min = self.formatted_min;
        let formatted_max = self.formatted_max;

        let track_width = self.size.track_width(&self.design_tokens);
        let track_height = self
            .custom_height
            .unwrap_or_else(|| self.size.track_height());
        let min_width = self.size.min_width();
        let show_ticks = self.show_ticks;

        // Calculate ticks based on scale type and available height
        let ticks = calculate_ticks(self.min, self.max, self.scale, track_height);

        let palette = SliderPalette::resolve(&theme, selected);

        // Capture values for closures
        let value = self.value;
        let min = self.min;
        let max = self.max;
        let scale = self.scale;
        let element_id = self.id.clone(); // Clone for use in track ID

        // Layout style: boxed (chassis around the whole slider) vs underlined
        // (title with thin rule, no surrounding chassis). Mirrors the
        // Potentiometer convention so the two controls stay visually paired.
        let underlined = self.design_tokens.meter_label_style
            == crate::audio_design_tokens::AudioDesignTokens::LABEL_UNDERLINED;

        let focus_handle = resolve_slider_focus(&element_id, self.focus_handle.clone(), cx);
        let mut container = build_chassis(
            &ChassisSpec::new(self.id.clone(), min_width, underlined, selected, disabled),
            &palette,
            &focus_handle,
        );

        // Shared current value tracker and interaction config
        let current_value = value_tracker(value);
        let interaction_config = InteractionConfig::vertical(min, max, scale, track_height);

        // Wrap handlers in Rc for sharing between container and track builders.
        // Slots stay `None` when disabled, matching the old `if !disabled` gates.
        let enabled = !disabled;
        let handlers = SliderHandlers {
            on_change: take_if_enabled(enabled, self.on_change.take()).map(std::rc::Rc::new),
            on_commit: take_if_enabled(enabled, self.on_commit.take()).map(std::rc::Rc::new),
            on_reset: take_if_enabled(enabled, self.on_reset.take()).map(std::rc::Rc::new),
            on_select: take_if_enabled(enabled, self.on_select.take()).map(std::rc::Rc::new),
            on_drag_start: take_if_enabled(enabled, self.on_drag_start.take())
                .map(std::rc::Rc::new),
            current_value,
            config: interaction_config,
            focus_handle: Some(focus_handle.clone()),
            element_id: element_id.clone(),
            disabled,
        };

        container = wire_container_handlers(container, &handlers);

        // Title block (empty labels collapse it) plus the value badge.
        let rule_color = if selected { theme.accent } else { theme.border };
        if let Some(title) = build_slider_title(
            formatted_label,
            &palette,
            selected,
            min_width,
            underlined,
            rule_color,
        ) {
            container = container.child(title);
        }
        container = container.child(build_value_badge(
            palette.value_bg,
            palette.value,
            value_str,
        ));

        // Track ID for click-to-position handling
        let track_id = self.track_id.clone();

        // Track corners follow the meter corner-radius design token so a
        // brutalist preset can render a square-cornered fader and a softer
        // preset keeps the rounded look.
        let track_corner_px = self
            .design_tokens
            .meter_corner_radius
            .clamp(0.0, (track_width / 2.0).min(8.0));
        let layout = slider_track_layout(
            &TrackDims::new(
                track_width,
                track_height,
                normalized,
                peak_normalized,
                track_corner_px,
                self.design_tokens.meter_glow.clamp(0.0, 1.0),
            ),
            &palette,
            theme.accent,
            theme.peak_marker,
            selected,
        );
        let mut track = build_track(track_id, &layout);
        track = wire_track_press_handlers(track, &handlers);
        track = wire_track_drag_handlers(track, &handlers);

        // Track with optional tick marks, else min/max scale markers.
        if show_ticks {
            container = container.child(build_track_with_ticks(
                track,
                &ticks,
                track_height,
                palette.scale,
            ));
        } else {
            container = container.child(track);
            container = container.child(build_scale_markers(
                formatted_min,
                formatted_max,
                palette.scale,
            ));
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::VerticalSlider;
    use crate::{AudioScale as Scale, VerticalSliderTheme};

    #[test]
    fn format_label_wraps_shortcut_key() {
        let slider = VerticalSlider::new("test")
            .label("Volume")
            .shortcut_key('v');
        assert_eq!(slider.format_label(), "[V]olume");

        let slider_unmatched = VerticalSlider::new("test").label("Gain").shortcut_key('z');
        assert_eq!(slider_unmatched.format_label(), "[Z] Gain");
    }

    #[test]
    fn format_label_handles_multibyte_shortcuts() {
        // ASCII key inside a multibyte label: byte-safe bracketing.
        let slider = VerticalSlider::new("test").label("Écho").shortcut_key('c');
        assert_eq!(slider.format_label(), "É[C]ho");

        // Exact non-ASCII key match still brackets the right char.
        let cjk = VerticalSlider::new("test").label("音量").shortcut_key('量');
        assert_eq!(cjk.format_label(), "音[量]");

        // Non-ASCII key with no exact match falls back to the prefix form.
        let fallback = VerticalSlider::new("test").label("Écho").shortcut_key('é');
        assert_eq!(fallback.format_label(), "[é] Écho");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn format_value_respects_units() {
        let slider_pct = VerticalSlider::new("test")
            .value(0.75)
            .min(0.0)
            .max(1.0)
            .unit("%");
        assert_eq!(slider_pct.format_value(), "75%");

        let slider_hz = VerticalSlider::new("test")
            .value(1000.0)
            .min(20.0)
            .max(20_000.0)
            .unit("Hz");
        assert_eq!(slider_hz.format_value(), "1000.0 Hz");

        let slider_default = VerticalSlider::new("test").value(3.14);
        assert_eq!(slider_default.format_value(), "3.1");

        let slider_ratio = VerticalSlider::new("test").value(4.0).unit(":1");
        assert_eq!(slider_ratio.format_value(), "4.0:1");
    }

    #[test]
    fn format_value_clamps_and_uses_range_relative_percentages() {
        let out_of_range = VerticalSlider::new("test")
            .value(150.0)
            .min(0.0)
            .max(100.0)
            .unit("%");
        assert_eq!(out_of_range.format_value(), "100%");

        let offset_range = VerticalSlider::new("test")
            .value(50.0)
            .min(20.0)
            .max(120.0)
            .unit("%");
        assert_eq!(offset_range.format_value(), "30%");
    }

    #[test]
    fn calculate_ticks_is_cached() {
        use super::calculate::calculate_ticks;

        let ticks_a = calculate_ticks(0.0, 100.0, Scale::Linear, 160.0);
        let ticks_b = calculate_ticks(0.0, 100.0, Scale::Linear, 160.0);
        assert_eq!(ticks_a.len(), ticks_b.len());
        assert!(ticks_a.iter().zip(ticks_b.iter()).all(|(a, b)| {
            a.normalized_pos == b.normalized_pos && a.is_major == b.is_major && a.label == b.label
        }));
    }

    #[test]
    fn value_to_normalized_respects_scale() {
        let linear = VerticalSlider::new("test")
            .value(50.0)
            .min(0.0)
            .max(100.0)
            .scale(Scale::Linear);
        assert!((linear.value_to_normalized(50.0) - 0.5).abs() < 1e-9);

        let log = VerticalSlider::new("test")
            .value(1000.0)
            .min(20.0)
            .max(20_000.0)
            .scale(Scale::Logarithmic);
        let norm = log.value_to_normalized(1000.0);
        assert!(norm > 0.0 && norm < 1.0);
    }

    #[test]
    fn accessibility_summary_includes_peak_and_disabled_state() {
        let summary = VerticalSlider::new("gain")
            .aria_label("Input gain")
            .value(6.0)
            .min(-60.0)
            .max(12.0)
            .unit("dB")
            .peak(Some(9.0))
            .disabled(true)
            .accessibility_summary();

        assert_eq!(summary.control_type, "vertical_slider");
        assert_eq!(summary.label, "Input gain");
        assert_eq!(summary.role, crate::accessibility::AriaRole::Slider);
        assert_eq!(summary.value_now, Some(6.0));
        assert_eq!(summary.value_min, Some(-60.0));
        assert_eq!(summary.value_max, Some(12.0));
        assert_eq!(summary.value_text, Some("6.0 dB".into()));
        assert_eq!(summary.peak_value, Some(9.0));
        assert!(summary.disabled);
        assert!(summary.description.contains("Disabled"));
    }

    #[test]
    fn builder_setters_chain() {
        let _slider = VerticalSlider::new("test")
            .value(50.0)
            .min(0.0)
            .max(100.0)
            .unit("%")
            .label("Gain")
            .shortcut_key('g')
            .size(crate::VerticalSliderSize::Lg)
            .scale(Scale::Linear)
            .height(200.0)
            .with_ticks()
            .selected(true)
            .disabled(false)
            .peak(Some(80.0))
            .on_change(|_val, _window, _cx| {})
            .on_commit(|_val, _window, _cx| {})
            .on_drag_start(|_pos, _val, _window, _cx| {})
            .on_select(|_window, _cx| {})
            .on_reset(|_window, _cx| {});
    }

    #[test]
    fn extended_builder_setters_chain() {
        let theme = VerticalSliderTheme {
            surface: gpui::rgba(0x1a1a1aff),
            surface_hover: gpui::rgba(0x2a2a2aff),
            track_bg: gpui::rgba(0x111111ff),
            accent: gpui::rgba(0xff6600ff),
            accent_muted: gpui::rgba(0xff660033),
            border: gpui::rgba(0x444444ff),
            text_secondary: gpui::rgba(0xaaaaaaff),
            text_primary: gpui::rgba(0xffffffff),
            text_muted: gpui::rgba(0x888888ff),
            text_on_accent: gpui::rgba(0xffffffff),
            background_secondary: gpui::rgba(0x222222ff),
            peak_marker: gpui::rgba(0xff0000ff),
        };
        let design = gpui_design::DesignSystem::neutral();

        let _slider = VerticalSlider::new("extended")
            .theme(theme)
            .design_tokens(crate::AudioDesignTokens::default())
            .design(design)
            .aria_label("Gain slider")
            .aria_role(crate::accessibility::AriaRole::Slider);
    }
}
