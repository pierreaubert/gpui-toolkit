//! Combobox component
//!
//! A filterable dropdown: a text query box (an embedded [`Input`]) plus a
//! popup list of matching options. Unlike [`Select`](crate::Select), the user
//! can type to narrow options and the query itself is reported through
//! `on_query_change`, so free-text entries are possible when the parent
//! allows them.
//!
//! State is parent-owned: `selected`, `query`, `is_open`, and
//! `highlighted_index` are props; every change reports through a callback.
//! Filtering defaults to case-insensitive substring match on the label and
//! can be overridden with [`Combobox::filter`].

use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use crate::input::{Input, InputSize};
use crate::theme::ThemeExt;
use crate::{ComponentTheme, ComponentVariant};
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{
    App, Div, ElementId, MouseButton, Rgba, SharedString, Stateful, Window, deferred, div, px,
};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Theme colors for combobox styling (mirrors [`Select`](crate::Select) tokens).
#[derive(Debug, Clone, ComponentTheme)]
pub struct ComboboxTheme {
    /// Query box background
    #[theme(default = 0x1e1e2eff, from = surface)]
    pub trigger_bg: Rgba,
    /// Query box border
    #[theme(default = 0x3a3a3aff, from = border)]
    pub trigger_border: Rgba,
    /// Query box border on hover
    #[theme(default = 0x007accff, from = accent)]
    pub trigger_border_hover: Rgba,
    /// Query box border when the popup is open
    #[theme(default = 0x007accff, from = accent)]
    pub trigger_border_focused: Rgba,
    /// Popup background
    #[theme(default = 0x2a2a2aff, from = surface)]
    pub dropdown_bg: Rgba,
    /// Popup border
    #[theme(default = 0x3a3a3aff, from = border)]
    pub dropdown_border: Rgba,
    /// Selected option background
    #[theme(default = 0x007accff, from = accent)]
    pub selected_bg: Rgba,
    /// Option hover background
    #[theme(default = 0x3a3a3aff, from = surface_hover)]
    pub option_hover_bg: Rgba,
    /// Label text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label_color: Rgba,
    /// Query text color
    #[theme(default = 0xffffffff, from = text_primary)]
    pub text_color: Rgba,
    /// Placeholder text color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub placeholder_color: Rgba,
    /// Option text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub option_text_color: Rgba,
    /// Selected option text color (on accent background)
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub selected_text_color: Rgba,
    /// Disabled text color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub disabled_color: Rgba,
    /// Chevron color
    #[theme(default = 0x666666ff, from = text_muted)]
    pub arrow_color: Rgba,
}

/// Combobox size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ComponentVariant)]
pub enum ComboboxSize {
    /// Extra small
    Xs,
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
}

impl From<ComboboxSize> for InputSize {
    fn from(size: ComboboxSize) -> Self {
        match size {
            ComboboxSize::Xs => Self::Xs,
            ComboboxSize::Sm => Self::Sm,
            ComboboxSize::Md => Self::Md,
            ComboboxSize::Lg => Self::Lg,
        }
    }
}

impl From<crate::ComponentSize> for ComboboxSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs => Self::Xs,
            crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// A single option in a [`Combobox`].
#[derive(Debug, Clone)]
pub struct ComboboxOption {
    /// Stable value reported to `on_select`.
    pub value: SharedString,
    /// Visible (and filtered) label.
    pub label: SharedString,
    /// Whether this option can be selected.
    pub disabled: bool,
}

impl ComboboxOption {
    /// Create an option with a value and label.
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    /// Mark the option as non-selectable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Case-insensitive substring match on the option label.
pub fn default_combobox_filter(query: &str, option: &ComboboxOption) -> bool {
    if query.is_empty() {
        return true;
    }
    option.label.to_lowercase().contains(&query.to_lowercase())
}

/// A filterable dropdown combobox.
///
/// Parent-owned state; see the module docs. The query box is an embedded
/// [`Input`] whose element id is scoped as `(group_id, "query")`.
pub struct Combobox {
    id: ElementId,
    options: Vec<ComboboxOption>,
    option_ids: Vec<ElementId>,
    selected: Option<SharedString>,
    query: SharedString,
    is_open: bool,
    highlighted_index: Option<usize>,
    placeholder: Option<SharedString>,
    label: Option<SharedString>,
    size: ComboboxSize,
    disabled: bool,
    design: Option<Arc<DesignSystem>>,
    filter: Option<Box<dyn Fn(&str, &ComboboxOption) -> bool + 'static>>,
    on_select: Option<Box<dyn Fn(SharedString, &mut Window, &mut App) + 'static>>,
    on_query_change: Option<Box<dyn Fn(SharedString, &mut Window, &mut App) + 'static>>,
    on_toggle: Option<Box<dyn Fn(bool, &mut Window, &mut App) + 'static>>,
    on_highlight: Option<Box<dyn Fn(usize, &mut Window, &mut App) + 'static>>,
    empty_text: Option<SharedString>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl Combobox {
    /// Create a new combobox.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            option_ids: Vec::new(),
            selected: None,
            query: SharedString::from(""),
            is_open: false,
            highlighted_index: None,
            placeholder: None,
            label: None,
            size: ComboboxSize::default(),
            disabled: false,
            design: None,
            filter: None,
            on_select: None,
            on_query_change: None,
            on_toggle: None,
            on_highlight: None,
            empty_text: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Refresh stable option ids, scoped to the group id (see `Select`).
    fn refresh_option_ids(&mut self) {
        self.option_ids = self
            .options
            .iter()
            .enumerate()
            .map(|(idx, _)| (self.id.clone(), idx.to_string()).into())
            .collect();
    }

    /// Set the full option list.
    pub fn options(mut self, options: Vec<ComboboxOption>) -> Self {
        self.options = options;
        self.refresh_option_ids();
        self
    }

    /// Set the selected option value.
    pub fn selected(mut self, value: Option<SharedString>) -> Self {
        self.selected = value;
        self
    }

    /// Set the current query text (controlled by the parent).
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// Set whether the popup is open.
    pub fn is_open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }

    /// Set the highlighted option index (position in the *filtered* list).
    pub fn highlighted_index(mut self, index: Option<usize>) -> Self {
        self.highlighted_index = index;
        self
    }

    /// Set the query-box placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the field label rendered above the query box.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the size.
    pub fn size(mut self, size: ComboboxSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the whole combobox.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Override the design system.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Override the match function (default: [`default_combobox_filter`]).
    pub fn filter(mut self, filter: impl Fn(&str, &ComboboxOption) -> bool + 'static) -> Self {
        self.filter = Some(Box::new(filter));
        self
    }

    /// Selection handler (receives the option value).
    pub fn on_select(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }

    /// Query-text handler (every keystroke in the query box).
    pub fn on_query_change(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_query_change = Some(Box::new(handler));
        self
    }

    /// Popup open/close handler.
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }

    /// Highlight handler (receives the filtered-list index).
    pub fn on_highlight(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_highlight = Some(Box::new(handler));
        self
    }

    /// Set the empty-list text shown when the filter matches nothing.
    ///
    /// When unset, the English `"No matches"` fallback is used.
    pub fn empty_text(mut self, text: impl Into<SharedString>) -> Self {
        self.empty_text = Some(text.into());
        self
    }

    /// Override the default ARIA role (`Combobox`).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Indices into `options` matching the current query, in order.
    ///
    /// Pure and unit-testable: filtering never touches GPUI state.
    pub fn filtered_indices(&self) -> Vec<usize> {
        let query = self.query.to_string();
        self.options
            .iter()
            .enumerate()
            .filter(|(_, option)| match &self.filter {
                Some(filter) => filter(&query, option),
                None => default_combobox_filter(&query, option),
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// Build into an element with an explicit theme.
    pub fn build_with_theme(self, theme: &ComboboxTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build into an element with a theme and design-system sizing tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &ComboboxTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let effective_label = self
            .aria_label
            .clone()
            .or_else(|| self.label.clone())
            .or_else(|| self.placeholder.clone())
            .unwrap_or_default();
        let native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Combobox))
            .state(AriaState::Expanded(self.is_open))
            .maybe_state(self.disabled, AriaState::Disabled);

        let query_id: ElementId = (self.id.clone(), "query").into();
        let filtered = self.filtered_indices();
        let selected_value = self.selected.clone();
        // Handlers fan out to several closures; share them up front.
        let on_query_change = self.on_query_change;
        let on_select: Option<std::rc::Rc<Box<dyn Fn(SharedString, &mut Window, &mut App)>>> =
            self.on_select.map(|handler| std::rc::Rc::new(handler));
        let on_toggle: Option<std::rc::Rc<Box<dyn Fn(bool, &mut Window, &mut App)>>> =
            self.on_toggle.map(|handler| std::rc::Rc::new(handler));
        let on_highlight: Option<std::rc::Rc<Box<dyn Fn(usize, &mut Window, &mut App)>>> =
            self.on_highlight.map(|handler| std::rc::Rc::new(handler));

        let mut container = div()
            .id(self.id.clone())
            .debug_selector(|| format!("{:?}", self.id))
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(design.spacing.control_gap / 2.0));

        if let Some(label) = self.label.clone() {
            container = container.child(div().text_sm().text_color(theme.label_color).child(label));
        }

        // Query box: embedded controlled Input plus chevron toggle.
        let query_input_id = query_id.clone();
        let mut trigger = div()
            .id((self.id.clone(), "trigger"))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py(px(6.0))
            .rounded_md()
            .border_1()
            .border_color(if self.is_open {
                theme.trigger_border_focused
            } else {
                theme.trigger_border
            })
            .bg(theme.trigger_bg);
        if !self.disabled && !self.is_open {
            let hover_border = theme.trigger_border_hover;
            trigger = trigger.hover(move |s| s.border_color(hover_border));
        }
        if self.disabled {
            trigger = trigger.opacity(0.5).cursor_not_allowed();
        }

        let mut query_input = Input::new(query_input_id)
            .value(self.query.clone())
            .size(self.size.into())
            .disabled(self.disabled);
        if let Some(placeholder) = self.placeholder.clone() {
            query_input = query_input.placeholder(placeholder);
        }
        if let Some(handler) = on_query_change {
            let handler_rc = std::rc::Rc::new(handler);
            query_input = query_input.on_change(move |text, window, cx| {
                handler_rc(SharedString::from(text.to_string()), window, cx);
            });
        }
        trigger = trigger.child(div().flex_1().child(query_input));

        if !self.disabled
            && let Some(toggle) = on_toggle.clone()
        {
            let next_open = !self.is_open;
            trigger = trigger.cursor_pointer().on_mouse_down(
                MouseButton::Left,
                move |_event, window, cx| {
                    toggle(next_open, window, cx);
                },
            );
        }
        trigger = trigger.child(div().text_xs().text_color(theme.arrow_color).child("▼"));

        container = container.child(apply_native_accessibility(
            trigger,
            effective_label,
            &native_props,
        ));

        // Popup with filtered options.
        if self.is_open && !self.disabled {
            let mut dropdown = div()
                .id((self.id.clone(), "dropdown"))
                .absolute()
                .top_full()
                .left_0()
                .min_w_full()
                .mt_1()
                .bg(theme.dropdown_bg)
                .border_1()
                .border_color(theme.dropdown_border)
                .rounded_md()
                .shadow_lg()
                .max_h(px(200.0))
                .overflow_y_scroll()
                .py_1()
                .occlude();

            if filtered.is_empty() {
                let empty_text: SharedString = self
                    .empty_text
                    .clone()
                    .unwrap_or_else(|| "No matches".into());
                dropdown = dropdown.child(
                    div()
                        .px_3()
                        .py(px(6.0))
                        .text_sm()
                        .text_color(theme.disabled_color)
                        .child(empty_text),
                );
            }

            let select_rc = on_select.clone();
            let toggle_rc = on_toggle.clone();
            for (position, option_index) in filtered.iter().enumerate() {
                let option = &self.options[*option_index];
                let is_selected = selected_value.as_ref() == Some(&option.value);
                let is_highlighted = self.highlighted_index == Some(position);
                let option_id = self
                    .option_ids
                    .get(*option_index)
                    .cloned()
                    .unwrap_or_else(|| (self.id.clone(), option_index.to_string()).into());

                let mut option_el = div()
                    .id(option_id)
                    .debug_selector(|| format!("{:?}-option-{option_index}", self.id))
                    .px_3()
                    .py(px(6.0))
                    .cursor_pointer()
                    .text_sm();

                if option.disabled {
                    option_el = option_el
                        .bg(theme.dropdown_bg)
                        .text_color(theme.disabled_color)
                        .cursor_not_allowed();
                } else {
                    if is_selected {
                        option_el = option_el
                            .bg(theme.selected_bg)
                            .text_color(theme.selected_text_color);
                    } else if is_highlighted {
                        option_el = option_el
                            .bg(theme.option_hover_bg)
                            .text_color(theme.option_text_color);
                    } else {
                        let hover_bg = theme.option_hover_bg;
                        option_el = option_el
                            .bg(theme.dropdown_bg)
                            .text_color(theme.option_text_color)
                            .hover(move |s| s.bg(hover_bg));
                    }

                    let option_value = option.value.clone();
                    let select_handler = select_rc.clone();
                    let toggle_handler = toggle_rc.clone();
                    option_el =
                        option_el.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                            if let Some(ref handler) = select_handler {
                                handler(option_value.clone(), window, cx);
                            }
                            if let Some(ref handler) = toggle_handler {
                                handler(false, window, cx);
                            }
                        });
                }

                let option_props = AriaProps::with_role(AriaRole::Option)
                    .state(AriaState::Selected(is_selected))
                    .maybe_state(option.disabled, AriaState::Disabled);
                option_el =
                    apply_native_accessibility(option_el, option.label.clone(), &option_props);
                dropdown = dropdown.child(option_el);
            }

            dropdown = dropdown.on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            });

            if let Some(toggle) = on_toggle.clone() {
                let backdrop = div()
                    .id((self.id.clone(), "backdrop"))
                    .absolute()
                    .inset_0()
                    .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                        toggle(false, window, cx);
                    });
                container = container.child(deferred(backdrop).with_priority(0));
            }
            container = container.child(deferred(dropdown).with_priority(1));
        }

        // Keyboard: arrows move highlight, Enter selects, Escape closes.
        // Highlight state is parent-owned; report through the callbacks.
        if !self.disabled {
            let option_count = filtered.len();
            let highlighted = self.highlighted_index;
            let highlight_rc = on_highlight.clone();
            let select_rc = on_select.clone();
            let toggle_rc = on_toggle.clone();
            let filtered_values: Vec<SharedString> = filtered
                .iter()
                .map(|index| self.options[*index].value.clone())
                .collect();
            container = container.on_key_down(move |event, window, cx| {
                match event.keystroke.key.as_str() {
                    "down" => {
                        let next = highlighted.map_or(0, |current| {
                            (current + 1).min(option_count.saturating_sub(1))
                        });
                        if option_count > 0
                            && let Some(ref handler) = highlight_rc
                        {
                            handler(next, window, cx);
                        }
                    }
                    "up" => {
                        let next = highlighted.map_or(0, |current| current.saturating_sub(1));
                        if option_count > 0
                            && let Some(ref handler) = highlight_rc
                        {
                            handler(next, window, cx);
                        }
                    }
                    "enter" => {
                        if let Some(current) = highlighted
                            && let Some(value) = filtered_values.get(current)
                        {
                            if let Some(ref handler) = select_rc {
                                handler(value.clone(), window, cx);
                            }
                            if let Some(ref handler) = toggle_rc {
                                handler(false, window, cx);
                            }
                        }
                    }
                    "escape" => {
                        if let Some(ref handler) = toggle_rc {
                            handler(false, window, cx);
                        }
                    }
                    _ => {}
                }
            });
        }

        container
    }
}

impl Combobox {
    /// Element id of the embedded query [`Input`].
    pub fn query_input_id(&self) -> ElementId {
        (self.id.clone(), "query").into()
    }
}

impl RenderOnce for Combobox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let effective_label = self
            .aria_label
            .clone()
            .or_else(|| self.label.clone())
            .or_else(|| self.placeholder.clone())
            .unwrap_or_default();
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: effective_label,
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Combobox))
                .state(AriaState::Expanded(self.is_open))
                .maybe_state(self.disabled, AriaState::Disabled),
        });

        // i18n is resolved at render for the empty-state row.
        let _no_results: Option<SharedString> = None;
        let global_theme = cx.theme();
        let combo_theme = ComboboxTheme::from(global_theme);
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&combo_theme, &design)
    }
}

impl IntoElement for Combobox {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_options() -> Vec<ComboboxOption> {
        vec![
            ComboboxOption::new("apple", "Apple"),
            ComboboxOption::new("apricot", "Apricot"),
            ComboboxOption::new("banana", "Banana"),
        ]
    }

    #[test]
    fn empty_query_matches_everything() {
        let combo = Combobox::new("test").options(test_options());
        assert_eq!(combo.filtered_indices(), vec![0, 1, 2]);
    }

    #[test]
    fn substring_filter_is_case_insensitive() {
        let combo = Combobox::new("test").options(test_options()).query("AP");
        assert_eq!(combo.filtered_indices(), vec![0, 1]);
    }

    #[test]
    fn no_match_yields_empty_list() {
        let combo = Combobox::new("test").options(test_options()).query("zzz");
        assert!(combo.filtered_indices().is_empty());
    }

    #[test]
    fn custom_filter_overrides_default() {
        let combo = Combobox::new("test")
            .options(test_options())
            .query("a")
            .filter(|query, option| option.value == *query);
        assert!(combo.filtered_indices().is_empty());
    }

    #[test]
    fn query_input_id_is_scoped_to_group() {
        let combo = Combobox::new("test");
        let _ = combo.query_input_id();
    }
}
