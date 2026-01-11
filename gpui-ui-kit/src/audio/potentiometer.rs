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

use super::interactions::{InteractionConfig, handle_keyboard, handle_scroll, value_tracker};
use crate::ComponentTheme;
use crate::scale::Scale;
use crate::theme::ThemeExt;
use gpui::prelude::*;
use gpui::*;

/// Theme colors for potentiometer styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct PotentiometerTheme {
    /// Background color of the container
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub surface: Rgba,
    /// Surface hover color
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub surface_hover: Rgba,
    /// Knob background color
    #[theme(default = 0x1a1a1aff, from = muted)]
    pub knob_bg: Rgba,
    /// Accent color
    #[theme(default = 0x007accff, from = accent)]
    pub accent: Rgba,
    /// Accent muted (for selection background)
    #[theme(
        default = 0x007acc33,
        from_expr = "Rgba { r: theme.accent.r, g: theme.accent.g, b: theme.accent.b, a: 0.2 }"
    )]
    pub accent_muted: Rgba,
    /// Border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Label text color
    #[theme(default = 0xaaaaaaff, from = text_secondary)]
    pub text_secondary: Rgba,
    /// Value text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_primary: Rgba,
    /// Muted text color (for indicator when not selected)
    #[theme(default = 0x888888ff, from = text_muted)]
    pub text_muted: Rgba,
    /// Text on accent background
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_on_accent: Rgba,
    /// Background secondary (for value badge)
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub background_secondary: Rgba,
}

/// Potentiometer size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PotentiometerSize {
    /// Compact size
    Sm,
    /// Default size
    #[default]
    Md,
    /// Large size
    Lg,
}

impl From<crate::ComponentSize> for PotentiometerSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Scale type for potentiometer value mapping
/// Re-exported from scale module for API consistency
pub type PotentiometerScale = Scale;

impl PotentiometerSize {
    fn knob_size(&self) -> f32 {
        match self {
            Self::Sm => 40.0,
            Self::Md => 60.0,
            Self::Lg => 80.0,
        }
    }

    fn indicator_radius(&self) -> f32 {
        match self {
            Self::Sm => 14.0,
            Self::Md => 20.0,
            Self::Lg => 26.0,
        }
    }

    fn min_width(&self) -> f32 {
        match self {
            Self::Sm => 80.0,
            Self::Md => 100.0,
            Self::Lg => 120.0,
        }
    }
}

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
    on_change: Option<Box<dyn Fn(f64, &mut Window, &mut App) + 'static>>,
    on_drag_start: Option<Box<dyn Fn(f32, f64, &mut Window, &mut App) + 'static>>,
    on_select: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    on_reset: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    focus_handle: Option<FocusHandle>,
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
            on_change: None,
            on_drag_start: None,
            on_select: None,
            on_reset: None,
            focus_handle: None,
        }
    }

    /// Convert a value to normalized position [0, 1] based on scale type
    fn value_to_normalized(&self, value: f64) -> f64 {
        self.scale.value_to_normalized(value, self.min, self.max)
    }

    /// Set the current value (clamped to min/max during render)
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Set the minimum value
    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    /// Set the maximum value
    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    /// Set the unit label (e.g., "dB", "Hz", "%", ":1")
    pub fn unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Set the display label
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the keyboard shortcut key for the label
    pub fn shortcut_key(mut self, key: char) -> Self {
        self.shortcut_key = Some(key);
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

    /// Set value change handler (called on scroll wheel and mouse click)
    ///
    /// When only `on_change` is provided (without `on_select` or `on_drag_start`),
    /// clicking the potentiometer will increment the value by 10% and wrap around at max.
    /// Scrolling will adjust the value by 5% increments.
    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
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

    /// Format the label with keyboard shortcut indicator
    fn format_label(&self) -> String {
        let label = self
            .label
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default();
        match self.shortcut_key {
            Some(key) => {
                let key_lower = key.to_ascii_lowercase();
                let label_lower = label.to_lowercase();
                if let Some(pos) = label_lower.find(key_lower) {
                    format!(
                        "{}[{}]{}",
                        &label[..pos],
                        label.chars().nth(pos).unwrap().to_ascii_uppercase(),
                        &label[pos + 1..]
                    )
                } else {
                    format!("[{}] {}", key.to_ascii_uppercase(), label)
                }
            }
            None => label,
        }
    }

    /// Format the value display (with unit suffix)
    /// Note: Currently unused, kept for potential future use
    #[allow(dead_code)]
    fn format_value(&self) -> String {
        let value = self.value.clamp(self.min, self.max);
        let unit = self.unit.to_string();
        if unit == ":1" {
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
        }
    }

    /// Format the value display (without unit, for center display)
    fn format_value_only(&self) -> String {
        let value = self.value.clamp(self.min, self.max);
        let unit = self.unit.to_string();
        if unit == ":1" {
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
        }
    }
}

impl RenderOnce for Potentiometer {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let default_theme = PotentiometerTheme::from(&global_theme);
        let theme = self.theme.clone().unwrap_or(default_theme);
        let selected = self.selected;
        let disabled = self.disabled;

        // Use scale-aware normalization for indicator position
        let normalized = self.value_to_normalized(self.value) as f32;

        // Calculate angle for indicator with dead zone at 6 o'clock (bottom)
        // In screen coordinates (y-down): 0° = 3 o'clock, 90° = 6 o'clock, 180° = 9 o'clock, 270° = 12 o'clock
        // Start at 135° (7:30 position) and sweep clockwise 270° to 45° (4:30 position)
        let start_rad: f32 = std::f32::consts::PI * 0.75; // 135° = 3π/4 (7:30)
        let end_rad: f32 = std::f32::consts::PI * 2.25; // 405° = 45° + 360° (4:30, going through top)
        let angle_rad = start_rad + (end_rad - start_rad) * normalized;

        let knob_size = self.size.knob_size();
        let radius = self.size.indicator_radius();
        let center = knob_size / 2.0;
        // Make indicator larger for Lg size to be more visible
        let indicator_size = match self.size {
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

        let formatted_label = self.format_label();
        let value_str_only = self.format_value_only();
        let unit_str = self.unit.to_string();
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

        let mut container = div()
            .id(self.id)
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .p_2()
            .rounded_lg()
            .bg(bg_color)
            .border_2()
            .border_color(border_color)
            .min_w(px(min_width));

        // Track focus if handle provided
        // Both track_focus (for focus observation) and focusable (for key events) are needed
        if let Some(ref focus_handle) = self.focus_handle {
            container = container.track_focus(focus_handle).focusable();
        }

        // Add shadow when selected
        if selected {
            container = container.shadow_md();
        }

        // Hover effect
        let hover_border = theme.accent;
        let hover_bg = theme.surface_hover;
        container = container.hover(|s| s.border_color(hover_border).bg(hover_bg));

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
            let on_reset_rc = self.on_reset.map(|handler| std::rc::Rc::new(handler));

            // Mouse down - focus, select, and optionally start drag
            let on_select = self.on_select;
            let on_drag_start = self.on_drag_start;
            let on_change_click = on_change_rc.clone();
            let focus_handle_click = self.focus_handle.clone();

            container = container.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                // Always focus for keyboard navigation
                if let Some(ref fh) = focus_handle_click {
                    fh.focus(window);
                }

                // Handle Selection
                if let Some(ref handler) = on_select {
                    handler(window, cx);
                }

                // Handle Drag or Click-Step
                if let Some(ref handler) = on_drag_start {
                    handler(event.position.y.into(), value, window, cx);
                } else if let Some(ref handler) = on_change_click {
                    // If no drag handler, use click to step value (scale-aware)
                    let new_value = scale.step_value(value, min, max, 1.0, 0.1);
                    handler(new_value, window, cx);
                }
            });

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
                container = container.on_key_down(move |event, window, cx| {
                    let key = event.keystroke.key.as_str();
                    if key == "escape" {
                        if let Some(ref reset_handler) = reset_key {
                            reset_handler(window, cx);
                        }
                    } else if let Some(ref handler) = handler_key {
                        if let Some(new_value) = handle_keyboard(
                            key,
                            &event.keystroke.modifiers,
                            current_value_key.get(),
                            &config_key,
                        ) {
                            current_value_key.set(new_value);
                            handler(new_value, window, cx);
                        }
                    }
                });
            }

            // Scroll wheel - adjust value
            if let Some(handler_rc) = on_change_rc {
                let current_value_scroll = current_value.clone();
                let config_scroll = interaction_config.clone();
                container = container.on_scroll_wheel(move |event, window, cx| {
                    cx.stop_propagation();
                    let val = current_value_scroll.get();
                    if let Some(new_value) =
                        handle_scroll(&event.delta, &event.modifiers, val, &config_scroll)
                    {
                        current_value_scroll.set(new_value);
                        handler_rc(new_value, window, cx);
                    }
                });
            }

            // Focus on mouse enter - keyboard follows hover like scroll wheel
            let focus_handle_hover = self.focus_handle.clone();
            container = container.on_mouse_move(move |event, window, _cx| {
                if let Some(ref fh) = focus_handle_hover {
                    if !fh.is_focused(window) && !event.pressed_button.is_some() {
                        fh.focus(window);
                    }
                }
            });
        }

        // Label with keyboard shortcut
        container = container.child(
            div()
                .text_xs()
                .font_weight(if selected {
                    FontWeight::BOLD
                } else {
                    FontWeight::SEMIBOLD
                })
                .text_color(label_color)
                .text_center()
                .child(formatted_label),
        );

        // Determine number of major ticks based on range and size
        // Algorithm:
        // 1. Try divisors for max (10, 5, 3, 2) - prefer 10 for large size
        // 2. Compute tick_interval = max / divisor
        // 3. Check if min is a multiple of tick_interval
        // 4. Number of ticks = range / tick_interval
        // Example: min=100, max=1000, large → 1000/10=100, 100%100=0 ✓ → ticks every 100 → 9 ticks
        let range = max - min;
        let is_large = matches!(self.size, PotentiometerSize::Lg);

        // Candidate divisors: large size can use 10, others prefer smaller counts
        let divisors: &[i32] = if is_large { &[10, 5, 3, 2] } else { &[5, 3, 2] };

        let num_major_ticks = {
            let mut best_tick_count = if is_large { 10 } else { 4 };

            for &div in divisors {
                // Skip if max is not cleanly divisible by this divisor
                if max.abs() < 0.0001 {
                    continue;
                }

                let tick_interval = max / div as f64;
                if tick_interval.abs() < 0.0001 {
                    continue;
                }

                // Check if min is a multiple of tick_interval
                let min_remainder = if min.abs() < 0.0001 {
                    0.0
                } else {
                    (min / tick_interval).fract().abs()
                };
                let min_aligned = min_remainder < 0.01 || (1.0 - min_remainder) < 0.01;

                if min_aligned {
                    // Compute how many ticks fit in the range
                    let tick_count = (range / tick_interval).round() as i32;
                    if tick_count >= 2 && tick_count <= (if is_large { 10 } else { 6 }) {
                        best_tick_count = tick_count;
                        break;
                    }
                }
            }

            best_tick_count
        };

        // Number of minor ticks between each major tick
        let minor_ticks_between = 4;

        // Knob graphic with ticks - need larger container for labels
        let container_size = knob_size + 30.0; // Extra space for tick labels
        let mut knob_container = div().w(px(container_size)).h(px(container_size)).relative();

        // Add tick marks and labels around the knob
        let knob_offset = 15.0; // Offset to center the knob in the larger container
        let tick_inner_radius = knob_size / 2.0; // Start at knob edge
        let major_tick_outer_radius = tick_inner_radius + 8.0; // Major ticks
        let minor_tick_outer_radius = tick_inner_radius + 5.0; // Minor ticks (shorter)
        let label_radius = major_tick_outer_radius + 8.0; // Labels outside ticks
        let major_tick_width = 3.0; // Doubled from 1.5
        let minor_tick_width = 1.5; // Thinner for minor ticks

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

        // Total number of tick positions (major + minor)
        let total_ticks = num_major_ticks * (minor_ticks_between + 1);

        for i in 0..=total_ticks {
            let tick_normalized = i as f32 / total_ticks as f32;
            let tick_angle = start_rad + (end_rad - start_rad) * tick_normalized;

            // Determine if this is a major tick (has label) or minor tick
            let is_major = i % (minor_ticks_between + 1) == 0;

            let (tick_outer_radius, tick_width, tick_color) = if is_major {
                (major_tick_outer_radius, major_tick_width, major_tick_color)
            } else {
                (minor_tick_outer_radius, minor_tick_width, minor_tick_color)
            };

            // Calculate tick line positions (inner and outer points)
            let inner_x = knob_offset + center + tick_inner_radius * tick_angle.cos();
            let inner_y = knob_offset + center + tick_inner_radius * tick_angle.sin();
            let outer_x = knob_offset + center + tick_outer_radius * tick_angle.cos();
            let outer_y = knob_offset + center + tick_outer_radius * tick_angle.sin();

            // Draw tick line using circles connected visually
            let tick_length = tick_outer_radius - tick_inner_radius;
            let num_dots = (tick_length / 1.5).max(2.0) as usize;
            for j in 0..=num_dots {
                let t = j as f32 / num_dots as f32;
                let dot_x = inner_x + (outer_x - inner_x) * t - tick_width / 2.0;
                let dot_y = inner_y + (outer_y - inner_y) * t - tick_width / 2.0;

                knob_container = knob_container.child(
                    div()
                        .absolute()
                        .left(px(dot_x))
                        .top(px(dot_y))
                        .w(px(tick_width))
                        .h(px(tick_width))
                        .rounded_full()
                        .bg(tick_color),
                );
            }

            // Add tick label only for major ticks
            if is_major {
                // Convert normalized position to actual value using scale
                let tick_value = scale.normalized_to_value(tick_normalized as f64, min, max);
                let label_x = knob_offset + center + label_radius * tick_angle.cos();
                let label_y = knob_offset + center + label_radius * tick_angle.sin();

                // Format tick label
                let unit = self.unit.as_ref();
                let label_text = if unit == "%" {
                    // tick_normalized is already 0.0 to 1.0, so multiply by 100 for percentage
                    format!("{:.0}", tick_normalized * 100.0)
                } else if unit == "Hz" {
                    if tick_value >= 1000.0 {
                        format!("{:.0}k", tick_value / 1000.0)
                    } else {
                        format!("{:.0}", tick_value)
                    }
                } else if unit == "dB" {
                    format!("{:.0}", tick_value)
                } else {
                    format!("{:.1}", tick_value)
                };

                knob_container = knob_container.child(
                    div()
                        .absolute()
                        .left(px(label_x - 6.0)) // Center the text
                        .top(px(label_y - 5.0))
                        .text_size(px(9.0)) // Smaller than text_xs (12px)
                        .text_color(major_tick_color)
                        .child(label_text),
                );
            }
        }

        // Knob circle (offset to center in larger container)
        // Use major_tick_color for border to match ticks and labels
        let mut knob = div()
            .absolute()
            .left(px(knob_offset))
            .top(px(knob_offset))
            .w(px(knob_size))
            .h(px(knob_size))
            .rounded_full()
            .bg(knob_bg)
            .border_2()
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

        // Indicator dot
        let mut indicator = div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(indicator_size))
            .h(px(indicator_size))
            .bg(indicator_color)
            .rounded_full();

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
            PotentiometerSize::Sm => value_display.text_xs(),
            PotentiometerSize::Md => value_display.text_xs(),
            PotentiometerSize::Lg => value_display.text_sm(),
        };

        knob = knob.child(value_display.child(value_str_only.clone()));

        knob_container = knob_container.child(knob);

        // Unit label at 6 o'clock position (270° standard = 90° screen, bottom center)
        // Position it at the same radius as the tick labels
        if !unit_str.is_empty() {
            let unit_angle = std::f32::consts::PI * 0.5; // 90° in screen coordinates (6 o'clock)
            let unit_x = knob_offset + center + label_radius * unit_angle.cos();
            let unit_y = knob_offset + center + label_radius * unit_angle.sin();

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
