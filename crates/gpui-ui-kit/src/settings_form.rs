//! SettingsForm component
//!
//! A structured form layout for settings screens with labeled rows,
//! section headers, and consistent spacing.
//!
//! # Usage
//!
//! ```ignore
//! SettingsForm::new("audio-settings")
//!     .section("Playback")
//!     .row(SettingsRow::new("Volume")
//!         .description("Master output volume")
//!         .control(slider_element))
//!     .row(SettingsRow::new("Mute")
//!         .control(toggle_element))
//! ```

use crate::ComponentTheme;
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, Pixels, Rgba, SharedString, Stateful, Window, div,
    px,
};

/// Theme colors for settings form styling
#[derive(Debug, Clone, ComponentTheme)]
pub struct SettingsFormTheme {
    /// Section header background
    #[theme(default = 0x1a1a1aff, from = surface)]
    pub section_bg: Rgba,
    /// Section header text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub section_text: Rgba,
    /// Row label text color
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub label_text: Rgba,
    /// Row description text color
    #[theme(default = 0x777777ff, from = text_muted)]
    pub description_text: Rgba,
    /// Row separator color
    #[theme(default = 0x2a2a2aff, from = border)]
    pub separator: Rgba,
}

/// A single row in a settings form
pub struct SettingsRow {
    label: SharedString,
    description: Option<SharedString>,
    control: Option<AnyElement>,
    label_width: Pixels,
}

impl SettingsRow {
    /// Create a new settings row
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            description: None,
            control: None,
            label_width: px(200.0),
        }
    }

    /// Set a description shown below the label
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the control element (toggle, slider, input, etc.)
    pub fn control(mut self, element: impl IntoElement) -> Self {
        self.control = Some(element.into_any_element());
        self
    }

    /// Set label column width
    pub fn label_width(mut self, width: Pixels) -> Self {
        self.label_width = width;
        self
    }
}

/// An entry in the settings form (either a section header or a row)
enum SettingsEntry {
    Section(SharedString),
    Row(SettingsRow),
}

/// A settings form layout component
pub struct SettingsForm {
    id: ElementId,
    entries: Vec<SettingsEntry>,
    label_width: Pixels,
}

impl SettingsForm {
    /// Create a new settings form
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            label_width: px(200.0),
        }
    }

    /// Add a section header
    pub fn section(mut self, title: impl Into<SharedString>) -> Self {
        self.entries.push(SettingsEntry::Section(title.into()));
        self
    }

    /// Add a settings row
    pub fn row(mut self, row: SettingsRow) -> Self {
        self.entries.push(SettingsEntry::Row(row));
        self
    }

    /// Set default label width for all rows
    pub fn label_width(mut self, width: Pixels) -> Self {
        self.label_width = width;
        self
    }

    /// Build the form with theme
    pub fn build_with_theme(self, theme: &SettingsFormTheme) -> Stateful<Div> {
        let mut container = div().id(self.id).flex().flex_col().w_full();

        for (i, entry) in self.entries.into_iter().enumerate() {
            match entry {
                SettingsEntry::Section(title) => {
                    if i > 0 {
                        container = container.child(div().h(px(8.0)));
                    }
                    container = container.child(
                        div()
                            .w_full()
                            .px_4()
                            .py_2()
                            .bg(theme.section_bg)
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.section_text)
                            .child(title),
                    );
                }
                SettingsEntry::Row(row) => {
                    let label_w = if row.label_width != px(200.0) {
                        row.label_width
                    } else {
                        self.label_width
                    };

                    let mut label_col = div()
                        .w(label_w)
                        .flex_shrink_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.label_text)
                                .child(row.label),
                        );

                    if let Some(desc) = row.description {
                        label_col = label_col.child(
                            div()
                                .text_xs()
                                .text_color(theme.description_text)
                                .child(desc),
                        );
                    }

                    let mut row_el = div()
                        .w_full()
                        .flex()
                        .items_center()
                        .px_4()
                        .py_3()
                        .border_b_1()
                        .border_color(theme.separator)
                        .child(label_col);

                    if let Some(control) = row.control {
                        row_el = row_el.child(div().flex_1().flex().justify_end().child(control));
                    }

                    container = container.child(row_el);
                }
            }
        }

        container
    }
}

impl RenderOnce for SettingsForm {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let global_theme = cx.theme();
        let theme = SettingsFormTheme::from(global_theme);
        self.build_with_theme(&theme)
    }
}

impl IntoElement for SettingsForm {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
