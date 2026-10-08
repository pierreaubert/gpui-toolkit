//! `FileInput` component
//!
//! A file picker trigger row showing the chosen file name (or a
//! placeholder) next to a browse affordance. The parent owns the native
//! file dialog; the component only displays state and fires `on_browse`
//! when activated with mouse or keyboard.
//!
//! # Usage
//!
//! ```ignore
//! FileInput::new("avatar-upload")
//!     .file_name("portrait.png")
//!     .accept(".png,.jpg")
//!     .on_browse(|_window, _cx| { /* open native picker */ })
//! ```

// Rust guideline compliant 2026-02-21

use crate::ComponentTheme;
use crate::accessibility::{
    AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState, apply_native_accessibility,
};
use crate::theme::ThemeExt;
use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled};
use gpui::{
    App, Div, ElementId, KeyDownEvent, MouseButton, Rgba, SharedString, Stateful, Window, div, px,
};
use gpui_design::DesignSystem;
use std::rc::Rc;
use std::sync::Arc;

/// Theme colors for the file input row.
#[derive(Debug, Clone, ComponentTheme)]
pub struct FileInputTheme {
    /// Field background.
    #[theme(default = 0x1e1e1eff, from = surface)]
    pub background: Rgba,
    /// Field border.
    #[theme(default = 0x3a3a3aff, from = border)]
    pub border: Rgba,
    /// Chosen file name color.
    #[theme(default = 0xccccccff, from = text_secondary)]
    pub text: Rgba,
    /// Placeholder and filter hint color.
    #[theme(default = 0x777777ff, from = text_muted)]
    pub placeholder: Rgba,
    /// Browse button background.
    #[theme(default = 0x007accff, from = accent)]
    pub browse_bg: Rgba,
    /// Browse button label color.
    #[theme(default = 0xffffffff, from = text_on_accent)]
    pub browse_text: Rgba,
}

/// File input size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileInputSize {
    /// Compact row.
    Sm,
    /// Standard row (default).
    #[default]
    Md,
    /// Large row.
    Lg,
}

impl From<crate::ComponentSize> for FileInputSize {
    fn from(size: crate::ComponentSize) -> Self {
        match size {
            crate::ComponentSize::Xs | crate::ComponentSize::Sm => Self::Sm,
            crate::ComponentSize::Md => Self::Md,
            crate::ComponentSize::Lg | crate::ComponentSize::Xl => Self::Lg,
        }
    }
}

/// File picker trigger showing the chosen file name.
///
/// The component never opens a dialog itself; `on_browse` notifies the
/// parent so it can present the platform picker and store the result.
#[derive(IntoElement)]
pub struct FileInput {
    id: ElementId,
    file_name: Option<SharedString>,
    placeholder: SharedString,
    accept: Option<SharedString>,
    size: FileInputSize,
    disabled: bool,
    on_browse: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    theme: Option<FileInputTheme>,
    design: Option<Arc<DesignSystem>>,
    aria_label: Option<SharedString>,
    aria_role: Option<AriaRole>,
}

impl FileInput {
    /// Create an empty file input row.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let input = FileInput::new("avatar-upload");
    /// ```
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            file_name: None,
            placeholder: SharedString::from("No file chosen"),
            accept: None,
            size: FileInputSize::default(),
            disabled: false,
            on_browse: None,
            theme: None,
            design: None,
            aria_label: None,
            aria_role: None,
        }
    }

    /// Set the chosen file name display.
    pub fn file_name(mut self, file_name: impl Into<SharedString>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    /// Set the placeholder shown when no file is chosen.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the accepted file filter hint (for example ".png,.jpg").
    pub fn accept(mut self, accept: impl Into<SharedString>) -> Self {
        self.accept = Some(accept.into());
        self
    }

    /// Set the row size.
    pub fn size(mut self, size: FileInputSize) -> Self {
        self.size = size;
        self
    }

    /// Disable the row.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the browse activation handler.
    pub fn on_browse(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_browse = Some(Rc::new(handler));
        self
    }

    /// Set custom theme colors.
    pub fn theme(mut self, theme: FileInputTheme) -> Self {
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

    /// Override the default ARIA role (Button).
    pub fn aria_role(mut self, role: AriaRole) -> Self {
        self.aria_role = Some(role);
        self
    }

    /// Text shown in the file name slot.
    fn display_text(&self) -> SharedString {
        self.file_name
            .clone()
            .unwrap_or_else(|| self.placeholder.clone())
    }

    /// Accessible name for the row.
    fn accessible_label(&self) -> SharedString {
        self.aria_label
            .clone()
            .unwrap_or_else(|| self.display_text())
    }

    /// Build the row with theme.
    pub fn build_with_theme(self, theme: &FileInputTheme) -> Stateful<Div> {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_theme_and_design(theme, &design)
    }

    /// Build the row with theme and design tokens.
    pub fn build_with_theme_and_design(
        self,
        theme: &FileInputTheme,
        design: &DesignSystem,
    ) -> Stateful<Div> {
        let native_label = self.accessible_label();
        let native_props = AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Button))
            .maybe_state(self.disabled, AriaState::Disabled);
        let display = self.display_text();
        let has_file = self.file_name.is_some();

        let mut row = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(design.spacing.control_gap))
            .px(px(design.spacing.control_padding_x))
            .py(px(design.spacing.control_padding_y * 0.5))
            .rounded(px(design.corners.md))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background);
        row = match self.size {
            FileInputSize::Sm => row.text_xs(),
            FileInputSize::Md => row.text_sm(),
            FileInputSize::Lg => row.text_lg(),
        };

        let mut name = if has_file {
            div().text_color(theme.text)
        } else {
            div().text_color(theme.placeholder)
        };
        name = name.child(display);
        if let Some(accept) = self.accept {
            name = name.child(
                div()
                    .text_color(theme.placeholder)
                    .child(SharedString::from(format!(" ({accept})"))),
            );
        }
        let browse = div()
            .px(px(design.spacing.control_padding_x * 0.75))
            .py(px(design.spacing.control_padding_y * 0.25))
            .rounded(px(design.corners.sm))
            .bg(theme.browse_bg)
            .text_color(theme.browse_text)
            .child("Browse");
        row = row.child(name).child(browse);

        if self.disabled {
            row = row.opacity(0.5).cursor_not_allowed();
        } else {
            row = row.cursor_pointer();
            if let Some(handler) = self.on_browse {
                let mouse_handler = handler.clone();
                row = row.on_mouse_up(MouseButton::Left, move |_event, window, cx| {
                    mouse_handler(window, cx);
                });
                row = row.on_key_down(move |event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
                        handler(window, cx);
                        cx.stop_propagation();
                    }
                });
            }
        }

        apply_native_accessibility(row, native_label, &native_props)
    }
}

impl RenderOnce for FileInput {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        cx.register_accessible(AccessibilityNode {
            element_id: self.id.clone(),
            label: self.accessible_label(),
            props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Button))
                .maybe_state(self.disabled, AriaState::Disabled),
        });

        let global_theme = cx.theme();
        let theme = self
            .theme
            .clone()
            .unwrap_or_else(|| FileInputTheme::from(global_theme));
        let design = crate::design::resolve_design(self.design.clone(), cx);
        self.build_with_theme_and_design(&theme, &design)
    }
}

#[cfg(test)]
mod tests {
    use super::{FileInput, FileInputSize, FileInputTheme};
    use gpui::rgba;

    #[test]
    fn file_input_defaults_to_empty_enabled_row() {
        let input = FileInput::new("upload");
        assert!(input.file_name.is_none());
        assert!(input.accept.is_none());
        assert_eq!(input.size, FileInputSize::Md);
        assert!(!input.disabled);
        assert_eq!(input.placeholder.as_ref(), "No file chosen");
    }

    #[test]
    fn file_input_builders_store_name_filter_and_size() {
        let input = FileInput::new("upload")
            .file_name("portrait.png")
            .accept(".png,.jpg")
            .size(FileInputSize::Lg)
            .disabled(true);
        assert_eq!(input.file_name.as_deref(), Some("portrait.png"));
        assert_eq!(input.accept.as_deref(), Some(".png,.jpg"));
        assert_eq!(input.size, FileInputSize::Lg);
        assert!(input.disabled);
    }

    #[test]
    fn file_input_display_text_prefers_file_name() {
        let empty = FileInput::new("upload");
        assert_eq!(empty.display_text().as_ref(), "No file chosen");
        let named = FileInput::new("upload").file_name("mix.wav");
        assert_eq!(named.display_text().as_ref(), "mix.wav");
    }

    #[test]
    fn file_input_theme_defaults_match_compiled_tokens() {
        let theme = FileInputTheme::default();
        assert_eq!(theme.background, rgba(0x1e1e1eff));
        assert_eq!(theme.border, rgba(0x3a3a3aff));
        assert_eq!(theme.text, rgba(0xccccccff));
        assert_eq!(theme.placeholder, rgba(0x777777ff));
        assert_eq!(theme.browse_bg, rgba(0x007accff));
        assert_eq!(theme.browse_text, rgba(0xffffffff));
    }
}
