//! `TopNav` component
//!
//! A generic top navigation bar with a brand slot, navigation items,
//! and a trailing actions slot. The bar stays product-neutral: labels,
//! brand content, and actions all come from the caller.
//!
//! # Usage
//!
//! ```ignore
//! TopNav::new("main-nav")
//!     .brand(div().child("Product"))
//!     .item(TopNavItem::new("home", "Home").active(true))
//!     .item(TopNavItem::new("docs", "Docs"))
//!     .trailing(search_field)
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, KeyDownEvent, MouseButton, Pixels, Rgba,
    SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Click handler for one navigation item.
type TopNavClick = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// Theme colors for top navigation.
#[derive(Debug, Clone, ComponentTheme)]
pub struct TopNavTheme {
    /// Bar background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Bottom divider.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Item label color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub item_text: Rgba,
    /// Item hover background.
    #[theme(default = 0x2a2a2aff, from = surface_hover)]
    pub item_hover: Rgba,
    /// Active item background.
    #[theme(default = 0x007accff, from = accent)]
    pub item_active: Rgba,
    /// Active item label color.
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub item_active_text: Rgba,
}

/// Top navigation size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TopNavSize {
    /// Compact bar.
    Sm,
    /// Standard bar (default).
    #[default]
    Md,
    /// Tall bar.
    Lg,
}

impl TopNavSize {
    fn bar_height(self, design: &DesignSystem) -> Pixels {
        match self {
            Self::Sm => px(design.interaction.min_touch_target * 0.9),
            Self::Md => px(design.interaction.min_touch_target * 1.1),
            Self::Lg => px(design.interaction.min_touch_target * 1.4),
        }
    }
}

impl From<crate::ComponentSize> for TopNavSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// One link in a top navigation bar.
pub struct TopNavItem {
    id: SharedString,
    label: SharedString,
    active: bool,
    disabled: bool,
    on_click: Option<TopNavClick>,
}

impl TopNavItem {
    /// Create a navigation item.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            active: false,
            disabled: false,
            on_click: None,
        }
    }

    /// Mark the item as the current location.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Disable the item.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the activation handler.
    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Get the item ID.
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// Get the item label.
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// Whether the item marks the current location.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Whether the item is disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// Generic top navigation bar.
#[derive(IntoElement)]
pub struct TopNav {
    id: ElementId,
    brand: Option<AnyElement>,
    items: Vec<TopNavItem>,
    trailing: Option<AnyElement>,
    size: TopNavSize,
    bordered: bool,
    theme: Option<TopNavTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl TopNav {
    /// Create a new empty navigation bar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            brand: None,
            items: Vec::new(),
            trailing: None,
            size: TopNavSize::default(),
            bordered: true,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the brand slot.
    pub fn brand(mut self, element: impl IntoElement) -> Self {
        self.brand = Some(element.into_any_element());
        self
    }

    /// Append a navigation item.
    pub fn item(mut self, item: TopNavItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set the trailing actions slot.
    pub fn trailing(mut self, element: impl IntoElement) -> Self {
        self.trailing = Some(element.into_any_element());
        self
    }

    /// Set the bar size.
    pub fn size(mut self, size: TopNavSize) -> Self {
        self.size = size;
        self
    }

    /// Show or hide the bottom divider.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: TopNavTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit ARIA label.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Navigation).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Render one item with active and disabled states.
    fn render_item(item: TopNavItem, theme: &TopNavTheme, design: &DesignSystem) -> Stateful<Div> {
        let hover_bg = theme.item_hover;
        let mut el = div()
            .id(SharedString::from(format!("top-nav-item-{}", item.id)))
            .px(px(design.spacing.control_padding_x * 0.75))
            .py(px(design.spacing.control_padding_y * 0.5))
            .rounded(px(design.corners.sm))
            .text_sm()
            .child(item.label.clone());

        if item.active {
            el = el
                .bg(theme.item_active)
                .text_color(theme.item_active_text)
                .font_weight(FontWeight::SEMIBOLD);
        } else if item.disabled {
            el = el.text_color(theme.item_text).opacity(0.5);
        } else {
            el = el
                .text_color(theme.item_text)
                .cursor_pointer()
                .hover(move |style| style.bg(hover_bg));
        }

        if let (false, Some(handler)) = (item.disabled, item.on_click) {
            let mouse_handler = handler.clone();
            el = el.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                mouse_handler(window, cx);
            });
            el = el.on_key_down(move |event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                    handler(window, cx);
                    cx.stop_propagation();
                }
            });
        }

        el
    }

    /// Build the bar with theme.
    pub fn build_with_theme(self, theme: &TopNavTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the bar with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &TopNavTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let native_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| SharedString::from("Top navigation"));
        let native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Navigation));

        let mut bar = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(design.spacing.control_gap))
            .px(px(design.spacing.control_padding_x))
            .h(self.size.bar_height(design))
            .w_full()
            .flex_shrink_0()
            .bg(theme.background);

        if self.bordered {
            bar = bar.border_b_1().border_color(theme.border);
        }
        if let Some(brand) = self.brand {
            bar = bar.child(div().flex_shrink_0().child(brand));
        }

        let mut row = div()
            .flex()
            .flex_1()
            .min_w_0()
            .items_center()
            .gap(px(design.spacing.grid_unit * 0.5));
        for item in self.items {
            row = row.child(Self::render_item(item, theme, design));
        }
        bar = bar.child(row);

        if let Some(trailing) = self.trailing {
            bar = bar.child(div().flex_shrink_0().child(trailing));
        }

        apply_native_accessibility(bar, native_label, &native_props)
    }
}

impl RenderOnce for TopNav {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self
                .aria_label
                .clone()
                .unwrap_or_else(|| SharedString::from("Top navigation")),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Navigation)),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| TopNavTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{TopNav, TopNavItem, TopNavSize, TopNavTheme};
    use gpui::rgba;

    #[test]
    fn nav_defaults_to_bordered_standard_bar() {
        let nav = TopNav::new("nav");
        assert_eq!(nav.size, TopNavSize::Md);
        assert!(nav.bordered);
        assert!(nav.items.is_empty());
        assert!(nav.brand.is_none());
        assert!(nav.trailing.is_none());
        assert!(nav.theme.is_none());
    }

    #[test]
    fn nav_item_builders_store_state() {
        let item = TopNavItem::new("docs", "Docs")
            .active(true)
            .disabled(false)
            .on_click(|_, _| {});
        assert_eq!(item.id().as_ref(), "docs");
        assert_eq!(item.label().as_ref(), "Docs");
        assert!(item.is_active());
        assert!(!item.is_disabled());

        let nav = TopNav::new("nav")
            .brand(gpui::div())
            .item(item)
            .trailing(gpui::div())
            .size(TopNavSize::Lg)
            .bordered(false);
        assert_eq!(nav.items.len(), 1);
        assert_eq!(nav.size, TopNavSize::Lg);
        assert!(!nav.bordered);
    }

    #[test]
    fn nav_theme_defaults_match_compiled_tokens() {
        let theme = TopNavTheme::default();
        assert_eq!(theme.background, rgba(0x1e1e1eff));
        assert_eq!(theme.item_active, rgba(0x007accff));
        assert_eq!(theme.item_active_text, rgba(0xffffffff));
    }

    #[test]
    fn nav_bar_height_grows_with_size() {
        let design = crate::design::neutral_design();
        let sm = TopNavSize::Sm.bar_height(&design);
        let md = TopNavSize::Md.bar_height(&design);
        let lg = TopNavSize::Lg.bar_height(&design);
        assert!(sm < md);
        assert!(md < lg);
    }
}
