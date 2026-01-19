//! Toggle/Switch component
//!
//! A toggle switch for boolean values with optional selection highlighting
//! for plugin parameter editing.
//!
//! Features:
//! - Click to toggle state
//! - Space key to toggle when selected
//! - Optional label
//! - Two visual styles: Sliding (iOS-style) and Segmented ([OFF|ON])

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::*;
use gpui::*;

/// Toggle size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToggleSize {
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
}

impl ToggleSize {
    fn track_width(&self) -> Pixels {
        match self {
            ToggleSize::Sm => px(32.0),
            ToggleSize::Md => px(40.0),
            ToggleSize::Lg => px(52.0),
        }
    }

    fn track_height(&self) -> Pixels {
        match self {
            ToggleSize::Sm => px(18.0),
            ToggleSize::Md => px(22.0),
            ToggleSize::Lg => px(28.0),
        }
    }

    fn knob_size(&self) -> Pixels {
        match self {
            ToggleSize::Sm => px(14.0),
            ToggleSize::Md => px(18.0),
            ToggleSize::Lg => px(24.0),
        }
    }

    fn knob_offset(&self) -> Pixels {
        match self {
            ToggleSize::Sm => px(2.0),
            ToggleSize::Md => px(2.0),
            ToggleSize::Lg => px(2.0),
        }
    }
}

impl From<crate::ComponentSize> for ToggleSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// Toggle visual style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToggleStyle {
    /// iOS-style sliding toggle (default)
    #[default]
    Sliding,
    /// Segmented [OFF | ON] style for audio plugins
    Segmented,
}

/// Theme colors for toggle styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct ToggleTheme {
    /// Background when checked
    #[theme(default = 0x007accff, from = accent)]
    pub checked_bg: Rgba,
    /// Background when unchecked
    #[theme(default = 0x3a3a3aff, from = muted)]
    pub unchecked_bg: Rgba,
    /// Knob/thumb color when unchecked
    #[theme(default = 0xffffffff, from = text_primary)]
    pub knob: Rgba,
    /// Knob/thumb color when checked (for contrast on light backgrounds)
    #[theme(default = 0xffffffff, from = background)]
    pub knob_on_checked: Rgba,
    /// Track border color (for visibility on dark backgrounds)
    #[theme(default = 0x3a3a3aff, from = border)]
    pub track_border: Rgba,
    /// Label text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label: Rgba,
    /// Selected state accent color
    #[theme(default = 0x007accff, from = accent)]
    pub accent: Rgba,
    /// Selected state background
    #[theme(default = 0x007acc33, from = accent_muted)]
    pub accent_muted: Rgba,
    /// Success color for ON state (segmented style)
    #[theme(default = 0x22c55eff, from = success)]
    pub success: Rgba,
    /// Border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Text on accent background
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_on_accent: Rgba,
    /// Muted text color
    #[theme(default = 0x888888ff, from = text_muted)]
    pub text_muted: Rgba,
    /// Primary text color (for selected labels)
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_primary: Rgba,
    /// Surface hover color (for active segmented button)
    #[theme(default = 0x4a4a4aff, from = surface_hover)]
    pub surface_hover: Rgba,
    /// Background color (for inactive segmented button)
    #[theme(default = 0x2a2a2aff, from = background)]
    pub background: Rgba,
}

/// A toggle switch component with optional selection highlighting
pub struct Toggle {
    id: ElementId,
    checked: bool,
    label: Option<SharedString>,
    size: ToggleSize,
    style: ToggleStyle,
    disabled: bool,
    selected: bool,
    theme: Option<ToggleTheme>,
    on_change: Option<Box<dyn Fn(bool, &mut Window, &mut App) + 'static>>,
}

impl Toggle {
    /// Create a new toggle
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            label: None,
            size: ToggleSize::default(),
            style: ToggleStyle::default(),
            disabled: false,
            selected: false,
            theme: None,
            on_change: None,
        }
    }

    /// Set checked state
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set label
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set size
    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = size;
        self
    }

    /// Set visual style
    pub fn style(mut self, style: ToggleStyle) -> Self {
        self.style = style;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set selected state (for plugin parameter editing)
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set theme colors
    pub fn theme(mut self, theme: ToggleTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set change handler
    pub fn on_change(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, global_theme: &ToggleTheme) -> Stateful<Div> {
        let theme = self.theme.clone().unwrap_or_else(|| global_theme.clone());
        let style = self.style;

        match style {
            ToggleStyle::Sliding => self.build_sliding(&theme),
            ToggleStyle::Segmented => self.build_segmented(&theme),
        }
    }

    fn build_sliding(self, theme: &ToggleTheme) -> Stateful<Div> {
        let track_width = self.size.track_width();
        let track_height = self.size.track_height();
        let knob_size = self.size.knob_size();
        let knob_offset = self.size.knob_offset();
        let checked = self.checked;
        let selected = self.selected;

        let track_bg = if checked {
            theme.checked_bg
        } else {
            theme.unchecked_bg
        };

        let knob_left = if checked {
            track_width - knob_size - knob_offset
        } else {
            knob_offset
        };

        let mut container = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer();

        // Apply selection styling
        if selected {
            container = container
                .px_3()
                .py_2()
                .rounded_lg()
                .bg(theme.accent_muted)
                .border_l_4()
                .border_color(theme.accent);
        }

        if self.disabled {
            container = container.opacity(0.5).cursor_not_allowed();
        }

        // Track - with border for visibility on dark backgrounds
        let mut track = div()
            .relative()
            .w(track_width)
            .h(track_height)
            .rounded_full()
            .bg(track_bg)
            .border_1()
            .border_color(theme.track_border);

        // Knob - always same color, just moves position
        let knob = div()
            .absolute()
            .top(knob_offset)
            .left(knob_left)
            .w(knob_size)
            .h(knob_size)
            .rounded_full()
            .bg(theme.knob)
            .shadow_md()
            .border_1()
            .border_color(theme.track_border);

        track = track.child(knob);

        // Label first if selected (for row layout)
        if let Some(label) = &self.label
            && selected
        {
            let label_el = match self.size {
                ToggleSize::Sm => div().text_xs(),
                ToggleSize::Md => div().text_sm(),
                ToggleSize::Lg => div(),
            };
            container = container.child(
                label_el
                    .text_color(theme.label)
                    .font_weight(FontWeight::MEDIUM)
                    .child(label.clone()),
            );
        }

        container = container.child(track);

        // Label after track if not selected
        if let Some(label) = &self.label
            && !selected
        {
            let label_el = match self.size {
                ToggleSize::Sm => div().text_xs(),
                ToggleSize::Md => div().text_sm(),
                ToggleSize::Lg => div(),
            };
            container = container.child(label_el.text_color(theme.label).child(label.clone()));
        }

        // Click and keyboard handlers
        if !self.disabled
            && let Some(handler) = self.on_change
        {
            let handler_rc = std::rc::Rc::new(handler);
            let new_checked = !checked;

            // Click handler
            let handler_click = handler_rc.clone();
            container = container.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                handler_click(new_checked, window, cx);
            });

            // Keyboard handler (Space key when selected)
            if selected {
                let handler_key = handler_rc.clone();
                container = container.on_key_down(move |event, window, cx| {
                    if event.keystroke.key == "space" {
                        handler_key(new_checked, window, cx);
                    }
                });
            }
        }

        container
    }

    fn build_segmented(self, theme: &ToggleTheme) -> Stateful<Div> {
        let checked = self.checked;
        let selected = self.selected;

        let mut container = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer();

        // Apply selection styling
        if selected {
            container = container
                .px_3()
                .py_2()
                .rounded_lg()
                .bg(theme.accent_muted)
                .border_l_4()
                .border_color(theme.accent);
        } else {
            container = container.px_3().py_2().rounded_lg();
        }

        if self.disabled {
            container = container.opacity(0.5).cursor_not_allowed();
        }

        // Label
        if let Some(label) = &self.label {
            let label_color = if selected {
                theme.text_primary
            } else {
                theme.label
            };
            let label_weight = if selected {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            };
            container = container.child(
                div()
                    .text_sm()
                    .text_color(label_color)
                    .font_weight(label_weight)
                    .child(label.clone()),
            );
        }

        // Segmented switch: [OFF | ON]
        let switch = div()
            .flex()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .overflow_hidden()
            // OFF button
            .child(
                div()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .bg(if !checked {
                        theme.surface_hover
                    } else {
                        theme.background
                    })
                    .text_color(if !checked {
                        theme.text_primary
                    } else {
                        theme.text_muted
                    })
                    .child("OFF"),
            )
            // Separator
            .child(div().w(px(1.0)).h_full().bg(theme.border))
            // ON button
            .child(
                div()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .bg(if checked {
                        theme.success
                    } else {
                        theme.background
                    })
                    .text_color(if checked {
                        theme.text_on_accent
                    } else {
                        theme.text_muted
                    })
                    .child("ON"),
            );

        container = container.child(switch);

        // Click and keyboard handlers
        if !self.disabled
            && let Some(handler) = self.on_change
        {
            let handler_rc = std::rc::Rc::new(handler);
            let new_checked = !checked;

            // Click handler
            let handler_click = handler_rc.clone();
            container = container.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                handler_click(new_checked, window, cx);
            });

            // Keyboard handler (Space key when selected)
            if selected {
                let handler_key = handler_rc.clone();
                container = container.on_key_down(move |event, window, cx| {
                    if event.keystroke.key == "space" {
                        handler_key(new_checked, window, cx);
                    }
                });
            }
        }

        container
    }
}

impl RenderOnce for Toggle {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let toggle_theme = ToggleTheme::from(&global_theme);
        self.build_with_theme(&toggle_theme)
    }
}

impl IntoElement for Toggle {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
