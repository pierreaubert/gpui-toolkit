//! Tabs component for tabbed navigation
//!
//! Provides a horizontal tab bar with content panels and theming support.

use crate::ComponentTheme;
use crate::theme::{ThemeExt, glow_shadow};
use gpui::prelude::*;
use gpui::*;

/// Theme colors for tabs styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct TabsTheme {
    /// Background color for the container (Pills variant)
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub container_bg: Rgba,
    /// Border color for the container (Underline variant)
    #[theme(default = 0x3a3a3aff, from = border)]
    pub container_border: Rgba,
    /// Background color for selected tab
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub selected_bg: Rgba,
    /// Background color for selected tab on hover
    #[theme(default = 0x4a4a4aff, from = surface_hover)]
    pub selected_hover_bg: Rgba,
    /// Background color for unselected tab on hover
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub hover_bg: Rgba,
    /// Accent color (underline, selected pill)
    #[theme(default = 0x007accff, from = accent)]
    pub accent: Rgba,
    /// Text color for selected tab
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_selected: Rgba,
    /// Text color for unselected tab
    #[theme(default = 0x888888ff, from = text_muted)]
    pub text_unselected: Rgba,
    /// Text color on hover
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text_hover: Rgba,
    /// Badge background color
    #[theme(default = 0x555555ff, from = muted)]
    pub badge_bg: Rgba,
    /// Close button color
    #[theme(default = 0x888888ff, from = text_muted)]
    pub close_color: Rgba,
    /// Close button hover color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub close_hover_color: Rgba,
    /// Icon color for selected tab (defaults to text_selected if not set)
    #[theme(default_expr = "None", from_expr = "None")]
    pub icon_selected: Option<Rgba>,
    /// Icon color for unselected tab (defaults to accent if not set)
    #[theme(default_expr = "None", from_expr = "None")]
    pub icon_unselected: Option<Rgba>,
}

/// Tab visual variant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabVariant {
    /// Underline indicator (default)
    #[default]
    Underline,
    /// Enclosed tabs with background
    Enclosed,
    /// Pill-shaped tabs
    Pills,
    /// Vertical card style: icon on top, label in middle, badge below
    VerticalCard,
}

/// Factory function type for creating icons with a specific color
pub type IconFactory = Box<dyn Fn(Rgba) -> AnyElement + 'static>;

/// A single tab item
pub struct TabItem {
    id: SharedString,
    label: SharedString,
    icon: Option<SharedString>,
    custom_icon: Option<AnyElement>,
    /// Icon factory that creates the icon with the given color at render time
    icon_factory: Option<IconFactory>,
    badge: Option<SharedString>,
    disabled: bool,
    closeable: bool,
}

impl TabItem {
    /// Create a new tab item
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            custom_icon: None,
            icon_factory: None,
            badge: None,
            disabled: false,
            closeable: false,
        }
    }

    /// Add a text/emoji icon
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Add a custom icon element (e.g., SVG)
    /// Note: If the icon needs to change color based on selection state,
    /// use `icon_with_color` instead.
    pub fn custom_icon(mut self, icon: impl IntoElement) -> Self {
        self.custom_icon = Some(icon.into_any_element());
        self
    }

    /// Add an icon that will be created with the correct color at render time.
    /// The factory function receives the icon color (based on selection state)
    /// and should return the icon element with that color applied.
    pub fn icon_with_color<F>(mut self, factory: F) -> Self
    where
        F: Fn(Rgba) -> AnyElement + 'static,
    {
        self.icon_factory = Some(Box::new(factory));
        self
    }

    /// Add a badge
    pub fn badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Disable the tab
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Make the tab closeable
    pub fn closeable(mut self, closeable: bool) -> Self {
        self.closeable = closeable;
        self
    }

    /// Get the tab ID
    pub fn id(&self) -> &SharedString {
        &self.id
    }
}

/// A tabs component with theming support
pub struct Tabs {
    id: ElementId,
    tabs: Vec<TabItem>,
    selected_index: usize,
    variant: TabVariant,
    theme: Option<TabsTheme>,
    on_change: Option<Box<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    on_close: Option<Box<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
    focus_handle: Option<FocusHandle>,
}

impl Tabs {
    /// Create a new tabs component with an ID
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            tabs: Vec::new(),
            selected_index: 0,
            variant: TabVariant::default(),
            theme: None,
            on_change: None,
            on_close: None,
            focus_handle: None,
        }
    }

    /// Set the focus handle for keyboard navigation
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set the tab items
    pub fn tabs(mut self, tabs: Vec<TabItem>) -> Self {
        self.tabs = tabs;
        self
    }

    /// Set the selected tab index
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    /// Set the visual variant
    pub fn variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the theme
    pub fn theme(mut self, theme: TabsTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set the tab change handler
    pub fn on_change(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Set the tab close handler
    pub fn on_close(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Box::new(handler));
        self
    }

    /// Build into element with theme
    ///
    /// # Keyboard Navigation
    /// - Left/Right arrows: Navigate between tabs
    /// - Home: Select first tab
    /// - End: Select last tab
    pub fn build_with_theme(self, global_theme: &TabsTheme, cx: &mut App) -> Stateful<Div> {
        let theme = self.theme.as_ref().unwrap_or(global_theme);

        // Get or create focus handle
        let focus_handle = self.focus_handle.unwrap_or_else(|| cx.focus_handle());

        let mut container = div()
            .id(self.id.clone())
            .track_focus(&focus_handle)
            .flex()
            .items_center()
            .focusable();

        // Apply variant-specific container styling
        match self.variant {
            TabVariant::Underline => {
                // No border on container - we'll add underlines per-tab
            }
            TabVariant::Enclosed => {
                container = container.gap_1();
            }
            TabVariant::Pills => {
                container = container.gap_2().p_1().bg(theme.container_bg).rounded_lg();
            }
            TabVariant::VerticalCard => {
                container = container.gap_2().p_1().bg(theme.container_bg).rounded_lg();
            }
        }

        // Wrap callbacks in Rc for safe sharing across closures
        let on_change_rc = self.on_change.map(|f| std::rc::Rc::new(f));
        let on_close_rc = self.on_close.map(|f| std::rc::Rc::new(f));

        // Capture tab count before consuming tabs
        let tab_count = self.tabs.len();

        for (index, tab) in self.tabs.into_iter().enumerate() {
            let is_selected = index == self.selected_index;
            let tab_id = tab.id.clone();
            let label = tab.label;
            let icon = tab.icon;
            let custom_icon = tab.custom_icon;
            let icon_factory = tab.icon_factory;
            let badge = tab.badge;
            let disabled = tab.disabled;
            let closeable = tab.closeable;

            let on_change = on_change_rc.clone();
            let on_close = on_close_rc.clone();

            // For Underline variant, we wrap the tab content and underline in a flex column
            let tab_element = if self.variant == TabVariant::Underline {
                let mut tab_content = div()
                    .id(SharedString::from(format!("tab-{}", tab_id)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_2();

                if is_selected {
                    tab_content = tab_content
                        .text_color(theme.text_selected)
                        .font_weight(FontWeight::SEMIBOLD);
                } else {
                    let hover_color = theme.text_hover;
                    tab_content = tab_content
                        .text_color(theme.text_unselected)
                        .hover(move |s| s.text_color(hover_color));
                }

                if disabled {
                    tab_content = tab_content.opacity(0.5).cursor_not_allowed();
                } else {
                    tab_content = tab_content.cursor_pointer();

                    if let Some(ref handler) = on_change {
                        let idx = index;
                        let handler = handler.clone();
                        tab_content = tab_content.on_mouse_down(
                            MouseButton::Left,
                            move |_event, window, cx| {
                                handler(idx, window, cx);
                            },
                        );
                    }
                }

                // Add icon
                if let Some(custom_icon) = custom_icon {
                    tab_content = tab_content.child(custom_icon);
                } else if let Some(icon) = icon {
                    tab_content = tab_content.child(div().text_sm().child(icon));
                }

                // Add label
                tab_content = tab_content.child(div().text_sm().child(label));

                // Add badge
                if let Some(badge) = badge {
                    tab_content = tab_content.child(
                        div()
                            .text_xs()
                            .px_1()
                            .py(px(1.0))
                            .bg(theme.badge_bg)
                            .rounded(px(3.0))
                            .child(badge),
                    );
                }

                // Add close button
                if closeable {
                    let id = tab_id.clone();
                    let close_color = theme.close_color;
                    let close_hover = theme.close_hover_color;
                    let mut close_btn = div()
                        .id(SharedString::from(format!("tab-close-{}", tab_id)))
                        .text_xs()
                        .text_color(close_color)
                        .hover(move |s| s.text_color(close_hover));

                    if let Some(ref handler) = on_close {
                        let handler = handler.clone();
                        close_btn = close_btn.on_mouse_down(
                            MouseButton::Left,
                            move |_event, window, cx| {
                                handler(&id, window, cx);
                            },
                        );
                    }

                    tab_content = tab_content.child(close_btn.child("×"));
                }

                // Create the underline - accent color for selected, border color for unselected
                let underline = if is_selected {
                    div().h(px(2.0)).w_full().bg(theme.accent)
                } else {
                    div().h(px(1.0)).w_full().bg(theme.container_border)
                };

                // Wrap in a flex column
                div()
                    .id(SharedString::from(format!("tab-wrapper-{}", tab_id)))
                    .flex()
                    .flex_col()
                    .child(tab_content)
                    .child(underline)
            } else if self.variant == TabVariant::VerticalCard {
                // VerticalCard variant: icon on left (spanning 2 rows), title + number on right
                // +---------------+
                // | ICON | Title  |
                // |      | Number |
                // +---------------+
                let mut tab_el = div()
                    .id(SharedString::from(format!("tab-{}", tab_id)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .min_w(px(90.0));

                if is_selected {
                    tab_el = tab_el
                        .bg(theme.accent)
                        .rounded_lg()
                        .text_color(theme.text_selected);
                } else {
                    let hover_bg = theme.selected_bg;
                    let hover_text = theme.text_hover;
                    tab_el = tab_el
                        .bg(theme.selected_bg)
                        .rounded_lg()
                        .text_color(theme.text_unselected)
                        .hover(move |style| {
                            style
                                .bg(hover_bg)
                                .text_color(hover_text)
                                .shadow(glow_shadow(hover_bg))
                        });
                }

                if disabled {
                    tab_el = tab_el.opacity(0.5).cursor_not_allowed();
                } else {
                    tab_el = tab_el.cursor_pointer();

                    if let Some(ref handler) = on_change {
                        let idx = index;
                        let handler = handler.clone();
                        tab_el =
                            tab_el.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                                handler(idx, window, cx);
                            });
                    }
                }

                // Icon on left (large, spans both rows visually)
                // Apply appropriate icon color based on selection state
                let icon_color = if is_selected {
                    theme.icon_selected.unwrap_or(theme.text_selected)
                } else {
                    theme.icon_unselected.unwrap_or(theme.accent)
                };
                // Prefer icon_factory (creates icon with explicit color) over custom_icon
                if let Some(factory) = icon_factory {
                    // Create the icon with the correct color at render time
                    let icon_element = factory(icon_color);
                    tab_el = tab_el.child(div().flex().items_center().child(icon_element));
                } else if let Some(custom_icon) = custom_icon {
                    tab_el = tab_el.child(
                        div()
                            .flex()
                            .items_center()
                            .text_color(icon_color)
                            .child(custom_icon),
                    );
                } else if let Some(icon) = icon {
                    tab_el = tab_el.child(
                        div()
                            .flex()
                            .items_center()
                            .text_xl()
                            .text_color(icon_color)
                            .child(icon),
                    );
                }

                // Right side: Title on top, Number below
                let mut right_col = div().flex().flex_col().gap(px(1.0));

                // Title
                right_col = right_col.child(
                    div()
                        .text_xs()
                        .font_weight(if is_selected {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .child(label),
                );

                // Number/badge below title
                if let Some(badge) = badge {
                    right_col =
                        right_col.child(div().text_sm().font_weight(FontWeight::BOLD).child(badge));
                }

                tab_el = tab_el.child(right_col);

                tab_el
            } else {
                // Non-underline variants (Enclosed, Pills)
                let mut tab_el = div()
                    .id(SharedString::from(format!("tab-{}", tab_id)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_2();

                match self.variant {
                    TabVariant::Enclosed => {
                        if is_selected {
                            tab_el = tab_el
                                .bg(theme.selected_bg)
                                .rounded_t_md()
                                .text_color(theme.text_selected);
                        } else {
                            let hover_bg = theme.hover_bg;
                            let hover_text = theme.text_hover;
                            tab_el = tab_el
                                .text_color(theme.text_unselected)
                                .hover(move |style| {
                                    style
                                        .bg(hover_bg)
                                        .text_color(hover_text)
                                        .shadow(glow_shadow(hover_bg))
                                });
                        }
                    }
                    TabVariant::Pills => {
                        if is_selected {
                            tab_el = tab_el
                                .bg(theme.accent)
                                .rounded_md()
                                .text_color(theme.text_selected);
                        } else {
                            let hover_bg = theme.selected_bg;
                            let hover_text = theme.text_hover;
                            tab_el = tab_el.rounded_md().text_color(theme.text_unselected).hover(
                                move |style| {
                                    style
                                        .bg(hover_bg)
                                        .text_color(hover_text)
                                        .shadow(glow_shadow(hover_bg))
                                },
                            );
                        }
                    }
                    TabVariant::Underline | TabVariant::VerticalCard => unreachable!(),
                }

                if disabled {
                    tab_el = tab_el.opacity(0.5).cursor_not_allowed();
                } else {
                    tab_el = tab_el.cursor_pointer();

                    if let Some(ref handler) = on_change {
                        let idx = index;
                        let handler = handler.clone();
                        tab_el =
                            tab_el.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                                handler(idx, window, cx);
                            });
                    }
                }

                // Add icon
                if let Some(custom_icon) = custom_icon {
                    tab_el = tab_el.child(custom_icon);
                } else if let Some(icon) = icon {
                    tab_el = tab_el.child(div().text_sm().child(icon));
                }

                // Add label
                tab_el = tab_el.child(div().text_sm().child(label));

                // Add badge
                if let Some(badge) = badge {
                    tab_el = tab_el.child(
                        div()
                            .text_xs()
                            .px_1()
                            .py(px(1.0))
                            .bg(theme.badge_bg)
                            .rounded(px(3.0))
                            .child(badge),
                    );
                }

                // Add close button
                if closeable {
                    let id = tab_id.clone();
                    let close_color = theme.close_color;
                    let close_hover = theme.close_hover_color;
                    let mut close_btn = div()
                        .id(SharedString::from(format!("tab-close-{}", tab_id)))
                        .text_xs()
                        .text_color(close_color)
                        .hover(move |s| s.text_color(close_hover));

                    if let Some(ref handler) = on_close {
                        let handler = handler.clone();
                        close_btn = close_btn.on_mouse_down(
                            MouseButton::Left,
                            move |_event, window, cx| {
                                handler(&id, window, cx);
                            },
                        );
                    }

                    tab_el = tab_el.child(close_btn.child("×"));
                }

                tab_el
            };

            container = container.child(tab_element);
        }

        // Add keyboard navigation
        let selected = self.selected_index;
        let on_change_key = on_change_rc.clone();
        let focus_handle_key = focus_handle.clone();

        container = container.on_key_down(move |event, window, cx| {
            if !focus_handle_key.is_focused(window) {
                return;
            }

            let key = event.keystroke.key.as_str();
            let new_index = match key {
                "left" => {
                    if selected > 0 {
                        Some(selected - 1)
                    } else {
                        None
                    }
                }
                "right" => {
                    if selected + 1 < tab_count {
                        Some(selected + 1)
                    } else {
                        None
                    }
                }
                "home" => Some(0),
                "end" => {
                    if tab_count > 0 {
                        Some(tab_count - 1)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(new_idx) = new_index {
                cx.stop_propagation();
                if let Some(ref handler) = on_change_key {
                    handler(new_idx, window, cx);
                }
            }
        });

        container
    }
}

impl Default for Tabs {
    fn default() -> Self {
        Self::new("tabs")
    }
}

impl RenderOnce for Tabs {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let tabs_theme = TabsTheme::from(&global_theme);
        self.build_with_theme(&tabs_theme, cx)
    }
}

impl IntoElement for Tabs {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
