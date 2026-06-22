//! SearchBar component
//!
//! A search input with icon, clear button, and optional autocomplete support.
//!
//! # Usage
//!
//! ```ignore
//! SearchBar::new("library-search")
//!     .placeholder("Search albums...")
//!     .value(current_query)
//!     .on_change(|query, window, cx| { /* filter results */ })
//!     .on_submit(|query, window, cx| { /* navigate to result */ })
//! ```

use crate::ComponentTheme;
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole};
use crate::input::{Input, InputSize, InputVariant};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{App, Div, ElementId, MouseButton, Rgba, SharedString, Stateful, Window, div, px};
use std::rc::Rc;

/// Theme colors for search bar styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct SearchBarTheme {
    /// Background color
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub background: Rgba,
    /// Border color
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Focused border color
    #[theme(default = 0x007accff, from = accent)]
    pub border_focus: Rgba,
    /// Placeholder text color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub placeholder: Rgba,
    /// Input text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text: Rgba,
    /// Icon color
    #[theme(default = 0x777777ff, from = text_muted)]
    pub icon: Rgba,
    /// Clear button color
    #[theme(default = 0x777777ff, from = text_muted)]
    pub clear_button: Rgba,
    /// Clear button hover color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub clear_button_hover: Rgba,
}

/// Search bar size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchBarSize {
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
}

/// A search bar component that wraps an Input with search-specific UX
pub struct SearchBar {
    id: ElementId,
    value: SharedString,
    placeholder: SharedString,
    size: SearchBarSize,
    show_icon: bool,
    show_clear: bool,
    on_change: Option<Rc<dyn Fn(&str, &mut Window, &mut App) + 'static>>,
    on_submit: Option<Rc<dyn Fn(&str, &mut Window, &mut App) + 'static>>,
    on_escape: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl SearchBar {
    /// Create a new search bar
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: "".into(),
            placeholder: "Search...".into(),
            size: SearchBarSize::default(),
            show_icon: true,
            show_clear: true,
            on_change: None,
            on_submit: None,
            on_escape: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the current value
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    /// Set placeholder text
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set size
    pub fn size(mut self, size: SearchBarSize) -> Self {
        self.size = size;
        self
    }

    /// Show or hide the search icon
    pub fn show_icon(mut self, show: bool) -> Self {
        self.show_icon = show;
        self
    }

    /// Show or hide the clear button
    pub fn show_clear(mut self, show: bool) -> Self {
        self.show_clear = show;
        self
    }

    /// Called on every text change (live filtering)
    pub fn on_change(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// Called when Enter is pressed
    pub fn on_submit(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_submit = Some(Rc::new(handler));
        self
    }

    /// Set an explicit ARIA label (overrides the placeholder fallback)
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Override the default ARIA role (Search)
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Called when Escape is pressed
    pub fn on_escape(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_escape = Some(Rc::new(handler));
        self
    }

    /// Build the search bar with theme.
    ///
    /// This renders the visual container. The actual text editing is delegated
    /// to the Input component — callers should compose SearchBar with Input
    /// or handle text input in their own way.
    pub fn build_with_theme(self, theme: &SearchBarTheme) -> Stateful<Div> {
        let clear_id = (self.id.clone(), "search-clear");
        let input_size = match self.size {
            SearchBarSize::Sm => InputSize::Sm,
            SearchBarSize::Md => InputSize::Md,
            SearchBarSize::Lg => InputSize::Lg,
        };
        let has_value = !self.value.is_empty();
        let on_change = self.on_change.clone();
        let on_submit = self.on_submit.clone();
        let on_escape = self.on_escape.clone();

        let mut container = div()
            .id((self.id.clone(), "container"))
            .flex()
            .items_center()
            .gap_2()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded(px(6.0));

        // Search icon
        if self.show_icon {
            container = container.child(div().text_color(theme.icon).text_sm().child("⌕"));
        }

        let mut input = Input::new(self.id.clone())
            .value(self.value.clone())
            .placeholder(self.placeholder.clone())
            .size(input_size)
            .variant(InputVariant::Flushed)
            .bg_color(theme.background)
            .text_color(theme.text)
            .placeholder_color(theme.placeholder)
            .border_color(theme.background);

        if let Some(handler) = on_change.clone() {
            input = input.on_text_change(move |query, window, cx| {
                handler(query.as_str(), window, cx);
            });
        }
        if let Some(handler) = on_submit {
            input = input.on_change(move |query, window, cx| {
                handler(query, window, cx);
            });
        }
        if let Some(handler) = on_escape {
            input = input.on_edit_end(move |value, window, cx| {
                if value.is_none() {
                    handler(window, cx);
                }
            });
        }

        container = container.child(div().flex_1().child(input));

        // Clear button
        if self.show_clear && has_value {
            let clear_color = theme.clear_button;
            let clear_hover = theme.clear_button_hover;

            let mut clear_btn = div()
                .id(clear_id)
                .cursor_pointer()
                .text_xs()
                .text_color(clear_color)
                .hover(move |s| s.text_color(clear_hover))
                .child("×");

            if let Some(handler_rc) = on_change {
                clear_btn = clear_btn.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    handler_rc("", window, cx);
                });
            }

            container = container.child(clear_btn);
        }

        container
    }
}

impl RenderOnce for SearchBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let effective_label = self
            .aria_label
            .clone()
            .unwrap_or_else(|| self.placeholder.clone());
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: effective_label,
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Search)),
        });

        let global_theme = cx.theme();
        let theme = SearchBarTheme::from(global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for SearchBar {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
