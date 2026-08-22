use super::component_lab::ComponentLab;
use super::initial_lab_state::InitialLabState;
use super::misc::button_variant;
use super::misc::design_for_theme_preset;
use super::misc::id_fragment;
use super::misc::lab_id;
use super::misc::prop_value_label;
use super::misc::scatter_story_data;
use super::misc::showcase_section_for_story_id;
use super::misc::spectrum_axis_magnitudes;
use super::misc::spectrum_magnitudes;
use super::misc::surface_colormap;
use super::number::number_prop;
use super::number::number_step;
use super::preview_align::PreviewAlign;
use super::preview_layout_constraints::PreviewLayoutConstraints;
use super::preview_overflow::PreviewOverflow;
use super::preview_sizing::PreviewSizing;
use super::preview_surface::PreviewSurface;
use super::story::story_file_name;
use super::story::text_prop;
use super::types::area_story_data;
use super::types::bar_story_data;
use super::types::line_story_data;
use crate::{
    ComponentStory, StoryDocument, StoryProp, StoryPropValue, ThemePreset,
    UI_KIT_EXPORTED_COMPONENT_STORY_IDS,
};
use gpui::SharedString;
#[cfg(feature = "profiler")]
use gpui_px::{Colormap, ScaleType};
use gpui_showcase::showcase::ShowcaseSection;
use gpui_ui_kit::ButtonVariant;
use serde_json::json;
use std::collections::BTreeMap;
#[cfg(feature = "profiler")]
#[test]
fn story_file_names_are_stable() {
    assert_eq!(
        story_file_name("audio-kit.potentiometer"),
        "audio_kit_potentiometer.story.json"
    );
}

#[test]
fn prop_helpers_fall_back_by_type() {
    let story =
        ComponentStory::new("test", "crate", "Title", "Description").props([StoryProp::new(
            "value",
            "Value",
            StoryPropValue::Number(0.25),
        )]);
    assert_eq!(number_prop(&story, "value", 1.0), 0.25);
    assert_eq!(text_prop(&story, "value", "fallback"), "fallback");
    assert_eq!(button_variant("outline"), ButtonVariant::Outline);
}

#[test]
fn theme_presets_resolve_design_languages() {
    let apple = ThemePreset::new("apple-hig", "Apple HIG", "apple_hig", false);
    let unknown = ThemePreset::new("custom", "Custom", "missing", false);

    assert_eq!(
        design_for_theme_preset(&apple).language.as_str(),
        "apple_hig"
    );
    assert_eq!(
        design_for_theme_preset(&unknown).language.as_str(),
        "neutral"
    );
}

#[test]
fn integer_story_props_use_integer_steps() {
    for prop_name in ["bars", "points", "size", "slices", "groups", "selected"] {
        assert_eq!(number_step(prop_name), 1.0);
    }
}

#[test]
fn layout_state_is_serializable() {
    let mut doc = StoryDocument::new(ComponentStory::new(
        "ui-kit.button",
        "gpui-ui-kit",
        "Button",
        "Primary action button",
    ));
    doc.layout = json!({
        "viewport": "mobile",
        "theme": "neutral",
        "motion": "reduced",
        "matrix": true,
        "constraints": {
            "sizing": "fixed",
            "min_width": 420.0,
            "min_height": 260.0,
            "aspect_ratio": 1.4,
            "padding": 16.0
        },
        "builder": {
            "horizontal_align": "start",
            "vertical_align": "stretch",
            "overflow": "scroll",
            "surface": "surface",
            "gap": 12.0,
            "border": false
        }
    });
    let serialized = serde_json::to_string(&doc).unwrap();
    assert!(serialized.contains("\"matrix\":true"));
    assert!(serialized.contains("\"motion\":\"reduced\""));
    assert!(serialized.contains("\"sizing\":\"fixed\""));
    assert!(serialized.contains("\"horizontal_align\":\"start\""));
    assert!(serialized.contains("\"overflow\":\"scroll\""));
}

#[test]
fn layout_constraints_parse_with_clamps() {
    let layout = json!({
        "constraints": {
            "sizing": "fit",
            "min_width": 80.0,
            "min_height": 9000.0,
            "aspect_ratio": 9.0,
            "padding": -8.0
        },
        "builder": {
            "horizontal_align": "end",
            "vertical_align": "stretch",
            "overflow": "scroll",
            "surface": "transparent",
            "gap": 900.0,
            "border": false
        }
    });
    let constraints = PreviewLayoutConstraints::from_layout(&layout);
    assert_eq!(constraints.sizing, PreviewSizing::Fit);
    assert_eq!(constraints.min_width, 160.0);
    assert_eq!(constraints.min_height, 1200.0);
    assert_eq!(constraints.aspect_ratio, 3.0);
    assert_eq!(constraints.padding, 0.0);
    assert_eq!(constraints.horizontal_align, PreviewAlign::End);
    assert_eq!(constraints.vertical_align, PreviewAlign::Stretch);
    assert_eq!(constraints.overflow, PreviewOverflow::Scroll);
    assert_eq!(constraints.surface, PreviewSurface::Transparent);
    assert_eq!(constraints.gap, 80.0);
    assert!(!constraints.border);
}

#[test]
fn initial_state_uses_saved_layout_when_valid() {
    let story = ComponentStory::new("ui-kit.button", "gpui-ui-kit", "Button", "Button");
    let mut doc = StoryDocument::new(story);
    doc.layout = json!({
        "viewport": "tablet",
        "theme": "apple-hig",
        "motion": "reduced",
        "matrix": true,
        "constraints": { "sizing": "fixed", "min_width": 440.0 },
        "builder": { "horizontal_align": "start", "overflow": "visible" }
    });
    let state = InitialLabState::from_document(&doc);
    assert_eq!(state.viewport_id, "tablet");
    assert_eq!(state.theme_id, "apple-hig");
    assert_eq!(state.motion_id, "reduced");
    assert!(state.matrix_mode);
    assert_eq!(state.layout_constraints.sizing, PreviewSizing::Fixed);
    assert_eq!(state.layout_constraints.min_width, 440.0);
    assert_eq!(
        state.layout_constraints.horizontal_align,
        PreviewAlign::Start
    );
    assert_eq!(state.layout_constraints.overflow, PreviewOverflow::Visible);
}

#[test]
fn px_line_story_data_is_chart_safe() {
    let sweep = line_story_data("sweep");
    assert_eq!(sweep.x.len(), sweep.y.len());
    assert!(sweep.x.iter().all(|value| *value > 0.0));
    assert_eq!(sweep.x_scale, ScaleType::Log);
    assert_eq!(
        sweep.comparison_y.as_ref().map(|values| values.len()),
        Some(sweep.x.len())
    );

    let flat = line_story_data("flat");
    assert_eq!(flat.x.len(), flat.y.len());
    assert!(flat.comparison_y.is_none());
}

#[test]
fn px_bar_story_data_matches_categories() {
    let bars = bar_story_data(7);
    assert_eq!(bars.categories.len(), 7);
    assert_eq!(bars.values.len(), bars.categories.len());
    assert_eq!(bars.comparison_values.len(), bars.categories.len());
}

#[test]
fn showcase_story_ids_map_to_sections() {
    assert_eq!(
        showcase_section_for_story_id("ui-kit.command-palette"),
        Some(ShowcaseSection::CommandPalette)
    );
    assert_eq!(
        showcase_section_for_story_id("ui-kit.accessibility"),
        Some(ShowcaseSection::Accessibility)
    );
    assert_eq!(showcase_section_for_story_id("ui-kit.button"), None);
}

#[test]
fn builtin_renderer_story_ids_have_preview_handlers() {
    let missing = crate::BUILTIN_RENDERER_STORY_IDS
        .iter()
        .copied()
        .filter(|story_id| !builtin_preview_handler_story_id(story_id))
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "builtin renderer stories without preview handlers: {missing:?}"
    );
}

fn builtin_preview_handler_story_id(story_id: &str) -> bool {
    matches!(
        story_id,
        "ui-kit.button"
            | "ui-kit.form"
            | "ui-kit.status"
            | "ui-kit.navigation"
            | "ui-kit.feedback"
            | "ui-kit.card"
            | "audio-kit.potentiometer"
            | "audio-kit.vertical-slider"
            | "audio-kit.volume-knob"
            | "audio-kit.meter"
            | "audio-kit.horizontal-meter"
            | "audio-kit.spectrum"
            | "audio-kit.spectrum-axis"
    ) || UI_KIT_EXPORTED_COMPONENT_STORY_IDS.contains(&story_id)
        || showcase_section_for_story_id(story_id).is_some()
        || crate::PX_CHART_STORY_IDS.contains(&story_id)
        || story_id.starts_with("px.mesh_plot.")
}

#[test]
fn surface_colormap_names_are_stable() {
    assert_eq!(surface_colormap("plasma"), Colormap::Plasma);
    assert_eq!(surface_colormap("coolwarm"), Colormap::CoolWarm);
    assert_eq!(surface_colormap("missing"), Colormap::Viridis);
}

#[test]
fn lab_ids_are_stable_and_cached() {
    let first = lab_id(&["story", "ui-kit.button"]);
    let second = lab_id(&["story", "ui-kit.button"]);
    assert_eq!(first, "lab-story-ui-kit-button");
    assert_eq!(first, second);

    assert_eq!(id_fragment("ui-kit.button"), "ui-kit-button");
    assert!(matches!(
        id_fragment("alphanumeric"),
        std::borrow::Cow::Borrowed(_)
    ));
}

#[test]
fn prop_value_label_formats_by_type() {
    assert_eq!(prop_value_label(&StoryPropValue::Bool(true)), "true");
    assert_eq!(prop_value_label(&StoryPropValue::Number(1.5)), "1.50");
    assert_eq!(
        prop_value_label(&StoryPropValue::Text(SharedString::new("hello"))),
        "hello"
    );
}

#[test]
fn generated_data_helpers_are_deterministic() {
    let scatter1 = scatter_story_data(24);
    let scatter2 = scatter_story_data(24);
    assert_eq!(scatter1.0, scatter2.0);
    assert_eq!(scatter1.1, scatter2.1);

    let box1 = super::misc::boxplot_story_data(4);
    let box2 = super::misc::boxplot_story_data(4);
    assert_eq!(box1.0, box2.0);

    let scalar1 = super::misc::scalar_field_data(8, 8);
    let scalar2 = super::misc::scalar_field_data(8, 8);
    assert_eq!(scalar1, scalar2);

    let tree1 = super::misc::treemap_story_data();
    let tree2 = super::misc::treemap_story_data();
    assert_eq!(tree1.total_value(), tree2.total_value());

    let spec1 = spectrum_magnitudes(32);
    let spec2 = spectrum_magnitudes(32);
    assert_eq!(spec1.len(), 32);
    assert_eq!(spec1, spec2);

    let axis1 = spectrum_axis_magnitudes();
    let axis2 = spectrum_axis_magnitudes();
    assert_eq!(axis1.len(), 72);
    assert_eq!(axis1, axis2);
}

#[test]
fn line_and_area_story_data_are_cached() {
    let sweep1 = line_story_data("sweep");
    let sweep2 = line_story_data("sweep");
    assert_eq!(sweep1.x, sweep2.x);
    assert_eq!(sweep1.y, sweep2.y);

    let area1 = area_story_data("decay");
    let area2 = area_story_data("decay");
    assert_eq!(area1.x, area2.x);
    assert_eq!(area1.y, area2.y);

    let bars1 = bar_story_data(6);
    let bars2 = bar_story_data(6);
    assert_eq!(bars1.values, bars2.values);
    assert_eq!(bars1.categories, bars2.categories);
}

#[test]
fn sidebar_labels_are_built_from_documents() {
    let story = ComponentStory::new("ui-kit.button", "gpui-ui-kit", "Button", "A button");
    let doc = StoryDocument::new(story);
    let mut documents = BTreeMap::new();
    documents.insert("ui-kit.button".to_string(), doc);
    let labels = ComponentLab::build_sidebar_labels(&documents, &["ui-kit.button".to_string()]);
    assert_eq!(labels.len(), 1);
    assert_eq!(labels["ui-kit.button"], "gpui-ui-kit / Button");
}
