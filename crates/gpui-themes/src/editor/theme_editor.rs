use super::color_field::ColorField;
use super::color_field::all_color_fields;
use super::misc::TRANSPARENT;
use super::misc::editor_header_presets;
use super::types::EditorTab;
use crate::showcase::ComponentShowcase;
use crate::theme::{BuiltInThemePreset, Color, ColorGroup, EditorTheme, slugify_theme_name};
use gpui::prelude::StatefulInteractiveElement;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::{
    Button, ButtonSize, ButtonVariant, ColorPickerView, HStack, StackSpacing, Text, TextSize,
    TextWeight, VStack,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

#[derive(Clone)]
struct ColorDetailText {
    hex: SharedString,
    rgba: SharedString,
    hsl: SharedString,
}

impl ColorDetailText {
    fn from_color(color: Color) -> Self {
        let (h, s, l) = color.to_hsl();
        Self {
            hex: SharedString::from(color.to_hex_string()),
            rgba: SharedString::from(format!(
                "RGBA: {}, {}, {}, {}",
                color.r, color.g, color.b, color.a
            )),
            hsl: SharedString::from(format!(
                "HSL: {:.0}°, {:.0}%, {:.0}%",
                h * 360.0,
                s * 100.0,
                l * 100.0
            )),
        }
    }
}

/// Theme editor state
pub struct ThemeEditor {
    /// Current theme being edited
    pub theme: Arc<EditorTheme>,
    /// Currently selected color group
    pub selected_group: ColorGroup,
    /// Currently selected color field index within group
    pub selected_field_index: usize,
    /// Current tab
    pub current_tab: EditorTab,
    /// All color fields
    pub color_fields: &'static [ColorField],
    /// Expanded accordion sections
    pub expanded_sections: Vec<SharedString>,
    /// Color picker model for modal
    pub color_picker: Option<Entity<ColorPickerView>>,
    /// Component showcase model
    pub showcase: Entity<ComponentShowcase>,
    /// Export format (json or rust)
    pub export_format: String,
    /// Show color picker modal
    pub show_color_modal: bool,
    /// Field being edited in modal
    pub editing_field: Option<ColorField>,
    /// Cached export format used to generate `cached_export_content`
    cached_export_format: String,
    /// Cached serialized export content for the export tab
    cached_export_content: SharedString,
    /// Cached export filename derived from theme name and format
    cached_export_filename: SharedString,
    /// Last file-export outcome shown in the export tab.
    export_status: Option<SharedString>,
    /// Theme Arc that `cached_export_content` corresponds to
    /// Set by edits; export content is regenerated only when it is viewed or used.
    export_cache_dirty: bool,
    /// Cached hex strings for color fields, keyed by field name
    hex_cache: HashMap<&'static str, SharedString>,
    color_detail_cache: HashMap<&'static str, ColorDetailText>,
}

/// Stable element ID for a color group row.
fn group_element_id(group: ColorGroup) -> SharedString {
    static IDS: OnceLock<HashMap<ColorGroup, SharedString>> = OnceLock::new();
    IDS.get_or_init(|| {
        ColorGroup::all()
            .iter()
            .map(|&g| (g, SharedString::from(format!("group-{g:?}"))))
            .collect()
    })
    .get(&group)
    .cloned()
    .unwrap_or_else(|| SharedString::from(format!("group-{group:?}")))
}

/// Stable element ID for a color field row.
fn field_element_id(field: &ColorField) -> SharedString {
    static IDS: OnceLock<HashMap<&'static str, SharedString>> = OnceLock::new();
    IDS.get_or_init(|| {
        all_color_fields()
            .iter()
            .map(|f| (f.name, SharedString::from(format!("field-{}", f.name))))
            .collect()
    })
    .get(field.name)
    .cloned()
    .unwrap_or_else(|| SharedString::from(format!("field-{}", field.name)))
}

/// Stable element ID for an editor tab.
fn tab_element_id(tab: EditorTab) -> SharedString {
    static IDS: OnceLock<HashMap<EditorTab, SharedString>> = OnceLock::new();
    IDS.get_or_init(|| {
        [EditorTab::Colors, EditorTab::Preview, EditorTab::Export]
            .iter()
            .map(|&t| (t, SharedString::from(format!("tab-{t:?}"))))
            .collect()
    })
    .get(&tab)
    .cloned()
    .unwrap_or_else(|| SharedString::from(format!("tab-{tab:?}")))
}

/// Stable element ID for a preset button in the header.
fn preset_element_id(preset_id: &str) -> SharedString {
    static IDS: OnceLock<HashMap<&'static str, SharedString>> = OnceLock::new();
    IDS.get_or_init(|| {
        editor_header_presets()
            .iter()
            .map(|preset| {
                let id = preset.id();
                (id, SharedString::from(format!("preset-{id}")))
            })
            .collect()
    })
    .get(preset_id)
    .cloned()
    .unwrap_or_else(|| SharedString::from(format!("preset-{preset_id}")))
}

/// Cached "Edit: {field}" label for the color editor panel.
fn field_edit_label(field: &ColorField) -> SharedString {
    static LABELS: OnceLock<HashMap<&'static str, SharedString>> = OnceLock::new();
    LABELS
        .get_or_init(|| {
            all_color_fields()
                .iter()
                .map(|f| (f.name, SharedString::from(format!("Edit: {}", f.name))))
                .collect()
        })
        .get(field.name)
        .cloned()
        .unwrap_or_else(|| SharedString::from(format!("Edit: {}", field.name)))
}

impl ThemeEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let theme = Arc::new(EditorTheme::dark());
        let showcase = cx.new(|_| ComponentShowcase::new(theme.clone()));

        let hex_cache = Self::build_hex_cache(&theme);
        let color_detail_cache = Self::build_color_detail_cache(&theme);
        let cached_export_content =
            SharedString::from(theme.to_json().unwrap_or_else(|e| format!("Error: {}", e)));
        let cached_export_filename =
            SharedString::from(format!("{}_theme.json", slugify_theme_name(&theme.name)));

        Self {
            theme: theme.clone(),
            selected_group: ColorGroup::Base,
            selected_field_index: 0,
            current_tab: EditorTab::Colors,
            color_fields: all_color_fields(),
            expanded_sections: vec![SharedString::from("Base Colors")],
            color_picker: None,
            showcase,
            export_format: "json".to_string(),
            show_color_modal: false,
            editing_field: None,
            cached_export_format: "json".to_string(),
            cached_export_content,
            cached_export_filename,
            export_status: None,
            export_cache_dirty: false,
            hex_cache,
            color_detail_cache,
        }
    }

    /// Get fields for a specific group
    pub(super) fn fields_for_group(group: ColorGroup) -> &'static [ColorField] {
        static GROUPS: OnceLock<HashMap<ColorGroup, &'static [ColorField]>> = OnceLock::new();
        GROUPS
            .get_or_init(|| {
                let mut map: HashMap<ColorGroup, Vec<ColorField>> = HashMap::new();
                for field in all_color_fields() {
                    map.entry(field.group).or_default().push(*field);
                }
                map.into_iter()
                    .map(|(group, fields)| {
                        let leaked: &'static [ColorField] = Box::leak(fields.into_boxed_slice());
                        (group, leaked)
                    })
                    .collect()
            })
            .get(&group)
            .copied()
            .unwrap_or(&[])
    }

    /// Get current selected field
    pub(super) fn current_field(&self) -> Option<&'static ColorField> {
        Self::fields_for_group(self.selected_group).get(self.selected_field_index)
    }

    /// Build a fresh hex string cache from the given theme.
    fn build_hex_cache(theme: &Arc<EditorTheme>) -> HashMap<&'static str, SharedString> {
        let mut cache = HashMap::new();
        for &field in all_color_fields() {
            let color = (field.getter)(theme);
            cache.insert(field.name, SharedString::from(color.to_hex_string()));
        }
        cache
    }

    fn build_color_detail_cache(
        theme: &Arc<EditorTheme>,
    ) -> HashMap<&'static str, ColorDetailText> {
        all_color_fields()
            .iter()
            .map(|field| {
                (
                    field.name,
                    ColorDetailText::from_color((field.getter)(theme)),
                )
            })
            .collect()
    }

    /// Refresh the export tab cache if the format or theme Arc has changed.
    fn refresh_export_cache(&mut self) {
        if self.current_tab != EditorTab::Export {
            return;
        }
        if self.export_cache_dirty || self.cached_export_format != self.export_format {
            self.cached_export_format.clone_from(&self.export_format);
            let content = if self.export_format == "json" {
                self.theme
                    .to_json()
                    .unwrap_or_else(|e| format!("Error: {}", e))
            } else {
                self.theme.to_rust_code()
            };
            self.cached_export_content = SharedString::from(content);
            let filename = format!(
                "{}_theme.{}",
                slugify_theme_name(&self.theme.name),
                self.export_format
            );
            self.cached_export_filename = SharedString::from(filename);
            self.export_cache_dirty = false;
        }
    }

    /// Update a color and sync to showcase
    pub(super) fn update_color(
        &mut self,
        field: &ColorField,
        color: Color,
        cx: &mut Context<Self>,
    ) {
        let mut theme = (*self.theme).clone();
        (field.setter)(&mut theme, color);
        self.theme = Arc::new(theme);
        // Refresh cached hex for the changed field
        self.export_cache_dirty = true;
        let updated_color = (field.getter)(&self.theme);
        self.hex_cache.insert(
            field.name,
            SharedString::from(updated_color.to_hex_string()),
        );
        self.color_detail_cache
            .insert(field.name, ColorDetailText::from_color(updated_color));
        // Export content depends on the theme, so refresh it
        self.refresh_export_cache();
        // Update showcase with a cheap Arc clone
        let new_theme = self.theme.clone();
        self.showcase.update(cx, |showcase, cx| {
            showcase.set_theme(new_theme);
            cx.notify();
        });
        cx.notify();
    }

    /// Load a preset theme
    pub(super) fn load_preset(&mut self, preset: &str, cx: &mut Context<Self>) {
        self.theme = Arc::new(
            BuiltInThemePreset::from_id(preset)
                .unwrap_or_default()
                .to_theme(),
        );
        // Rebuild the hex cache for the new preset theme
        self.hex_cache = Self::build_hex_cache(&self.theme);
        self.color_detail_cache = Self::build_color_detail_cache(&self.theme);
        self.export_cache_dirty = true;
        // Export content depends on the theme, so refresh it
        self.refresh_export_cache();
        self.showcase.update(cx, |showcase, cx| {
            showcase.set_theme(self.theme.clone());
            cx.notify();
        });
        cx.notify();
    }

    /// Open color picker modal for current field
    pub(super) fn open_color_modal(&mut self, cx: &mut Context<Self>) {
        // Clone field info before mutating self
        let field_info = self.current_field().map(|field| {
            let color = (field.getter)(&self.theme);
            (color, field.name, field.group, field.getter, field.setter)
        });

        if let Some((color, field_name, group, getter, setter)) = field_info {
            // Create color picker entity
            self.color_picker = Some(cx.new(|_| ColorPickerView::new(field_name, color)));
            self.editing_field = Some(ColorField {
                group,
                name: field_name,
                getter,
                setter,
            });
            self.show_color_modal = true;
            cx.notify();
        }
    }

    /// Apply color from modal
    pub(super) fn apply_color_from_modal(&mut self, cx: &mut Context<Self>) {
        if let (Some(picker), Some(field)) = (&self.color_picker, &self.editing_field) {
            let color = picker.read(cx).color();
            let field_clone = ColorField {
                group: field.group,
                name: field.name,
                getter: field.getter,
                setter: field.setter,
            };
            self.update_color(&field_clone, color, cx);
        }
        self.close_color_modal(cx);
    }

    /// Close color picker modal
    pub(super) fn close_color_modal(&mut self, cx: &mut Context<Self>) {
        self.show_color_modal = false;
        self.color_picker = None;
        self.editing_field = None;
        cx.notify();
    }

    /// Render the sidebar with color groups
    pub(super) fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;
        let selected_group = self.selected_group;

        VStack::new()
            .spacing(StackSpacing::None)
            .child(
                div()
                    .p_3()
                    .border_b_1()
                    .border_color(theme.border.to_rgba())
                    .child(
                        Text::new("Color Groups")
                            .size(TextSize::Sm)
                            .weight(TextWeight::Bold)
                            .color(theme.text_secondary.to_rgba()),
                    ),
            )
            .children(ColorGroup::all().iter().map(|group| {
                let is_selected = *group == selected_group;
                let bg = if is_selected {
                    theme.surface_selected.to_rgba()
                } else {
                    TRANSPARENT
                };
                let text_color = if is_selected {
                    theme.text_primary.to_rgba()
                } else {
                    theme.text_secondary.to_rgba()
                };

                div()
                    .id(group_element_id(*group))
                    .cursor_pointer()
                    .px_3()
                    .py_2()
                    .bg(bg)
                    .hover(|s| s.bg(theme.surface_hover.to_rgba()))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener({
                            let group = *group;
                            move |this, _: &MouseUpEvent, _window, cx| {
                                this.selected_group = group;
                                this.selected_field_index = 0;
                                cx.notify();
                            }
                        }),
                    )
                    .child(
                        Text::new(group.label())
                            .size(TextSize::Sm)
                            .color(text_color),
                    )
            }))
            .build()
    }

    /// Render color list for current group
    pub(super) fn render_color_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;
        let fields = Self::fields_for_group(self.selected_group);
        let selected_index = self.selected_field_index;

        VStack::new()
            .spacing(StackSpacing::None)
            .child(
                div()
                    .p_3()
                    .border_b_1()
                    .border_color(theme.border.to_rgba())
                    .child(
                        Text::new(self.selected_group.label())
                            .size(TextSize::Md)
                            .weight(TextWeight::Bold)
                            .color(theme.text_primary.to_rgba()),
                    ),
            )
            .children(fields.iter().enumerate().map(|(idx, field)| {
                let color = (field.getter)(&self.theme);
                let is_selected = idx == selected_index;
                let bg = if is_selected {
                    theme.surface_selected.to_rgba()
                } else {
                    TRANSPARENT
                };

                div()
                    .id(field_element_id(field))
                    .cursor_pointer()
                    .px_3()
                    .py_2()
                    .bg(bg)
                    .hover(|s| s.bg(theme.surface_hover.to_rgba()))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener({
                            move |this, _: &MouseUpEvent, _window, cx| {
                                this.selected_field_index = idx;
                                cx.notify();
                            }
                        }),
                    )
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(
                                div()
                                    .w(px(24.0))
                                    .h(px(24.0))
                                    .rounded(px(4.0))
                                    .bg(color.to_rgba())
                                    .border_1()
                                    .border_color(theme.border.to_rgba()),
                            )
                            .child(
                                Text::new(field.name)
                                    .size(TextSize::Sm)
                                    .color(theme.text_primary.to_rgba()),
                            )
                            .child(div().flex_1())
                            .child(
                                Text::new(
                                    self.hex_cache.get(field.name).cloned().unwrap_or_else(|| {
                                        SharedString::from(color.to_hex_string())
                                    }),
                                )
                                .size(TextSize::Xs)
                                .color(theme.text_muted.to_rgba()),
                            )
                            .build(),
                    )
            }))
            .build()
    }

    /// Render color editor panel
    pub(super) fn render_color_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;

        if let Some(field) = self.current_field() {
            let color = (field.getter)(&self.theme);

            let detail = self
                .color_detail_cache
                .get(field.name)
                .cloned()
                .unwrap_or_else(|| ColorDetailText::from_color(color));

            div().p_4().child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        Text::new(field_edit_label(field))
                            .size(TextSize::Md)
                            .weight(TextWeight::Bold)
                            .color(theme.text_primary.to_rgba()),
                    )
                    // Large color preview (clickable)
                    .child(
                        div()
                            .id("color-preview")
                            .w_full()
                            .h(px(80.0))
                            .rounded_lg()
                            .bg(color.to_rgba())
                            .border_1()
                            .border_color(theme.border.to_rgba())
                            .cursor_pointer()
                            .hover(|s| s.border_color(theme.accent.to_rgba()))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                                    this.open_color_modal(cx);
                                }),
                            ),
                    )
                    // Hex display
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(
                                Text::new("Hex:")
                                    .size(TextSize::Sm)
                                    .color(theme.text_secondary.to_rgba()),
                            )
                            .child(
                                Text::new(detail.hex.clone())
                                    .size(TextSize::Md)
                                    .weight(TextWeight::Medium)
                                    .color(theme.text_primary.to_rgba()),
                            )
                            .build(),
                    )
                    // RGBA display
                    .child(
                        Text::new(detail.rgba.clone())
                            .size(TextSize::Sm)
                            .color(theme.text_muted.to_rgba()),
                    )
                    // HSL display
                    .child(
                        Text::new(detail.hsl.clone())
                            .size(TextSize::Sm)
                            .color(theme.text_muted.to_rgba()),
                    )
                    // Edit button
                    .child(
                        Button::new("edit-color-btn", "Edit Color")
                            .variant(ButtonVariant::Primary)
                            .size(ButtonSize::Md)
                            .build()
                            .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                this.open_color_modal(cx);
                            })),
                    )
                    .build(),
            )
        } else {
            div().p_4().child(
                Text::new("Select a color to edit")
                    .size(TextSize::Md)
                    .color(theme.text_muted.to_rgba()),
            )
        }
    }

    /// Render the colors tab
    pub(super) fn render_colors_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;

        div()
            .flex()
            .flex_row()
            .size_full()
            // Sidebar
            .child(
                div()
                    .w(px(180.0))
                    .h_full()
                    .bg(theme.background_secondary.to_rgba())
                    .border_r_1()
                    .border_color(theme.border.to_rgba())
                    .child(self.render_sidebar(cx)),
            )
            // Color list
            .child(
                div()
                    .w(px(280.0))
                    .h_full()
                    .bg(theme.background.to_rgba())
                    .border_r_1()
                    .border_color(theme.border.to_rgba())
                    .child(self.render_color_list(cx)),
            )
            // Color editor
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .bg(theme.background_secondary.to_rgba())
                    .child(self.render_color_editor(cx)),
            )
    }

    /// Render the preview tab
    pub(super) fn render_preview_tab(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.showcase.clone())
    }

    /// Render the export tab
    pub(super) fn render_export_tab(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        self.refresh_export_cache();

        let theme = &self.theme;
        let export_format = self.export_format.clone();
        let export_content = self.cached_export_content.clone();
        let export_status = self.export_status.clone();

        div().p_6().size_full().child(
            VStack::new()
                .spacing(StackSpacing::Lg)
                // Theme name display
                .child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(
                            Text::new("Theme Name:")
                                .size(TextSize::Md)
                                .weight(TextWeight::Bold)
                                .color(theme.text_primary.to_rgba()),
                        )
                        .child(
                            Text::new(SharedString::from(self.theme.name.clone()))
                                .size(TextSize::Md)
                                .color(theme.text_primary.to_rgba()),
                        )
                        .build(),
                )
                // Format selection
                .child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(
                            Text::new("Export Format:")
                                .size(TextSize::Md)
                                .color(theme.text_primary.to_rgba()),
                        )
                        .child(
                            Button::new("format-json", "JSON")
                                .variant(if export_format == "json" {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Secondary
                                })
                                .size(ButtonSize::Sm)
                                .build()
                                .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                    this.export_format = "json".to_string();
                                    this.refresh_export_cache();
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("format-rust", "Rust")
                                .variant(if export_format == "rust" {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Secondary
                                })
                                .size(ButtonSize::Sm)
                                .build()
                                .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                    this.export_format = "rust".to_string();
                                    this.refresh_export_cache();
                                    cx.notify();
                                })),
                        )
                        .build(),
                )
                // Export preview
                .child(
                    div()
                        .id("theme-export-preview")
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .overflow_y_scroll()
                        .p_4()
                        .bg(theme.background_tertiary.to_rgba())
                        .rounded_lg()
                        .border_1()
                        .border_color(theme.border.to_rgba())
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.text_primary.to_rgba())
                                .child(export_content),
                        ),
                )
                // Action buttons
                .child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(
                            Button::new("copy-btn", "Copy to Clipboard")
                                .variant(ButtonVariant::Primary)
                                .size(ButtonSize::Md)
                                .build()
                                .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                    this.refresh_export_cache();
                                    let content = this.cached_export_content.to_string();
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(content));
                                })),
                        )
                        .child(
                            Button::new("save-btn", "Save to File")
                                .variant(ButtonVariant::Secondary)
                                .size(ButtonSize::Md)
                                .build()
                                .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                    this.refresh_export_cache();
                                    let content = this.cached_export_content.to_string();
                                    let filename = this.cached_export_filename.to_string();
                                    if let Err(error) = std::fs::write(&filename, content) {
                                        this.export_status = Some(SharedString::from(format!(
                                            "Could not save {filename}: {error}"
                                        )));
                                    } else {
                                        this.export_status =
                                            Some(SharedString::from(format!("Saved {filename}")));
                                    }
                                    cx.notify();
                                })),
                        )
                        .when_some(export_status, |row, status| {
                            row.child(
                                Text::new(status)
                                    .size(TextSize::Sm)
                                    .color(theme.text_secondary.to_rgba()),
                            )
                        })
                        .build(),
                )
                .build(),
        )
    }

    /// Render the header with presets and tabs
    pub(super) fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;
        let current_tab = self.current_tab;

        VStack::new()
            .spacing(StackSpacing::None)
            // Top bar with presets
            .child(
                div()
                    .px_4()
                    .py_2()
                    .bg(theme.background_secondary.to_rgba())
                    .border_b_1()
                    .border_color(theme.border.to_rgba())
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(
                                Text::new("Theme Editor")
                                    .size(TextSize::Lg)
                                    .weight(TextWeight::Bold)
                                    .color(theme.text_primary.to_rgba()),
                            )
                            .child(div().flex_1())
                            .child(
                                Text::new("Load Preset:")
                                    .size(TextSize::Sm)
                                    .color(theme.text_secondary.to_rgba()),
                            )
                            .children(editor_header_presets().iter().map(|preset| {
                                let preset = *preset;
                                Button::new(preset_element_id(preset.id()), preset.name())
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm)
                                    .build()
                                    .on_click(cx.listener(
                                        move |this, _: &ClickEvent, _window, cx| {
                                            this.load_preset(preset.id(), cx);
                                        },
                                    ))
                            }))
                            .build(),
                    ),
            )
            // Tab bar
            .child(
                div()
                    .px_4()
                    .py_1()
                    .bg(theme.surface.to_rgba())
                    .border_b_1()
                    .border_color(theme.border.to_rgba())
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::None)
                            .child(self.render_tab_button(
                                "Colors",
                                EditorTab::Colors,
                                current_tab,
                                cx,
                            ))
                            .child(self.render_tab_button(
                                "Preview",
                                EditorTab::Preview,
                                current_tab,
                                cx,
                            ))
                            .child(self.render_tab_button(
                                "Export",
                                EditorTab::Export,
                                current_tab,
                                cx,
                            ))
                            .build(),
                    ),
            )
            .build()
    }

    pub(super) fn render_tab_button(
        &self,
        label: &'static str,
        tab: EditorTab,
        current: EditorTab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = &self.theme;
        let is_selected = tab == current;
        let bg = if is_selected {
            theme.surface_selected.to_rgba()
        } else {
            TRANSPARENT
        };
        let text_color = if is_selected {
            theme.text_primary.to_rgba()
        } else {
            theme.text_secondary.to_rgba()
        };

        div()
            .id(tab_element_id(tab))
            .cursor_pointer()
            .px_4()
            .py_2()
            .bg(bg)
            .hover(|s| s.bg(theme.surface_hover.to_rgba()))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseUpEvent, _window, cx| {
                    this.current_tab = tab;
                    cx.notify();
                }),
            )
            .child(Text::new(label).size(TextSize::Sm).color(text_color))
    }

    /// Render the color picker modal
    pub(super) fn render_color_modal(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;

        if !self.show_color_modal {
            return div().into_any_element();
        }

        let picker_content = if let Some(picker) = &self.color_picker {
            div().child(picker.clone()).into_any_element()
        } else {
            div().into_any_element()
        };

        // Build the dialog content manually since we need entity interaction
        // The Dialog component expects global handlers, but we need entity context
        let backdrop_color = Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.6,
        };

        // Backdrop clicks intentionally do nothing: an accidental click must
        // not discard an in-progress color selection. Cancel and Escape are
        // explicit close actions.
        div()
            .id("modal-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(backdrop_color)
            .child(
                div()
                    .id("color-modal-dialog")
                    .w(px(500.0))
                    .max_h(px(700.0))
                    .bg(theme.surface.to_rgba())
                    .border_1()
                    .border_color(theme.accent.to_rgba())
                    .rounded_lg()
                    .shadow_lg()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    // Stop propagation so clicks don't reach the backdrop
                    .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _window, cx| {
                        cx.stop_propagation();
                    })
                    // Header
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(theme.border.to_rgba())
                            .child(
                                Text::new("Edit Color")
                                    .size(TextSize::Lg)
                                    .weight(TextWeight::Bold)
                                    .color(theme.text_primary.to_rgba()),
                            )
                            .child(
                                div()
                                    .id("close-modal-btn")
                                    .px_2()
                                    .py_1()
                                    .rounded(px(3.0))
                                    .cursor_pointer()
                                    .text_color(theme.text_muted.to_rgba())
                                    .hover(|s| {
                                        s.bg(theme.surface_hover.to_rgba())
                                            .text_color(theme.text_primary.to_rgba())
                                    })
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(|this, _: &MouseUpEvent, _window, cx| {
                                            this.close_color_modal(cx);
                                        }),
                                    )
                                    .child("×"),
                            ),
                    )
                    // Content
                    .child(div().flex_1().p_4().child(picker_content))
                    // Footer
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_t_1()
                            .border_color(theme.border.to_rgba())
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Md)
                                    .child(div().flex_1())
                                    .child(
                                        Button::new("cancel-btn", "Cancel")
                                            .variant(ButtonVariant::Ghost)
                                            .size(ButtonSize::Md)
                                            .build()
                                            .on_click(cx.listener(
                                                |this, _: &ClickEvent, _window, cx| {
                                                    this.close_color_modal(cx);
                                                },
                                            )),
                                    )
                                    .child(
                                        Button::new("apply-btn", "Apply")
                                            .variant(ButtonVariant::Primary)
                                            .size(ButtonSize::Md)
                                            .build()
                                            .on_click(cx.listener(
                                                |this, _: &ClickEvent, _window, cx| {
                                                    this.apply_color_from_modal(cx);
                                                },
                                            )),
                                    )
                                    .build(),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl Render for ThemeEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;
        let current_tab = self.current_tab;

        div()
            .size_full()
            .bg(theme.background.to_rgba())
            .flex()
            .flex_col()
            .relative()
            // Header
            .child(self.render_header(cx))
            // Content based on tab
            .child(div().flex_1().min_h_0().child(match current_tab {
                EditorTab::Colors => self.render_colors_tab(cx).into_any_element(),
                EditorTab::Preview => self.render_preview_tab(cx).into_any_element(),
                EditorTab::Export => self.render_export_tab(cx).into_any_element(),
            }))
            // Color picker modal (rendered on top when visible)
            .child(self.render_color_modal(cx))
    }
}

#[cfg(test)]
mod theme_editor_tests {
    use super::ColorGroup;
    use super::EditorTab;
    use super::ThemeEditor;
    use super::all_color_fields;
    use super::field_edit_label;
    use super::field_element_id;
    use super::group_element_id;
    use super::tab_element_id;

    #[test]
    fn test_fields_for_group_filters_and_is_stable() {
        let base = ThemeEditor::fields_for_group(ColorGroup::Base);
        assert!(!base.is_empty());
        for field in base {
            assert_eq!(field.group, ColorGroup::Base);
        }

        // Repeated calls return the same static slice.
        let base2 = ThemeEditor::fields_for_group(ColorGroup::Base);
        assert_eq!(base.as_ptr(), base2.as_ptr());
    }

    #[test]
    fn test_element_ids_are_stable() {
        let fields = all_color_fields();
        let first = fields.first().unwrap();
        assert_eq!(
            field_element_id(first).as_ref(),
            format!("field-{}", first.name)
        );
        assert_eq!(group_element_id(ColorGroup::Base).as_ref(), "group-Base");
        assert_eq!(tab_element_id(EditorTab::Colors).as_ref(), "tab-Colors");
    }

    #[test]
    fn test_field_edit_label_is_cached() {
        let field = all_color_fields().first().copied().unwrap();
        let label1 = field_edit_label(&field);
        let label2 = field_edit_label(&field);
        assert_eq!(label1.as_ref(), format!("Edit: {}", field.name));
        assert_eq!(label1, label2);
    }
}
