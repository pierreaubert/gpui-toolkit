//! IconButton component
//!
//! A button that displays only an icon, with optional tooltip.
//! Supports both text/emoji icons and custom child elements (like SVG icons).

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState};
use crate::theme::{ThemeExt, glow_shadow};
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, FocusHandle, KeyDownEvent, KeyboardClickEvent,
    Pixels, Rems, Rgba, SharedString, Stateful, Window, div, px, rems,
};
use gpui_design::DesignSystem;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

// Thread-local FocusHandle registry for IconButton, mirroring the pattern
// in `button.rs`. See its module-level comment for rationale; in short:
// `RenderOnce` allocates a fresh handle every render, so we cache by id
// here to keep IconButton Tab-reachable across re-renders.
const MAX_ICON_BUTTON_FOCUS_HANDLES: usize = 1024;

thread_local! {
    static ICON_BUTTON_FOCUS_HANDLES: RefCell<HashMap<ElementId, FocusHandle>> =
        RefCell::new(HashMap::new());
}

fn icon_button_focus_handle(id: &ElementId, cx: &mut App) -> FocusHandle {
    ICON_BUTTON_FOCUS_HANDLES.with(|handles| {
        let mut handles = handles.borrow_mut();
        while handles.len() > MAX_ICON_BUTTON_FOCUS_HANDLES {
            if let Some(key) = handles.keys().next().cloned() {
                handles.remove(&key);
            }
        }
        handles
            .entry(id.clone())
            .or_insert_with(|| cx.focus_handle())
            .clone()
    })
}

/// Theme colors for icon button styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct IconButtonTheme {
    /// Background color for ghost variant
    #[theme(default = 0x00000000, from = transparent)]
    pub ghost_bg: Rgba,
    /// Background color on hover for ghost variant
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub ghost_hover_bg: Rgba,
    /// Background color when selected
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub selected_bg: Rgba,
    /// Background color on hover when selected
    #[theme(default = 0x4a4a4aff, from = muted)]
    pub selected_hover_bg: Rgba,
    /// Filled variant background
    #[theme(default = 0x3a3a3aff, from = surface)]
    pub filled_bg: Rgba,
    /// Filled variant hover background
    #[theme(default = 0x4a4a4aff, from = surface_hover)]
    pub filled_hover_bg: Rgba,
    /// Accent color (for filled selected, outline border)
    #[theme(default = 0x007accff, from = accent)]
    pub accent: Rgba,
    /// Accent hover color
    #[theme(default = 0x0098ffff, from = accent)]
    pub accent_hover: Rgba,
    /// Default text/icon color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Text color when selected or on accent background
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub text_on_accent: Rgba,
    /// Border color for outline variant
    #[theme(default = 0x555555ff, from = border)]
    pub border: Rgba,
}

/// IconButton size variants. Sizes are returned as `Rems` so the click
/// target scales with `window.set_rem_size` (font zoom). The `Sm`, `Md`,
/// `Lg`, and `Xl` variants all meet the WCAG 2.5.8 24×24 minimum target
/// size at 1× zoom; `Xs` (1.0 rem ≈ 16 px) is intentionally below the
/// floor and reserved for dense chrome / chart-internal use where the
/// WCAG target rule is informational at small viewports. Prefer `Sm`
/// (or `Md`) anywhere a user can reasonably hit the button with a mouse
/// or touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconButtonSize {
    /// Extra small (1.0 rem ≈ 16 px) — dense chrome / chart internals.
    /// Below the WCAG 24×24 floor; do not use for primary controls.
    Xs,
    /// Small (1.5 rem ≈ 24 px) — meets the WCAG floor.
    Sm,
    /// Medium (1.5 rem ≈ 24 px, default) — meets the WCAG floor.
    #[default]
    Md,
    /// Large (2.0 rem ≈ 32 px).
    Lg,
    /// Extra large (3.0 rem ≈ 48 px).
    Xl,
    /// Custom size, expressed in rems × 16 (i.e. logical px at 1× zoom).
    Custom(u32),
}

impl IconButtonSize {
    /// Get the click-target size in rems. The default GPUI rem is 16 px,
    /// so the table maps to 16 / 24 / 24 / 32 / 48 logical px at 1× zoom
    /// and scales linearly with font zoom.
    pub fn size(&self) -> Rems {
        self.size_with_design(&DesignSystem::neutral())
    }

    fn size_with_design(&self, design: &DesignSystem) -> Rems {
        match self {
            IconButtonSize::Xs => rems(design.interaction.min_touch_target * 0.5 / 16.0),
            IconButtonSize::Sm => rems(design.interaction.min_touch_target * 0.75 / 16.0),
            IconButtonSize::Md => rems(design.interaction.min_touch_target * 0.75 / 16.0),
            IconButtonSize::Lg => rems(design.interaction.min_touch_target / 16.0),
            IconButtonSize::Xl => rems(design.interaction.min_touch_target * 1.5 / 16.0),
            IconButtonSize::Custom(size) => rems(*size as f32 / 16.0),
        }
    }
}

impl From<crate::ComponentSize> for IconButtonSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg => Self::Lg,
            crate::ComponentSize::Xl => Self::Xl,
        }
    }
}

/// IconButton variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconButtonVariant {
    /// Ghost button (transparent, default)
    #[default]
    Ghost,
    /// Filled background
    Filled,
    /// Outline border
    Outline,
}

/// Icon content - either text/emoji or a custom element
enum IconContent {
    Text(SharedString),
    Element(AnyElement),
}

/// An icon-only button component
///
/// # Examples
///
/// ```ignore
/// // With text/emoji icon
/// IconButton::new("btn", "🔊")
///     .variant(IconButtonVariant::Ghost)
///     .on_click(|window, cx| { /* handle click */ })
///
/// // With custom element (e.g., SVG icon)
/// IconButton::with_child("btn", my_svg_icon)
///     .size(IconButtonSize::Lg)
///     .rounded_full()
///     .theme(my_theme)
/// ```
pub struct IconButton {
    id: ElementId,
    content: IconContent,
    size: IconButtonSize,
    variant: IconButtonVariant,
    disabled: bool,
    selected: bool,
    rounded_full: bool,
    padding: Option<Pixels>,
    theme: Option<IconButtonTheme>,
    design: Option<Arc<DesignSystem>>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl IconButton {
    /// Create a new icon button with a text/emoji icon
    pub fn new(id: impl Into<ElementId>, icon: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            content: IconContent::Text(icon.into()),
            size: IconButtonSize::default(),
            variant: IconButtonVariant::default(),
            disabled: false,
            selected: false,
            rounded_full: false,
            padding: None,
            theme: None,
            design: None,
            on_click: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Create a new icon button with a custom child element (e.g., SVG icon)
    pub fn with_child(id: impl Into<ElementId>, child: impl IntoElement) -> Self {
        Self {
            id: id.into(),
            content: IconContent::Element(child.into_any_element()),
            size: IconButtonSize::default(),
            variant: IconButtonVariant::default(),
            disabled: false,
            selected: false,
            rounded_full: false,
            padding: None,
            theme: None,
            design: None,
            on_click: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the button size
    pub fn size(mut self, size: IconButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Set the button variant
    pub fn variant(mut self, variant: IconButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set selected state
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set the click handler that ignores the event payload.
    ///
    /// Use this when the handler doesn't need the `ClickEvent` itself.
    /// For handlers that integrate with `cx.listener(...)`, see
    /// [`Self::on_click_event`].
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        let handler = Rc::new(handler);
        self.on_click = Some(Rc::new(move |_event: &ClickEvent, window, cx| {
            handler(window, cx);
        }));
        self
    }

    /// Set the click handler with access to the `ClickEvent` payload.
    /// Matches the signature `cx.listener(...)` produces, so call sites
    /// that previously did `.build().on_click(cx.listener(...))` can drop
    /// the `.build()` and call this directly.
    pub fn on_click_event(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Use fully rounded corners (circular button)
    pub fn rounded_full(mut self) -> Self {
        self.rounded_full = true;
        self
    }

    /// Set custom padding (overrides default size-based padding)
    pub fn padding(mut self, padding: Pixels) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Set the button theme
    pub fn theme(mut self, theme: IconButtonTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Override the design system used for sizing and radii defaults.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit ARIA label (overrides the icon text)
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Button)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Get the computed colors based on variant and state
    pub fn compute_colors(
        &self,
        fallback_theme: &IconButtonTheme,
    ) -> (Rgba, Rgba, Rgba, Option<Rgba>) {
        let theme = self.theme.as_ref().unwrap_or(fallback_theme);

        match self.variant {
            IconButtonVariant::Ghost => {
                if self.selected {
                    (
                        theme.selected_bg,
                        theme.selected_hover_bg,
                        theme.text_on_accent,
                        None,
                    )
                } else {
                    (theme.ghost_bg, theme.ghost_hover_bg, theme.text, None)
                }
            }
            IconButtonVariant::Filled => {
                if self.selected {
                    (theme.accent, theme.accent_hover, theme.text_on_accent, None)
                } else {
                    (theme.filled_bg, theme.filled_hover_bg, theme.text, None)
                }
            }
            IconButtonVariant::Outline => {
                if self.selected {
                    (
                        theme.selected_bg,
                        theme.selected_hover_bg,
                        theme.text_on_accent,
                        Some(theme.accent),
                    )
                } else {
                    (
                        theme.ghost_bg,
                        theme.ghost_hover_bg,
                        theme.text,
                        Some(theme.border),
                    )
                }
            }
        }
    }

    /// Build into element with theme
    pub fn build_with_theme(
        self,
        global_theme: &crate::theme::Theme,
        icon_theme: &IconButtonTheme,
    ) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(global_theme, icon_theme, &design)
    }

    /// Build into element with theme and design-system sizing tokens.
    pub fn build_with_theme_and_design(
        self,
        global_theme: &crate::theme::Theme,
        icon_theme: &IconButtonTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let size = self.size.size_with_design(design);
        let (bg, bg_hover, text_color, border) = self.compute_colors(icon_theme);

        let mut el = div()
            .id(self.id)
            .font_family(global_theme.font_family.clone())
            .flex()
            .items_center()
            .justify_center()
            .w(size)
            .h(size)
            .bg(bg)
            .text_color(text_color)
            .cursor_pointer();

        // Apply padding if specified
        if let Some(padding) = self.padding {
            el = el.p(padding);
        }

        // Apply rounding
        if self.rounded_full {
            el = el.rounded_full();
        } else {
            el = el.rounded(px(design.corners.md));
        }

        if let Some(border_color) = border {
            el = el.border_1().border_color(border_color);
        }

        if self.disabled {
            el = el.opacity(0.5).cursor_not_allowed();
        } else {
            el = el.hover(move |style| style.bg(bg_hover).shadow(glow_shadow(bg_hover)));

            if let Some(handler) = self.on_click {
                el = el.on_click(move |event: &ClickEvent, window, cx| {
                    handler(event, window, cx);
                });
            }
        }

        // Add content
        match self.content {
            IconContent::Text(text) => el.child(text),
            IconContent::Element(element) => el.child(element),
        }
    }
}

impl RenderOnce for IconButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Register in accessibility tree
        let fallback_label = match &self.content {
            IconContent::Text(text) => text.clone(),
            IconContent::Element(_) => SharedString::default(),
        };
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.aria_label.clone().unwrap_or(fallback_label),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Button))
                .maybe_state(self.disabled, AriaState::Disabled),
        });

        // Capture pieces needed for keyboard activation before `self` is
        // moved into `build_with_theme`. Resolve design and focus handle
        // first because they need `&mut App`; theme is borrowed afterwards.
        // Same convention as button.rs: direct `build_with_theme` callers
        // bypass focus registration just like they bypass accessibility
        // registration today (per gpui-ui-kit/CLAUDE.md).
        let focus_handle = icon_button_focus_handle(&self.id, cx);
        let disabled = self.disabled;
        let on_click_for_kbd = self.on_click.clone();
        let design = crate::design::resolve_design(self.design.clone(), cx);

        let global_theme = cx.theme();
        let icon_theme = IconButtonTheme::from(global_theme.as_ref());
        let focus_ring_color = icon_theme.accent;
        let mut el = self
            .build_with_theme_and_design(global_theme.as_ref(), &icon_theme, &design)
            .track_focus(&focus_handle)
            // CSS `:focus-visible` analogue — only renders when reached via
            // keyboard. Layered 2px accent border on top of the (optional)
            // 1px outline-variant border.
            .focus_visible(move |style| style.border_2().border_color(focus_ring_color));

        // Keyboard activation — Enter and Space mirror the click handler
        // with a synthesized `ClickEvent::Keyboard` payload. Required for
        // WCAG 2.1.1 (Keyboard accessible).
        if !disabled && let Some(handler) = on_click_for_kbd {
            el = el.on_key_down(move |event: &KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                if key == "enter" || key == "space" {
                    let click = ClickEvent::Keyboard(KeyboardClickEvent::default());
                    handler(&click, window, cx);
                    cx.stop_propagation();
                }
            });
        }

        el
    }
}

impl IntoElement for IconButton {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
