//! Breadcrumbs component
//!
//! Navigation breadcrumb trail.

use crate::theme::{Theme, ThemeExt};
use gpui::prelude::*;
use gpui::*;

/// A single breadcrumb item
#[derive(Clone)]
pub struct BreadcrumbItem {
    id: SharedString,
    label: SharedString,
    icon: Option<SharedString>,
    href: Option<SharedString>,
}

impl BreadcrumbItem {
    /// Create a new breadcrumb item
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            href: None,
        }
    }

    /// Set icon
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set href/path
    pub fn href(mut self, href: impl Into<SharedString>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Get the item ID
    pub fn id(&self) -> &SharedString {
        &self.id
    }
}

/// Breadcrumbs separator style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BreadcrumbSeparator {
    /// Forward slash (default)
    #[default]
    Slash,
    /// Chevron arrow
    Chevron,
    /// Dot
    Dot,
}

impl BreadcrumbSeparator {
    fn char(&self) -> &'static str {
        match self {
            BreadcrumbSeparator::Slash => "/",
            BreadcrumbSeparator::Chevron => "›",
            BreadcrumbSeparator::Dot => "•",
        }
    }
}

/// A breadcrumbs navigation component
pub struct Breadcrumbs {
    items: Vec<BreadcrumbItem>,
    separator: BreadcrumbSeparator,
    on_click: Option<Box<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
}

impl Breadcrumbs {
    /// Create new breadcrumbs
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            separator: BreadcrumbSeparator::default(),
            on_click: None,
        }
    }

    /// Set items
    pub fn items(mut self, items: Vec<BreadcrumbItem>) -> Self {
        self.items = items;
        self
    }

    /// Set separator style
    pub fn separator(mut self, separator: BreadcrumbSeparator) -> Self {
        self.separator = separator;
        self
    }

    /// Set click handler
    pub fn on_click(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Build into element with theme
    pub fn build_with_theme(self, theme: &Theme) -> Div {
        let mut container = div().flex().items_center().gap_2().text_sm();

        let last_idx = self.items.len().saturating_sub(1);

        for (idx, item) in self.items.iter().enumerate() {
            let is_last = idx == last_idx;
            let item_id = item.id.clone();

            // Separator (except for first item)
            if idx > 0 {
                container = container.child(
                    div()
                        .text_color(theme.text_muted)
                        .child(self.separator.char()),
                );
            }

            // Breadcrumb item
            let mut crumb = div()
                .id(SharedString::from(format!("breadcrumb-{}", item_id)))
                .flex()
                .items_center()
                .gap_1();

            if is_last {
                // Current page - not clickable
                crumb = crumb
                    .text_color(theme.text_primary)
                    .font_weight(FontWeight::MEDIUM);
            } else {
                // Previous pages - clickable
                let hover_color = theme.accent;
                crumb = crumb
                    .text_color(theme.text_muted)
                    .cursor_pointer()
                    .hover(move |s| s.text_color(hover_color));

                // Click handler
                if let Some(ref handler) = self.on_click {
                    let handler_ptr: *const dyn Fn(&SharedString, &mut Window, &mut App) =
                        handler.as_ref();
                    let id = item_id.clone();
                    crumb =
                        crumb.on_mouse_up(MouseButton::Left, move |_event, window, cx| unsafe {
                            (*handler_ptr)(&id, window, cx);
                        });
                }
            }

            // Icon
            if let Some(icon) = &item.icon {
                crumb = crumb.child(div().child(icon.clone()));
            }

            // Label
            crumb = crumb.child(item.label.clone());

            container = container.child(crumb);
        }

        container
    }
}

impl Default for Breadcrumbs {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Breadcrumbs {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        self.build_with_theme(&theme)
    }
}

impl IntoElement for Breadcrumbs {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
