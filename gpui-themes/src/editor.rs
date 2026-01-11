//! Theme editor main component
//!
//! Provides the main theme editor UI with:
//! - Color group navigation
//! - Color editing with live preview via modal
//! - Export to JSON and Rust

use crate::showcase::ComponentShowcase;
use crate::theme::{Color, ColorGroup, EditorTheme};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::{
    Button, ButtonSize, ButtonVariant, ColorPickerView, HStack, StackSpacing, Text, TextSize,
    TextWeight, VStack,
};

/// Transparent color constant
const TRANSPARENT: Rgba = Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

/// Currently selected color field
#[derive(Debug, Clone)]
pub struct ColorField {
    pub group: ColorGroup,
    pub name: &'static str,
    pub getter: fn(&EditorTheme) -> Color,
    pub setter: fn(&mut EditorTheme, Color),
}

impl ColorField {
    pub const fn new(
        group: ColorGroup,
        name: &'static str,
        getter: fn(&EditorTheme) -> Color,
        setter: fn(&mut EditorTheme, Color),
    ) -> Self {
        Self {
            group,
            name,
            getter,
            setter,
        }
    }
}

/// All editable color fields
pub fn all_color_fields() -> Vec<ColorField> {
    vec![
        // Base colors
        ColorField::new(
            ColorGroup::Base,
            "Background",
            |t| t.background,
            |t, c| t.background = c,
        ),
        ColorField::new(
            ColorGroup::Base,
            "Background Secondary",
            |t| t.background_secondary,
            |t, c| t.background_secondary = c,
        ),
        ColorField::new(
            ColorGroup::Base,
            "Background Tertiary",
            |t| t.background_tertiary,
            |t, c| t.background_tertiary = c,
        ),
        ColorField::new(
            ColorGroup::Base,
            "Surface",
            |t| t.surface,
            |t, c| t.surface = c,
        ),
        ColorField::new(
            ColorGroup::Base,
            "Surface Hover",
            |t| t.surface_hover,
            |t, c| t.surface_hover = c,
        ),
        ColorField::new(
            ColorGroup::Base,
            "Surface Selected",
            |t| t.surface_selected,
            |t, c| t.surface_selected = c,
        ),
        // Text colors
        ColorField::new(
            ColorGroup::Text,
            "Text Primary",
            |t| t.text_primary,
            |t, c| t.text_primary = c,
        ),
        ColorField::new(
            ColorGroup::Text,
            "Text Secondary",
            |t| t.text_secondary,
            |t, c| t.text_secondary = c,
        ),
        ColorField::new(
            ColorGroup::Text,
            "Text Muted",
            |t| t.text_muted,
            |t, c| t.text_muted = c,
        ),
        ColorField::new(
            ColorGroup::Text,
            "Text Disabled",
            |t| t.text_disabled,
            |t, c| t.text_disabled = c,
        ),
        ColorField::new(
            ColorGroup::Text,
            "Text on Accent",
            |t| t.text_on_accent,
            |t, c| t.text_on_accent = c,
        ),
        ColorField::new(
            ColorGroup::Text,
            "Text on Accent Muted",
            |t| t.text_on_accent_muted,
            |t, c| t.text_on_accent_muted = c,
        ),
        // Border colors
        ColorField::new(
            ColorGroup::Border,
            "Border",
            |t| t.border,
            |t, c| t.border = c,
        ),
        ColorField::new(
            ColorGroup::Border,
            "Border Focused",
            |t| t.border_focused,
            |t, c| t.border_focused = c,
        ),
        // Accent colors
        ColorField::new(
            ColorGroup::Accent,
            "Accent",
            |t| t.accent,
            |t, c| t.accent = c,
        ),
        ColorField::new(
            ColorGroup::Accent,
            "Accent Hover",
            |t| t.accent_hover,
            |t, c| t.accent_hover = c,
        ),
        ColorField::new(
            ColorGroup::Accent,
            "Accent Muted",
            |t| t.accent_muted,
            |t, c| t.accent_muted = c,
        ),
        // Semantic colors
        ColorField::new(
            ColorGroup::Semantic,
            "Success",
            |t| t.success,
            |t, c| t.success = c,
        ),
        ColorField::new(
            ColorGroup::Semantic,
            "Warning",
            |t| t.warning,
            |t, c| t.warning = c,
        ),
        ColorField::new(
            ColorGroup::Semantic,
            "Error",
            |t| t.error,
            |t, c| t.error = c,
        ),
        ColorField::new(ColorGroup::Semantic, "Info", |t| t.info, |t, c| t.info = c),
        // Meter colors
        ColorField::new(
            ColorGroup::Meter,
            "Meter Normal",
            |t| t.meter_normal,
            |t, c| t.meter_normal = c,
        ),
        ColorField::new(
            ColorGroup::Meter,
            "Meter Warning",
            |t| t.meter_warning,
            |t, c| t.meter_warning = c,
        ),
        ColorField::new(
            ColorGroup::Meter,
            "Meter Clip",
            |t| t.meter_clip,
            |t, c| t.meter_clip = c,
        ),
        ColorField::new(
            ColorGroup::Meter,
            "Meter Background",
            |t| t.meter_colors.background,
            |t, c| t.meter_colors.background = c,
        ),
        ColorField::new(
            ColorGroup::Meter,
            "Meter Normal (Full)",
            |t| t.meter_colors.normal,
            |t, c| t.meter_colors.normal = c,
        ),
        ColorField::new(
            ColorGroup::Meter,
            "Meter Warning (Full)",
            |t| t.meter_colors.warning,
            |t, c| t.meter_colors.warning = c,
        ),
        ColorField::new(
            ColorGroup::Meter,
            "Meter Clip (Full)",
            |t| t.meter_colors.clip,
            |t, c| t.meter_colors.clip = c,
        ),
        ColorField::new(
            ColorGroup::Meter,
            "Meter Peak",
            |t| t.meter_colors.peak,
            |t, c| t.meter_colors.peak = c,
        ),
        ColorField::new(
            ColorGroup::Meter,
            "Meter Text",
            |t| t.meter_colors.text,
            |t, c| t.meter_colors.text = c,
        ),
        // Button colors
        ColorField::new(
            ColorGroup::Button,
            "Mute Active",
            |t| t.button_mute_active,
            |t, c| t.button_mute_active = c,
        ),
        ColorField::new(
            ColorGroup::Button,
            "Solo Active",
            |t| t.button_solo_active,
            |t, c| t.button_solo_active = c,
        ),
        ColorField::new(
            ColorGroup::Button,
            "Dim Active",
            |t| t.button_dim_active,
            |t, c| t.button_dim_active = c,
        ),
        // Progress bar
        ColorField::new(
            ColorGroup::Progress,
            "Progress Background",
            |t| t.progress_bar_bg,
            |t, c| t.progress_bar_bg = c,
        ),
        ColorField::new(
            ColorGroup::Progress,
            "Progress Fill",
            |t| t.progress_bar_fill,
            |t, c| t.progress_bar_fill = c,
        ),
        // Toast backgrounds
        ColorField::new(
            ColorGroup::Toast,
            "Toast Success",
            |t| t.toast_success_bg,
            |t, c| t.toast_success_bg = c,
        ),
        ColorField::new(
            ColorGroup::Toast,
            "Toast Error",
            |t| t.toast_error_bg,
            |t, c| t.toast_error_bg = c,
        ),
        ColorField::new(
            ColorGroup::Toast,
            "Toast Info",
            |t| t.toast_info_bg,
            |t, c| t.toast_info_bg = c,
        ),
        ColorField::new(
            ColorGroup::Toast,
            "Toast Warning",
            |t| t.toast_warning_bg,
            |t, c| t.toast_warning_bg = c,
        ),
        // Plugin colors
        ColorField::new(
            ColorGroup::Plugin,
            "EQ",
            |t| t.plugin_colors.eq,
            |t, c| t.plugin_colors.eq = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Gain",
            |t| t.plugin_colors.gain,
            |t, c| t.plugin_colors.gain = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Upmixer",
            |t| t.plugin_colors.upmixer,
            |t, c| t.plugin_colors.upmixer = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Compressor",
            |t| t.plugin_colors.compressor,
            |t, c| t.plugin_colors.compressor = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Limiter",
            |t| t.plugin_colors.limiter,
            |t, c| t.plugin_colors.limiter = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Gate",
            |t| t.plugin_colors.gate,
            |t, c| t.plugin_colors.gate = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Loudness",
            |t| t.plugin_colors.loudness,
            |t, c| t.plugin_colors.loudness = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Binaural",
            |t| t.plugin_colors.binaural,
            |t, c| t.plugin_colors.binaural = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Convolution",
            |t| t.plugin_colors.convolution,
            |t, c| t.plugin_colors.convolution = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Monitor",
            |t| t.plugin_colors.monitor,
            |t, c| t.plugin_colors.monitor = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Spectrum",
            |t| t.plugin_colors.spectrum,
            |t, c| t.plugin_colors.spectrum = c,
        ),
        ColorField::new(
            ColorGroup::Plugin,
            "Mute/Solo",
            |t| t.plugin_colors.mute_solo,
            |t, c| t.plugin_colors.mute_solo = c,
        ),
        // Graph colors
        ColorField::new(
            ColorGroup::Graph,
            "Input",
            |t| t.graph_colors.input,
            |t, c| t.graph_colors.input = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "Target",
            |t| t.graph_colors.target,
            |t, c| t.graph_colors.target = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "Filter Response",
            |t| t.graph_colors.filter_response,
            |t, c| t.graph_colors.filter_response = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "Corrected",
            |t| t.graph_colors.corrected,
            |t, c| t.graph_colors.corrected = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "Error",
            |t| t.graph_colors.error,
            |t, c| t.graph_colors.error = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "Deviation",
            |t| t.graph_colors.deviation,
            |t, c| t.graph_colors.deviation = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "Grid",
            |t| t.graph_colors.grid,
            |t, c| t.graph_colors.grid = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "Secondary Line",
            |t| t.graph_colors.secondary_line,
            |t, c| t.graph_colors.secondary_line = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "Directivity ER",
            |t| t.graph_colors.directivity_er,
            |t, c| t.graph_colors.directivity_er = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "Directivity SP",
            |t| t.graph_colors.directivity_sp,
            |t, c| t.graph_colors.directivity_sp = c,
        ),
        // Spectrum colors
        ColorField::new(
            ColorGroup::Spectrum,
            "Spectrum Background",
            |t| t.spectrum_colors.background,
            |t, c| t.spectrum_colors.background = c,
        ),
        ColorField::new(
            ColorGroup::Spectrum,
            "Bass",
            |t| t.spectrum_colors.bass,
            |t, c| t.spectrum_colors.bass = c,
        ),
        ColorField::new(
            ColorGroup::Spectrum,
            "Mids",
            |t| t.spectrum_colors.mids,
            |t, c| t.spectrum_colors.mids = c,
        ),
        ColorField::new(
            ColorGroup::Spectrum,
            "Treble",
            |t| t.spectrum_colors.treble,
            |t, c| t.spectrum_colors.treble = c,
        ),
        // EQ Curve colors
        ColorField::new(
            ColorGroup::Graph,
            "EQ Background",
            |t| t.eq_curve_colors.background,
            |t, c| t.eq_curve_colors.background = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "EQ Grid",
            |t| t.eq_curve_colors.grid,
            |t, c| t.eq_curve_colors.grid = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "EQ Curve Boost",
            |t| t.eq_curve_colors.curve_boost,
            |t, c| t.eq_curve_colors.curve_boost = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "EQ Curve Cut",
            |t| t.eq_curve_colors.curve_cut,
            |t, c| t.eq_curve_colors.curve_cut = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "EQ Fill Boost",
            |t| t.eq_curve_colors.fill_boost,
            |t, c| t.eq_curve_colors.fill_boost = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "EQ Fill Cut",
            |t| t.eq_curve_colors.fill_cut,
            |t, c| t.eq_curve_colors.fill_cut = c,
        ),
        ColorField::new(
            ColorGroup::Graph,
            "EQ Zero Line",
            |t| t.eq_curve_colors.zero_line,
            |t, c| t.eq_curve_colors.zero_line = c,
        ),
        // Additional colors
        ColorField::new(
            ColorGroup::Additional,
            "Peak Indicator",
            |t| t.peak_indicator,
            |t, c| t.peak_indicator = c,
        ),
        ColorField::new(
            ColorGroup::Additional,
            "Drag Over Highlight",
            |t| t.drag_over_highlight,
            |t, c| t.drag_over_highlight = c,
        ),
        ColorField::new(
            ColorGroup::Additional,
            "Drag Over Border",
            |t| t.drag_over_border,
            |t, c| t.drag_over_border = c,
        ),
        ColorField::new(
            ColorGroup::Additional,
            "Neutral Indicator",
            |t| t.neutral_indicator,
            |t, c| t.neutral_indicator = c,
        ),
        ColorField::new(
            ColorGroup::Additional,
            "Warning Background",
            |t| t.warning_background,
            |t, c| t.warning_background = c,
        ),
        ColorField::new(
            ColorGroup::Additional,
            "Knob Color",
            |t| t.knob_color,
            |t, c| t.knob_color = c,
        ),
        ColorField::new(
            ColorGroup::Additional,
            "Optimization Color",
            |t| t.optimization_color,
            |t, c| t.optimization_color = c,
        ),
        ColorField::new(
            ColorGroup::Additional,
            "Grid Color",
            |t| t.grid_color,
            |t, c| t.grid_color = c,
        ),
    ]
}

/// Main view tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTab {
    #[default]
    Colors,
    Preview,
    Export,
}

/// Theme editor state
pub struct ThemeEditor {
    /// Current theme being edited
    pub theme: EditorTheme,
    /// Currently selected color group
    pub selected_group: ColorGroup,
    /// Currently selected color field index within group
    pub selected_field_index: usize,
    /// Current tab
    pub current_tab: EditorTab,
    /// All color fields
    pub color_fields: Vec<ColorField>,
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
}

impl ThemeEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let theme = EditorTheme::dark();
        let showcase = cx.new(|_| ComponentShowcase::new(theme.clone()));

        Self {
            theme,
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
        }
    }

    /// Get fields for a specific group
    fn fields_for_group(&self, group: ColorGroup) -> Vec<&ColorField> {
        self.color_fields
            .iter()
            .filter(|f| f.group == group)
            .collect()
    }

    /// Get current selected field
    fn current_field(&self) -> Option<&ColorField> {
        let fields = self.fields_for_group(self.selected_group);
        fields.get(self.selected_field_index).copied()
    }

    /// Update a color and sync to showcase
    fn update_color(&mut self, field: &ColorField, color: Color, cx: &mut Context<Self>) {
        (field.setter)(&mut self.theme, color);
        // Update showcase
        self.showcase.update(cx, |showcase, _| {
            showcase.set_theme(self.theme.clone());
        });
        cx.notify();
    }

    /// Load a preset theme
    fn load_preset(&mut self, preset: &str, cx: &mut Context<Self>) {
        self.theme = match preset {
            "dark" => EditorTheme::dark(),
            "light" => EditorTheme::light(),
            _ => EditorTheme::dark(),
        };
        self.showcase.update(cx, |showcase, _| {
            showcase.set_theme(self.theme.clone());
        });
        cx.notify();
    }

    /// Open color picker modal for current field
    fn open_color_modal(&mut self, cx: &mut Context<Self>) {
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
    fn apply_color_from_modal(&mut self, cx: &mut Context<Self>) {
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
    fn close_color_modal(&mut self, cx: &mut Context<Self>) {
        self.show_color_modal = false;
        self.color_picker = None;
        self.editing_field = None;
        cx.notify();
    }

    /// Render the sidebar with color groups
    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .id(SharedString::from(format!("group-{:?}", group)))
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
    fn render_color_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;
        let fields = self.fields_for_group(self.selected_group);
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
                    .id(SharedString::from(format!("field-{}", field.name)))
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
                                Text::new(SharedString::from(color.to_hex_string()))
                                    .size(TextSize::Xs)
                                    .color(theme.text_muted.to_rgba()),
                            )
                            .build(),
                    )
            }))
            .build()
    }

    /// Render color editor panel
    fn render_color_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;

        if let Some(field) = self.current_field() {
            let color = (field.getter)(&self.theme);
            let field_name = field.name;

            div().p_4().child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        Text::new(SharedString::from(format!("Edit: {}", field_name)))
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
                                Text::new(SharedString::from(color.to_hex_string()))
                                    .size(TextSize::Md)
                                    .weight(TextWeight::Medium)
                                    .color(theme.text_primary.to_rgba()),
                            )
                            .build(),
                    )
                    // RGBA display
                    .child(
                        Text::new(SharedString::from(format!(
                            "RGBA: {}, {}, {}, {}",
                            color.r, color.g, color.b, color.a
                        )))
                        .size(TextSize::Sm)
                        .color(theme.text_muted.to_rgba()),
                    )
                    // HSL display
                    .child({
                        let (h, s, l) = color.to_hsl();
                        Text::new(SharedString::from(format!(
                            "HSL: {:.0}°, {:.0}%, {:.0}%",
                            h * 360.0,
                            s * 100.0,
                            l * 100.0
                        )))
                        .size(TextSize::Sm)
                        .color(theme.text_muted.to_rgba())
                    })
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
    fn render_colors_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
    fn render_preview_tab(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.showcase.clone())
    }

    /// Render the export tab
    fn render_export_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;
        let export_format = self.export_format.clone();

        let export_content = if export_format == "json" {
            self.theme
                .to_json()
                .unwrap_or_else(|e| format!("Error: {}", e))
        } else {
            self.theme.to_rust_code()
        };

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
                                    cx.notify();
                                })),
                        )
                        .build(),
                )
                // Export preview
                .child(
                    div()
                        .flex_1()
                        .w_full()
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
                                .build(),
                        )
                        .child(
                            Button::new("save-btn", "Save to File")
                                .variant(ButtonVariant::Secondary)
                                .size(ButtonSize::Md)
                                .build(),
                        )
                        .build(),
                )
                .build(),
        )
    }

    /// Render the header with presets and tabs
    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child(
                                Button::new("preset-dark", "Dark")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm)
                                    .build()
                                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                        this.load_preset("dark", cx);
                                    })),
                            )
                            .child(
                                Button::new("preset-light", "Light")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm)
                                    .build()
                                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                        this.load_preset("light", cx);
                                    })),
                            )
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

    fn render_tab_button(
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
            .id(SharedString::from(format!("tab-{:?}", tab)))
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
    fn render_color_modal(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

        // The modal is a child of the backdrop. The dialog stops propagation so clicks
        // on it don't trigger the backdrop's close handler.
        div()
            .id("modal-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(backdrop_color)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                    this.close_color_modal(cx);
                }),
            )
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
