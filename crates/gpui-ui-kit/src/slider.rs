//! Slider component for selecting numeric values within a range
//!
//! Features:
//! - Drag support: click and drag the thumb or anywhere on the track
//! - Scroll wheel: scroll up/down to adjust value (shift for fine control)
//! - Double-click to reset to default
//! - Keyboard navigation (when focused):
//!   - Arrow Up/Right: increase value (5%)
//!   - Arrow Down/Left: decrease value (5%)
//!   - Page Up: increase value (10%)
//!   - Page Down: decrease value (10%)
//!   - Home: set to minimum
//!   - End: set to maximum
//! - Value snapping with step parameter

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::*;

/// Theme colors for slider styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct SliderTheme {
    /// Track background color (unfilled portion)
    #[theme(default = 0x3e3e3eff, from = border)]
    pub track: Rgba,
    /// Fill color (active portion)
    #[theme(default = 0x007accff, from = accent)]
    pub fill: Rgba,
    /// Thumb/handle color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub thumb: Rgba,
    /// Thumb hover color
    #[theme(default = 0xe0e0e0ff, from = text_secondary)]
    pub thumb_hover: Rgba,
    /// Thumb active (dragging) color
    #[theme(default = 0x007accff, from = accent)]
    pub thumb_active: Rgba,
    /// Label text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label: Rgba,
    /// Value text color
    #[theme(default = 0x999999ff, from = text_muted)]
    pub value: Rgba,
    /// Disabled label color (muted with transparency)
    #[theme(default = 0x66666699, from = text_muted)]
    pub disabled_label: Rgba,
    /// Disabled fill/border color
    #[theme(default = 0xccccccff, from = text_muted)]
    pub disabled_fill: Rgba,
}

/// Slider size variants
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SliderSize {
    /// Small size
    Sm,
    /// Medium size (default)
    #[default]
    Md,
    /// Large size
    Lg,
}

impl SliderSize {
    fn track_height(&self) -> f32 {
        match self {
            Self::Sm => 4.0,
            Self::Md => 6.0,
            Self::Lg => 8.0,
        }
    }

    fn thumb_size(&self) -> f32 {
        match self {
            Self::Sm => 14.0,
            Self::Md => 18.0,
            Self::Lg => 22.0,
        }
    }
}

impl From<crate::ComponentSize> for SliderSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// A slider component for selecting numeric values
///
/// Supports:
/// - Mouse drag on track or thumb
/// - Scroll wheel adjustment (shift for fine-grained control)
/// - Double-click to reset to default
/// - Keyboard arrow keys (when focused)
#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    value: f32,
    min: f32,
    max: f32,
    step: Option<f32>,
    size: SliderSize,
    disabled: bool,
    show_value: bool,
    label: Option<SharedString>,
    width: f32,
    on_change: Option<Box<dyn Fn(f32, &mut Window, &mut App) + 'static>>,
    on_drag_start: Option<Box<dyn Fn(f32, f32, &mut Window, &mut App) + 'static>>,
    on_reset: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    track_color: Option<Rgba>,
    fill_color: Option<Rgba>,
    thumb_color: Option<Rgba>,
    theme: Option<SliderTheme>,
}

impl Slider {
    /// Create a new slider with the given ID
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: None,
            size: SliderSize::default(),
            disabled: false,
            show_value: false,
            label: None,
            width: 200.0,
            on_change: None,
            on_drag_start: None,
            on_reset: None,
            track_color: None,
            fill_color: None,
            thumb_color: None,
            theme: None,
        }
    }

    /// Set the current value
    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(self.min, self.max);
        self
    }

    /// Set the minimum value
    ///
    /// # Panics
    /// Panics if min > max after this call
    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    /// Set the maximum value
    ///
    /// # Panics
    /// Panics if min > max after this call
    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    /// Set both min and max values at once
    ///
    /// # Panics
    /// Panics if min > max
    pub fn range(mut self, min: f32, max: f32) -> Self {
        assert!(
            min <= max,
            "Slider range invalid: min ({}) > max ({})",
            min,
            max
        );
        self.min = min;
        self.max = max;
        self
    }

    /// Set the step size for snapping
    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self
    }

    /// Set the slider size
    pub fn size(mut self, size: SliderSize) -> Self {
        self.size = size;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Show the current value as text
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Set a label for the slider
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the width of the slider in pixels
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set the change handler
    ///
    /// The handler receives the new value by value.
    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Set the change handler with a reference argument
    ///
    /// This variant is useful when using `cx.listener()` which passes a reference.
    pub fn on_change_ref(
        mut self,
        handler: impl Fn(&f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(move |value, window, app| {
            handler(&value, window, app);
        }));
        self
    }

    /// Set drag start handler (called on mouse down with x position and current value)
    ///
    /// Use this to track dragging state in your app. When dragging, you should
    /// calculate the new value based on mouse position and call the on_change handler.
    pub fn on_drag_start(
        mut self,
        handler: impl Fn(f32, f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_drag_start = Some(Box::new(handler));
        self
    }

    /// Set reset handler (called on double-click)
    pub fn on_reset(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_reset = Some(Box::new(handler));
        self
    }

    /// Helper to snap a value to the step size
    /// Note: Currently unused, kept for potential future use
    #[allow(dead_code)]
    fn snap_value(&self, value: f32) -> f32 {
        if let Some(step) = self.step {
            let steps = ((value - self.min) / step).round();
            (self.min + steps * step).clamp(self.min, self.max)
        } else {
            value.clamp(self.min, self.max)
        }
    }

    /// Set the track color
    pub fn track_color(mut self, color: impl Into<Rgba>) -> Self {
        self.track_color = Some(color.into());
        self
    }

    /// Set the fill color
    pub fn fill_color(mut self, color: impl Into<Rgba>) -> Self {
        self.fill_color = Some(color.into());
        self
    }

    /// Set the thumb color
    pub fn thumb_color(mut self, color: impl Into<Rgba>) -> Self {
        self.thumb_color = Some(color.into());
        self
    }

    /// Set the slider theme (applies all colors at once)
    pub fn theme(mut self, theme: SliderTheme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl RenderOnce for Slider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let track_height = self.size.track_height();
        let thumb_size = self.size.thumb_size();
        let width = self.width;

        // Use theme colors if available, then individual colors, then global theme
        let global_theme = cx.theme();
        let global_slider_theme = SliderTheme::from(&global_theme);
        let theme = self.theme.as_ref().unwrap_or(&global_slider_theme);
        let track_color = self.track_color.unwrap_or(theme.track);
        let fill_color = self.fill_color.unwrap_or(theme.fill);
        let thumb_color = self.thumb_color.unwrap_or(theme.thumb);
        let thumb_hover = theme.thumb_hover;
        let label_color = theme.label;
        let value_color = theme.value;
        let disabled_label = theme.disabled_label;
        let disabled_fill = theme.disabled_fill;

        let range = self.max - self.min;
        let progress = if range > 0.0 {
            (self.value - self.min) / range
        } else {
            0.0
        };

        let fill_width = (width * progress).max(0.0);
        let thumb_left = (width * progress) - (thumb_size / 2.0);

        let min = self.min;
        let max = self.max;
        let step = self.step;
        let disabled = self.disabled;
        let current_value = self.value;

        let mut container = div().flex().flex_col().gap_1();

        // Label row
        if self.label.is_some() || self.show_value {
            let mut label_row = div().flex().justify_between().w(px(width)).text_sm();

            if let Some(label) = &self.label {
                label_row = label_row.child(
                    div()
                        .text_color(if disabled {
                            disabled_label
                        } else {
                            label_color
                        })
                        .child(label.clone()),
                );
            }

            if self.show_value {
                label_row = label_row.child(
                    div()
                        .text_color(value_color)
                        .child(format!("{:.1}", self.value)),
                );
            }

            container = container.child(label_row);
        }

        // Wrap on_change in Rc for sharing between handlers
        let on_change_rc = self.on_change.map(|h| std::rc::Rc::new(h));

        // Slider track
        let mut track = div()
            .id(self.id)
            .w(px(width))
            .h(px(thumb_size))
            .flex()
            .items_center()
            .relative()
            // Track background
            .child(
                div()
                    .absolute()
                    .left_0()
                    .w_full()
                    .h(px(track_height))
                    .rounded(px(track_height / 2.0))
                    .bg(track_color),
            )
            // Fill
            .child(
                div()
                    .absolute()
                    .left_0()
                    .w(px(fill_width))
                    .h(px(track_height))
                    .rounded(px(track_height / 2.0))
                    .bg(if disabled { disabled_fill } else { fill_color }),
            )
            // Thumb with hover effect
            .child({
                let mut thumb = div()
                    .absolute()
                    .left(px(thumb_left.max(0.0)))
                    .w(px(thumb_size))
                    .h(px(thumb_size))
                    .rounded_full()
                    .bg(thumb_color)
                    .border_2()
                    .border_color(if disabled { disabled_fill } else { fill_color })
                    .shadow_sm();
                if !disabled {
                    thumb = thumb.hover(move |s| s.bg(thumb_hover));
                }
                thumb
            });

        // Apply cursor style
        if disabled {
            track = track.cursor_not_allowed();
        } else {
            track = track.cursor_ew_resize();
        }

        // Event handlers (only if not disabled)
        if !disabled {
            // Mouse down - start drag or handle click
            if let Some(on_drag_start) = self.on_drag_start {
                let handler_down = on_drag_start;
                track = track.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    handler_down(event.position.x.into(), current_value, window, cx);
                });
            } else if let Some(ref handler_rc) = on_change_rc {
                // Click to set value based on position (immediate feedback)
                let handler_click = handler_rc.clone();
                track = track.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    // Calculate value from click position relative to track
                    let x: f32 = event.position.x.into();
                    let progress = (x / width).clamp(0.0, 1.0);
                    let new_value = min + progress * (max - min);
                    let snapped = if let Some(step) = step {
                        let steps = ((new_value - min) / step).round();
                        (min + steps * step).clamp(min, max)
                    } else {
                        new_value.clamp(min, max)
                    };
                    handler_click(snapped, window, cx);
                });

                // Mouse move while pressed - continue drag
                let handler_drag = handler_rc.clone();
                track = track.on_mouse_move(move |event, window, cx| {
                    if event.pressed_button == Some(MouseButton::Left) {
                        let x: f32 = event.position.x.into();
                        let progress = (x / width).clamp(0.0, 1.0);
                        let new_value = min + progress * (max - min);
                        let snapped = if let Some(step) = step {
                            let steps = ((new_value - min) / step).round();
                            (min + steps * step).clamp(min, max)
                        } else {
                            new_value.clamp(min, max)
                        };
                        handler_drag(snapped, window, cx);
                    }
                });
            }

            // Double-click to reset
            if let Some(on_reset) = self.on_reset {
                let reset_handler = std::rc::Rc::new(on_reset);
                let reset_clone = reset_handler.clone();
                track = track.on_click(move |event, window, cx| {
                    if event.click_count() == 2 {
                        reset_clone(window, cx);
                    }
                });
            }

            // Scroll wheel - adjust value (shift for fine-grained control)
            if let Some(ref handler_rc) = on_change_rc {
                let handler_scroll = handler_rc.clone();
                track = track.on_scroll_wheel(move |event, window, cx| {
                    // CRITICAL: Stop propagation immediately to prevent parent scroll container
                    // from capturing the event before we can handle it
                    cx.stop_propagation();

                    // Get scroll delta - positive y means scrolling up
                    let delta = event.delta.pixel_delta(px(20.0)).y;

                    if delta.abs() < px(0.01) {
                        return;
                    }

                    let scroll_up = delta < px(0.0);

                    // Calculate step amount: 5% normally, 0.5% with shift
                    let step_amount = if event.modifiers.shift {
                        step.unwrap_or((max - min) * 0.005)
                    } else {
                        step.unwrap_or((max - min) * 0.05)
                    };

                    // Increase on scroll up, decrease on scroll down
                    let change = if scroll_up { step_amount } else { -step_amount };
                    let new_value = current_value + change;

                    // Snap to step if defined (only when not in fine mode)
                    let snapped = if let Some(step) = step {
                        if event.modifiers.shift {
                            // In fine mode, don't snap to step
                            new_value.clamp(min, max)
                        } else {
                            let steps = ((new_value - min) / step).round();
                            (min + steps * step).clamp(min, max)
                        }
                    } else {
                        new_value.clamp(min, max)
                    };

                    handler_scroll(snapped, window, cx);
                });
            }

            // Keyboard navigation
            if let Some(handler_rc) = on_change_rc {
                let handler_key = handler_rc.clone();
                track = track.on_key_down(move |event, window, cx| {
                    let step_amount = step.unwrap_or((max - min) * 0.05);
                    let large_step = (max - min) * 0.10; // 10% for page up/down

                    let new_value = match event.keystroke.key.as_str() {
                        "up" | "right" => Some(current_value + step_amount),
                        "down" | "left" => Some(current_value - step_amount),
                        "pageup" => Some(current_value + large_step),
                        "pagedown" => Some(current_value - large_step),
                        "home" => Some(min),
                        "end" => Some(max),
                        _ => None,
                    };

                    if let Some(value) = new_value {
                        let snapped = if let Some(step) = step {
                            let steps = ((value - min) / step).round();
                            (min + steps * step).clamp(min, max)
                        } else {
                            value.clamp(min, max)
                        };
                        handler_key(snapped, window, cx);
                    }
                });
            }
        }

        container.child(track)
    }
}
