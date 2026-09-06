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
    InteractionConfig, ValueTracker, clear_drag_state, drag_has_moved, get_drag_state, handle_drag,
    handle_keyboard, handle_scroll, mark_drag_moved, store_drag_state, value_tracker,
};
use super::vertical_slider::take_if_enabled;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState};
use crate::audio_accessibility::{
    AudioAccessibilitySummary, normalized, range_description, value_text,
};
use crate::theme::ThemeExt;
use d3rs::render2d::{Renderer2D, VelloBackend};
use gpui::prelude::*;
use gpui::*;
use std::sync::Arc;

mod knob_arc_element;
mod potentiometer_size;
mod tick_element;
mod types;

pub use potentiometer_size::*;
pub use types::*;

use knob_arc_element::KnobArcElement;
use tick_element::{PotentiometerTickGeometry, PotentiometerTickLinesElement, get_tick_geometry};

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
    renderer_2d: Renderer2D,
    vello_backend: VelloBackend,
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
            renderer_2d: Renderer2D::default(),
            vello_backend: VelloBackend::default(),
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

/// Selection-aware colors resolved once per potentiometer render.
struct KnobPalette {
    bg: Rgba,
    border: Rgba,
    knob_bg: Rgba,
    label: Rgba,
    value: Rgba,
    indicator: Rgba,
    major_tick: Rgba,
    minor_tick: Rgba,
    hover_border: Rgba,
    hover_bg: Rgba,
}

impl KnobPalette {
    fn resolve(theme: &PotentiometerTheme, selected: bool, size: PotentiometerSize) -> Self {
        let accent = theme.accent;
        Self {
            bg: if selected {
                theme.accent_muted
            } else {
                theme.surface
            },
            border: if selected { theme.accent } else { theme.border },
            knob_bg: if selected {
                theme.surface_hover
            } else {
                theme.knob_bg
            },
            label: if selected {
                theme.accent
            } else {
                theme.text_secondary
            },
            value: if selected {
                theme.text_on_accent
            } else {
                theme.text_primary
            },
            // For Lg size or when selected, use accent color for visibility.
            indicator: if matches!(size, PotentiometerSize::Lg) || selected {
                theme.accent
            } else {
                theme.text_muted
            },
            major_tick: Rgba {
                r: accent.r,
                g: accent.g,
                b: accent.b,
                a: if selected { 0.8 } else { 0.5 },
            },
            minor_tick: Rgba {
                r: accent.r,
                g: accent.g,
                b: accent.b,
                a: if selected { 0.4 } else { 0.25 },
            },
            hover_border: theme.accent,
            hover_bg: theme.surface_hover,
        }
    }
}

/// Indicator dot size by knob size and selection state.
fn indicator_size_for(size: PotentiometerSize, selected: bool) -> f32 {
    match size {
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
    }
}

type SharedPotChange = std::rc::Rc<Box<dyn Fn(f64, &mut Window, &mut App) + 'static>>;
type SharedPotNotify = std::rc::Rc<Box<dyn Fn(&mut Window, &mut App) + 'static>>;
type SharedPotDragStart = std::rc::Rc<Box<dyn Fn(f32, f64, &mut Window, &mut App) + 'static>>;

/// Owned interaction wiring shared by the gesture and discrete builders.
struct PotHandlers {
    on_change: Option<SharedPotChange>,
    on_commit: Option<SharedPotChange>,
    on_reset: Option<SharedPotNotify>,
    on_select: Option<SharedPotNotify>,
    on_drag_start: Option<SharedPotDragStart>,
    current_value: ValueTracker,
    config: InteractionConfig,
    focus_handle: Option<FocusHandle>,
    drag_key: ElementId,
    value: f64,
    min: f64,
    max: f64,
    scale: PotentiometerScale,
    disabled: bool,
}

/// Copyable 2D renderer selection for custom-painted dial elements.
#[derive(Clone, Copy)]
struct DialRenderers {
    renderer_2d: Renderer2D,
    vello_backend: VelloBackend,
}

/// Dial geometry derived from size tokens and the normalized value.
struct DialMetrics {
    knob_size: f32,
    center: f32,
    radius: f32,
    start_rad: f32,
    end_rad: f32,
    angle_rad: f32,
    indicator_size: f32,
    label_radius: f32,
    container_width: f32,
    container_height: f32,
    horizontal_gutter: f32,
    vertical_gutter: f32,
}

/// Value-arc paint parameters.
struct ArcSpec {
    color: Rgba,
    width: f32,
    track_width: f32,
    glow: f32,
    start_rad: f32,
    end_rad: f32,
    segments: u32,
    normalized: f32,
}

impl Potentiometer {
    fn dial_metrics(&self, normalized: f32, selected: bool) -> DialMetrics {
        let knob_size = self.size.knob_size();
        let center = knob_size / 2.0;
        let start_rad: f32 = self.design_tokens.knob_arc_start_deg.to_radians();
        let end_rad: f32 = (self.design_tokens.knob_arc_start_deg
            + self.design_tokens.knob_arc_sweep_deg)
            .to_radians();
        // Tick ring radii and label gutters around the knob graphic.
        let tick_inner_radius = knob_size / 2.0;
        let major_tick_outer_radius = tick_inner_radius + 8.0;
        let label_radius = major_tick_outer_radius + 8.0;
        let horizontal_label_gutter = 44.0;
        let vertical_label_gutter = (label_radius - center + 10.0 + 1.5).ceil();
        DialMetrics {
            knob_size,
            center,
            radius: self.size.indicator_radius(),
            start_rad,
            end_rad,
            angle_rad: start_rad + (end_rad - start_rad) * normalized,
            indicator_size: indicator_size_for(self.size, selected),
            label_radius,
            container_width: knob_size + horizontal_label_gutter * 2.0,
            container_height: knob_size + vertical_label_gutter * 2.0,
            horizontal_gutter: horizontal_label_gutter,
            vertical_gutter: vertical_label_gutter,
        }
    }

    fn tick_geometry(&self, metrics: &DialMetrics) -> Arc<PotentiometerTickGeometry> {
        let tick_inner_radius = metrics.knob_size / 2.0;
        get_tick_geometry(
            self.min,
            self.max,
            self.scale,
            self.size,
            &self.unit,
            self.design_tokens.knob_arc_start_deg,
            self.design_tokens.knob_arc_sweep_deg,
            metrics.knob_size,
            metrics.center,
            metrics.horizontal_gutter,
            metrics.vertical_gutter,
            metrics.container_width,
            metrics.container_height,
            tick_inner_radius + 8.0,
            tick_inner_radius + 5.0,
            tick_inner_radius,
            metrics.label_radius,
        )
    }

    fn arc_spec(&self, metrics: &DialMetrics, arc_color: Rgba, normalized: f32) -> ArcSpec {
        let size_idx = match self.size {
            PotentiometerSize::Xs => 0,
            PotentiometerSize::Sm => 1,
            PotentiometerSize::Md => 2,
            PotentiometerSize::Lg => 3,
        };
        ArcSpec {
            color: arc_color,
            width: self.design_tokens.knob_arc_widths[size_idx],
            track_width: self.design_tokens.knob_arc_track_widths[size_idx].max(0.0),
            glow: self.design_tokens.knob_arc_glow,
            start_rad: metrics.start_rad,
            end_rad: metrics.end_rad,
            segments: self.design_tokens.knob_arc_segments,
            normalized,
        }
    }

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

struct ChassisSpec {
    id: ElementId,
    min_width: f32,
    underlined: bool,
    selected: bool,
    disabled: bool,
    focus_handle: Option<FocusHandle>,
}

impl ChassisSpec {
    fn new(
        id: ElementId,
        min_width: f32,
        underlined: bool,
        selected: bool,
        disabled: bool,
        focus_handle: Option<FocusHandle>,
    ) -> Self {
        Self {
            id,
            min_width,
            underlined,
            selected,
            disabled,
            focus_handle,
        }
    }
}

/// Boxed/underlined chassis plus focus, shadow, hover, and cursor styling.
fn build_pot_chassis(spec: ChassisSpec, palette: &KnobPalette) -> Stateful<Div> {
    let mut container = div()
        .id(spec.id)
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .min_w(px(spec.min_width));

    if spec.underlined {
        // No chassis fill or border — just the title rule + knob below.
        // Selection is communicated via the title color and weight.
    } else {
        container = container
            .p_2()
            .rounded_lg()
            .bg(palette.bg)
            .border_2()
            .border_color(palette.border);
    }

    // Track focus if handle provided. Both track_focus (for focus
    // observation) and focusable (for key events) are needed.
    if let Some(ref focus_handle) = spec.focus_handle {
        container = container.track_focus(focus_handle).focusable();
    }

    // Add shadow when selected (chassis only — underlined style stays flat).
    if spec.selected && !spec.underlined {
        container = container.shadow_md();
    }

    // Hover effect — only apply chassis-style hover when boxed; the
    // underlined variant relies on indicator/title color changes instead.
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

/// Title block with keyboard-shortcut label. Empty labels skip the entire
/// title+rule block so no vertical space is reserved for an empty row.
fn build_pot_title(
    formatted_label: SharedString,
    label_color: Rgba,
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
        .text_color(label_color)
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
                // Thin underline rule — width keyed off the knob
                // container so the rule lines up with the knob's
                // bounding box.
                .child(div().h(px(1.0)).w(px(min_width * 0.85)).bg(rule_color)),
        )
    } else {
        Some(label_text)
    }
}

fn build_tick_lines(
    geometry: &PotentiometerTickGeometry,
    metrics: &DialMetrics,
    palette: &KnobPalette,
    renderers: DialRenderers,
) -> PotentiometerTickLinesElement {
    PotentiometerTickLinesElement {
        id: ElementId::named_usize("potentiometer-ticks", 0),
        container_width: metrics.container_width,
        container_height: metrics.container_height,
        major_tick_color: palette.major_tick,
        minor_tick_color: palette.minor_tick,
        ticks: geometry.ticks.clone(),
        major_tick_width: geometry.major_tick_width,
        minor_tick_width: geometry.minor_tick_width,
        renderer_2d: renderers.renderer_2d,
        vello_backend: renderers.vello_backend,
        #[cfg(feature = "vello")]
        painter: d3rs::vello2d::VelloScenePainter::new().backend(renderers.vello_backend),
    }
}

/// Major tick labels positioned around the dial, anchored away from the
/// knob center by the same dead-zone rule as the inline render path.
fn tick_label_divs(geometry: &PotentiometerTickGeometry, center: f32, color: Rgba) -> Vec<Div> {
    let char_w = 5.4_f32;
    let line_h = 10.0_f32;
    let dead_zone = 0.30_f32;
    let pad = 1.5_f32;
    geometry
        .labels
        .iter()
        .map(|(label_text, label_x, label_y)| {
            let tick_angle = {
                let dx = label_x - geometry.knob_offset_x - center;
                let dy = label_y - geometry.knob_offset_y - center;
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

            div()
                .absolute()
                .left(px(label_x + dx))
                .top(px(label_y + dy))
                .text_size(px(9.0))
                .text_color(color)
                .child(label_text.clone())
        })
        .collect()
}

/// Value arc painted behind the tick marks, around the knob.
fn build_value_arc(
    geometry: &PotentiometerTickGeometry,
    metrics: &DialMetrics,
    spec: &ArcSpec,
    renderers: DialRenderers,
) -> Div {
    div().absolute().inset_0().child(KnobArcElement {
        id: ElementId::named_usize("potentiometer-arc", 0),
        container_width: metrics.container_width,
        container_height: metrics.container_height,
        knob_offset_x: geometry.knob_offset_x,
        knob_offset_y: geometry.knob_offset_y,
        knob_size: metrics.knob_size,
        normalized: spec.normalized,
        arc_color: spec.color,
        arc_width: spec.width,
        track_arc_width: spec.track_width,
        arc_glow: spec.glow,
        arc_start_rad: spec.start_rad,
        arc_end_rad: spec.end_rad,
        arc_segments: spec.segments,
        renderer_2d: renderers.renderer_2d,
        vello_backend: renderers.vello_backend,
        #[cfg(feature = "vello")]
        painter: d3rs::vello2d::VelloScenePainter::new().backend(renderers.vello_backend),
    })
}

/// Knob circle with the selected-state ring. Indicator and value display are
/// added by the caller.
fn build_knob_shell(
    geometry: &PotentiometerTickGeometry,
    metrics: &DialMetrics,
    bg: Rgba,
    border_width: f32,
    border_color: Rgba,
    ring: Option<Rgba>,
) -> Div {
    let mut knob = div()
        .absolute()
        .left(px(geometry.knob_offset_x))
        .top(px(geometry.knob_offset_y))
        .w(px(metrics.knob_size))
        .h(px(metrics.knob_size))
        .rounded_full()
        .bg(bg)
        .border(px(border_width))
        .border_color(border_color);

    if let Some(ring_color) = ring {
        knob = knob.shadow_sm();
        // Arc indicator when selected
        knob = knob.child(
            div()
                .absolute()
                .inset_0()
                .rounded_full()
                .border_2()
                .border_color(ring_color),
        );
    }
    knob
}

/// Indicator marker. The dot rect is positioned at the arc tip; non-dot
/// shapes keep the same anchor and width so geometry math stays meaningful.
fn build_indicator(
    metrics: &DialMetrics,
    color: Rgba,
    size: PotentiometerSize,
    selected: bool,
    style: u8,
) -> Div {
    let indicator_size = metrics.indicator_size;
    let x = metrics.center + metrics.radius * metrics.angle_rad.cos() - (indicator_size / 2.0);
    let y = metrics.center + metrics.radius * metrics.angle_rad.sin() - (indicator_size / 2.0);
    let mut indicator = div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(indicator_size))
        .h(px(indicator_size))
        .bg(color);

    match style {
        crate::audio_design_tokens::AudioDesignTokens::INDICATOR_TICK => {
            // Radial tick: stretch toward the rim, narrow tangentially.
            // We approximate the radial direction by always extending the
            // marker outward from center; for the dead-zone-bottom layout
            // a simple square stretched 1.5x reads correctly at the
            // common quadrants and stays inside the bounding box.
            let tick_len = indicator_size * 1.6;
            let tick_thick = (indicator_size * 0.55).max(2.0);
            let tick_x =
                metrics.center + metrics.radius * metrics.angle_rad.cos() - (tick_thick / 2.0);
            let tick_y =
                metrics.center + metrics.radius * metrics.angle_rad.sin() - (tick_len / 2.0);
            indicator = div()
                .absolute()
                .left(px(tick_x))
                .top(px(tick_y))
                .w(px(tick_thick))
                .h(px(tick_len))
                .bg(color)
                .rounded_sm();
        }
        crate::audio_design_tokens::AudioDesignTokens::INDICATOR_ARROW => {
            // Arrow approximation: small triangular cap rendered as a
            // tilted square (rotated 45° via overflow trick is not
            // available without transforms in GPUI's stable surface, so
            // approximate with a smaller, brighter rounded square at the
            // arc tip).
            let arrow_size = indicator_size * 0.85;
            let arrow_x =
                metrics.center + metrics.radius * metrics.angle_rad.cos() - (arrow_size / 2.0);
            let arrow_y =
                metrics.center + metrics.radius * metrics.angle_rad.sin() - (arrow_size / 2.0);
            indicator = div()
                .absolute()
                .left(px(arrow_x))
                .top(px(arrow_y))
                .w(px(arrow_size))
                .h(px(arrow_size))
                .bg(color)
                .rounded_sm();
        }
        // INDICATOR_DOT (or any unknown — fall through to dot).
        _ => {
            indicator = indicator.rounded_full();
        }
    }

    // Add shiny shadow for Lg size and selected state
    indicator = match size {
        PotentiometerSize::Lg => indicator.shadow_md(), // Always shiny for Lg
        _ => indicator.when(selected, |d| d.shadow_sm()),
    };
    indicator
}

/// Current value centered in the knob.
fn build_value_display(value_str: SharedString, color: Rgba, size: PotentiometerSize) -> Div {
    let mut value_display = div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .font_weight(FontWeight::BOLD)
        .text_color(color);

    // Increase font size for large potentiometer
    value_display = match size {
        PotentiometerSize::Xs => value_display.text_xs(),
        PotentiometerSize::Sm => value_display.text_xs(),
        PotentiometerSize::Md => value_display.text_xs(),
        PotentiometerSize::Lg => value_display.text_sm(),
    };
    value_display.child(value_str)
}

/// Unit label at the 6 o'clock position, at the tick-label radius.
fn build_unit_label(
    geometry: &PotentiometerTickGeometry,
    metrics: &DialMetrics,
    unit: SharedString,
    color: Rgba,
) -> Option<Div> {
    if unit.is_empty() {
        return None;
    }
    let unit_angle = std::f32::consts::PI * 0.5; // 90° in screen coordinates (6 o'clock)
    let unit_x = geometry.knob_offset_x + metrics.center + metrics.label_radius * unit_angle.cos();
    let unit_y = geometry.knob_offset_y + metrics.center + metrics.label_radius * unit_angle.sin();

    // Calculate approximate centering offset based on typical unit string lengths
    // "%" is 1 char, "Hz" is 2 chars, "dB" is 2 chars
    // At text_xs (12px), approximate char width is ~7px
    let estimated_width = unit.len() as f32 * 7.0;
    let center_offset_x = estimated_width / 2.0;

    Some(
        div()
            .absolute()
            .left(px(unit_x - center_offset_x))
            .top(px(unit_y - 14.0)) // Move up (was -6.0, now -6-sizeoffont to be closer to circle)
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(color)
            .child(unit),
    )
}

/// Press/drag/release gesture wiring (mouse down, move, up, up-out).
fn wire_pot_gesture(mut container: Stateful<Div>, h: &PotHandlers) -> Stateful<Div> {
    if h.disabled {
        return container;
    }
    // Mouse down - focus, select, and optionally start drag
    let on_select = h.on_select.clone();
    let on_drag_start = h.on_drag_start.clone();
    let on_change_click = h.on_change.clone();
    let drag_key_down = h.drag_key.clone();
    let current_value_down = h.current_value.clone();
    let focus_handle_click = h.focus_handle.clone();
    let value = h.value;

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
    if let Some(ref handler) = h.on_change {
        let drag_handler = handler.clone();
        let drag_key_move = h.drag_key.clone();
        let current_value_drag = h.current_value.clone();
        let config_drag = h.config.clone();
        container = container.on_mouse_move(move |event, window, cx| {
            if event.pressed_button == Some(MouseButton::Left)
                && let Some(state) = get_drag_state(&drag_key_move)
            {
                let position: f32 = event.position.y.into();
                if (position - state.start_pos).abs() > f32::EPSILON {
                    mark_drag_moved(&drag_key_move);
                }
                if let Some(new_value) = handle_drag(position, &state, &config_drag) {
                    current_value_drag.set(new_value);
                    drag_handler(new_value, window, cx);
                }
            }
        });
        let release_change = handler.clone();
        let release_commit = h.on_commit.clone();
        let drag_key_up = h.drag_key.clone();
        let current_value_up = h.current_value.clone();
        let min = h.min;
        let max = h.max;
        let scale = h.scale;
        container = container.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
            if get_drag_state(&drag_key_up).is_some() {
                let mut final_value = current_value_up.get();
                if !drag_has_moved(&drag_key_up) {
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

        // GPUI dispatches this capture-phase handler when release
        // occurs beyond the knob's hitbox. Finish the gesture there
        // as well so automation receives its commit and the retained
        // drag state cannot leak into a later gesture.
        let release_change_out = handler.clone();
        let release_commit_out = h.on_commit.clone();
        let drag_key_up_out = h.drag_key.clone();
        let current_value_up_out = h.current_value.clone();
        let config_up_out = h.config.clone();
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
                if drag_has_moved(&drag_key_up_out) {
                    if final_value != previous_value {
                        release_change_out(final_value, window, cx);
                    }
                    if let Some(ref commit) = release_commit_out {
                        commit(final_value, window, cx);
                    }
                }
            }
            clear_drag_state(drag_key_up_out.clone());
        });
    }
    container
}

/// Discrete interaction wiring: double-click reset, keyboard, scroll wheel.
fn wire_pot_discrete(mut container: Stateful<Div>, h: &PotHandlers) -> Stateful<Div> {
    if h.disabled {
        return container;
    }
    // Double-click - reset
    if let Some(ref reset_rc) = h.on_reset {
        let reset_handler = reset_rc.clone();
        container = container.on_click(move |event, window, cx| {
            if event.click_count() == 2 {
                reset_handler(window, cx);
            }
        });
    }

    // Keyboard navigation - register when focused (works on focus, not selection)
    // Register if either on_change or on_reset is provided
    if h.on_change.is_some() || h.on_reset.is_some() {
        let handler_key = h.on_change.clone();
        let reset_key = h.on_reset.clone();
        let current_value_key = h.current_value.clone();
        let config_key = h.config.clone();
        let commit_key = h.on_commit.clone();
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
    if let Some(handler_rc) = h.on_change.clone() {
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
                handler_rc(new_value, window, cx);
                if let Some(ref commit) = commit_scroll {
                    commit(new_value, window, cx);
                }
            }
        });
    }
    container
}

/// Knob graphic: container with cached tick lines/labels, value arc,
/// knob shell, indicator, and centered value display.
#[allow(clippy::too_many_arguments)]
fn build_knob_graphic(
    metrics: &DialMetrics,
    tick_geometry: &PotentiometerTickGeometry,
    palette: &KnobPalette,
    renderers: DialRenderers,
    arc_spec: &ArcSpec,
    selected: bool,
    accent: Rgba,
    accent_muted: Rgba,
    value_str_only: SharedString,
    size: PotentiometerSize,
    knob_border_width: f32,
    knob_indicator_style: u8,
) -> Div {
    let mut knob_container = div()
        .w(px(metrics.container_width))
        .h(px(metrics.container_height))
        .relative();

    // Cached tick geometry/labels by (min, max, scale, size, unit); all
    // tick lines paint in a single custom element instead of one div per dot.
    knob_container =
        knob_container.child(build_tick_lines(tick_geometry, metrics, palette, renderers));

    // Add cached tick labels as div children.
    for label_div in tick_label_divs(tick_geometry, metrics.center, palette.major_tick) {
        knob_container = knob_container.child(label_div);
    }

    // Value arc — painted behind tick marks, around the knob
    knob_container =
        knob_container.child(build_value_arc(tick_geometry, metrics, arc_spec, renderers));

    // Knob circle (offset to center in larger container).
    // The border matches ticks and labels.
    let ring = selected.then_some(accent_muted);
    let mut knob = build_knob_shell(
        tick_geometry,
        metrics,
        palette.knob_bg,
        knob_border_width,
        palette.major_tick,
        ring,
    );

    // Indicator marker + current value in center of knob.
    knob = knob.child(build_indicator(
        metrics,
        palette.indicator,
        size,
        selected,
        knob_indicator_style,
    ));
    let value_display_color = if selected { accent } else { palette.value };
    knob = knob.child(build_value_display(
        value_str_only,
        value_display_color,
        size,
    ));
    knob_container = knob_container.child(knob);
    knob_container
}

impl RenderOnce for Potentiometer {
    fn render(mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.register_accessible(cx);

        let global_theme = cx.theme();
        let default_theme = PotentiometerTheme::from(global_theme.as_ref());
        let theme = self.theme.clone().unwrap_or(default_theme);
        let selected = self.selected;
        let disabled = self.disabled;
        let palette = KnobPalette::resolve(&theme, selected, self.size);
        let size = self.size;
        let knob_border_width = self.design_tokens.knob_border_width;
        let knob_indicator_style = self.design_tokens.knob_indicator_style;

        // Use scale-aware normalization for indicator position
        let normalized = self.value_to_normalized(self.value) as f32;
        let metrics = self.dial_metrics(normalized, selected);
        let tick_geometry = self.tick_geometry(&metrics);
        let arc_color = self.accent_color.unwrap_or(theme.accent);
        let arc_spec = self.arc_spec(&metrics, arc_color, normalized);

        let formatted_label = self.formatted_label;
        let value_str_only = self.formatted_value_only;
        let unit_str = self.unit.clone();
        let min_width = self.size.min_width();

        // Capture values for closures
        let value = self.value;
        let min = self.min;
        let max = self.max;
        let scale = self.scale;

        // Shared current value tracker and interaction config
        let current_value = value_tracker(value);
        // Potentiometer uses rotational config (drag distance = knob_size for full range)
        let interaction_config = InteractionConfig::rotational(min, max, scale, metrics.knob_size);
        let drag_key = self.id.clone();

        // Layout style: boxed (chassis around the whole knob) vs underlined
        // (title above with a thin rule, no surrounding chassis).
        let underlined = self.design_tokens.knob_label_style
            == crate::audio_design_tokens::AudioDesignTokens::LABEL_UNDERLINED;

        let enabled = !disabled;
        let handlers = PotHandlers {
            on_change: take_if_enabled(enabled, self.on_change.take()).map(std::rc::Rc::new),
            on_commit: take_if_enabled(enabled, self.on_commit.take()).map(std::rc::Rc::new),
            on_reset: take_if_enabled(enabled, self.on_reset.take()).map(std::rc::Rc::new),
            on_select: take_if_enabled(enabled, self.on_select.take()).map(std::rc::Rc::new),
            on_drag_start: take_if_enabled(enabled, self.on_drag_start.take())
                .map(std::rc::Rc::new),
            current_value,
            config: interaction_config,
            focus_handle: self.focus_handle.clone(),
            drag_key,
            value,
            min,
            max,
            scale,
            disabled,
        };

        let chassis = ChassisSpec::new(
            self.id.clone(),
            min_width,
            underlined,
            selected,
            disabled,
            self.focus_handle.clone(),
        );
        let mut container = build_pot_chassis(chassis, &palette);
        container = wire_pot_gesture(container, &handlers);
        container = wire_pot_discrete(container, &handlers);

        // Title block (empty labels skip it entirely).
        let rule_color = if selected { theme.accent } else { theme.border };
        if let Some(title) = build_pot_title(
            formatted_label,
            palette.label,
            selected,
            min_width,
            underlined,
            rule_color,
        ) {
            container = container.child(title);
        }

        // Knob graphic with ticks. Horizontal labels still need room for the
        // widest tick text, but the top/bottom gutter stays tight: just the
        // tick label's own line height plus a small gap.
        let renderers = DialRenderers {
            renderer_2d: self.renderer_2d,
            vello_backend: self.vello_backend,
        };
        let mut knob_container = build_knob_graphic(
            &metrics,
            &tick_geometry,
            &palette,
            renderers,
            &arc_spec,
            selected,
            theme.accent,
            theme.accent_muted,
            value_str_only,
            size,
            knob_border_width,
            knob_indicator_style,
        );

        // Unit label at 6 o'clock position (270° standard = 90° screen,
        // bottom center), at the same radius as the tick labels.
        let unit_color = if selected {
            theme.accent
        } else {
            theme.text_secondary
        };
        if let Some(unit_label) = build_unit_label(&tick_geometry, &metrics, unit_str, unit_color) {
            knob_container = knob_container.child(unit_label);
        }

        container.child(knob_container)
    }
}

#[cfg(test)]
mod tests {
    use super::Potentiometer;
    use crate::{AudioScale as Scale, PotentiometerTheme};
    use d3rs::render2d::{Renderer2D, VelloBackend};

    #[test]
    fn default_renderer_contract_is_shared_with_d3rs() {
        let pot = Potentiometer::new("test");
        assert_eq!(pot.renderer_2d, Renderer2D::default());
        assert_eq!(pot.vello_backend, VelloBackend::default());
    }

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
    fn format_label_handles_multibyte_shortcuts() {
        // ASCII key inside a multibyte label: byte-safe bracketing.
        let pot = Potentiometer::new("test").label("Écho").shortcut_key('c');
        assert_eq!(pot.format_label(), "É[C]ho");

        // Exact non-ASCII key match still brackets the right char.
        let cjk = Potentiometer::new("test").label("音量").shortcut_key('量');
        assert_eq!(cjk.format_label(), "音[量]");

        // Non-ASCII key with no exact match falls back to the prefix form.
        let fallback = Potentiometer::new("test").label("Écho").shortcut_key('é');
        assert_eq!(fallback.format_label(), "[é] Écho");
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
    fn knob_shell_arc_and_ticks_share_one_center() {
        // Regression guard for the component-lab story (Frequency, 1000 Hz,
        // log scale, Lg): the knob face, value arc, and tick ring must stay
        // concentric in the graphic container.
        let pot = Potentiometer::new("test")
            .value(1000.0)
            .min(20.0)
            .max(20_000.0)
            .unit("Hz")
            .scale(Scale::Logarithmic)
            .size(crate::PotentiometerSize::Lg);
        let normalized = pot.value_to_normalized(1000.0) as f32;
        let metrics = pot.dial_metrics(normalized, false);
        let geometry = pot.tick_geometry(&metrics);

        // Knob shell center within the container.
        let shell_cx = geometry.knob_offset_x + metrics.center;
        let shell_cy = geometry.knob_offset_y + metrics.center;
        // Value-arc center uses the same offsets by construction.
        let arc_cx = geometry.knob_offset_x + metrics.knob_size / 2.0;
        let arc_cy = geometry.knob_offset_y + metrics.knob_size / 2.0;
        assert!((shell_cx - arc_cx).abs() < 1e-6);
        assert!((shell_cy - arc_cy).abs() < 1e-6);
        // Shell is centered in the container.
        assert!((shell_cx - metrics.container_width / 2.0).abs() < 1e-6);
        assert!((shell_cy - metrics.container_height / 2.0).abs() < 1e-6);
        // Tick ring is centered on the shell: every tick endpoint lies on
        // its ring circle about the shell center (a 270-degree sweep has a
        // dead zone, so ticks have no exact opposites to pair up).
        assert!(!geometry.ticks.is_empty());
        for tick in geometry.ticks.iter() {
            let inner_dist =
                ((tick.inner_x - shell_cx).powi(2) + (tick.inner_y - shell_cy).powi(2)).sqrt();
            let outer_dist =
                ((tick.outer_x - shell_cx).powi(2) + (tick.outer_y - shell_cy).powi(2)).sqrt();
            assert!((inner_dist - metrics.knob_size / 2.0).abs() < 1e-3);
            let expected_outer = if tick.is_major {
                metrics.knob_size / 2.0 + 8.0
            } else {
                metrics.knob_size / 2.0 + 5.0
            };
            assert!((outer_dist - expected_outer).abs() < 1e-3);
        }
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
