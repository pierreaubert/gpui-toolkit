use super::component_lab::ComponentLab;
use super::component_lab::ExportedStoryFamily;
use super::component_lab::exported_story_family;
use super::deep_link::coerce_prop_value;
use super::deep_link::encode_lab_deep_link;
use super::deep_link::parse_lab_deep_link;
use super::initial_lab_state::InitialLabState;
use super::misc::StoryPreviewKind;
use super::misc::button_variant;
use super::misc::design_for_theme_preset;
use super::misc::id_fragment;
use super::misc::lab_id;
use super::misc::lock_recover;
use super::misc::prop_number_label;
use super::misc::prop_value_label;
use super::misc::scatter_story_data;
use super::misc::showcase_section_for_story_id;
use super::misc::sidebar_window;
use super::misc::spectrum_axis_magnitudes;
use super::misc::spectrum_magnitudes;
use super::misc::story_preview_kind;
use super::misc::surface_colormap;
use super::number::number_prop;
use super::number::number_step;
use super::preview_align::PreviewAlign;
use super::preview_layout_constraints::PreviewLayoutConstraints;
use super::preview_overflow::PreviewOverflow;
use super::preview_sizing::PreviewSizing;
use super::preview_surface::PreviewSurface;
#[cfg(feature = "profiler")]
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
#[cfg(feature = "visual-capture")]
use gpui::{AppContext as _, TestAppContext};
use gpui_px::{Colormap, ScaleType};
use gpui_showcase::showcase::ShowcaseSection;
use gpui_ui_kit::ButtonVariant;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Mutex;
#[cfg(feature = "profiler")]
#[test]
fn story_file_names_are_stable() {
    assert_eq!(
        story_file_name("audio-kit.potentiometer"),
        "audio~2dkit~2epotentiometer.story.json"
    );
    assert_ne!(story_file_name("a.b"), story_file_name("a-b"));
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

#[cfg(feature = "visual-capture")]
#[gpui::test]
async fn manual_reload_restores_selected_document_layout(cx: &mut TestAppContext) {
    let temporary = tempfile::tempdir().expect("temporary story directory");
    let stories_dir = temporary.path().join("stories");
    std::fs::create_dir_all(&stories_dir).expect("create story directory");

    let story = crate::builtin_story_registry()
        .expect("builtin story registry")
        .story("ui-kit.button")
        .expect("button story")
        .clone();
    let viewport_id = story
        .viewports
        .last()
        .expect("button viewport preset")
        .id
        .to_string();
    let theme_id = story
        .themes
        .last()
        .expect("button theme preset")
        .id
        .to_string();
    let motion_id = story
        .motions
        .last()
        .expect("button motion preset")
        .id
        .to_string();
    let mut persisted = StoryDocument::new(story);
    persisted.layout = json!({
        "viewport": viewport_id,
        "theme": theme_id,
        "motion": motion_id,
        "matrix": true,
        "constraints": {
            "sizing": "fixed",
            "min_width": 440.0,
            "padding": 16.0,
        },
        "builder": {
            "horizontal_align": "start",
            "overflow": "visible",
        },
    });
    let expected = InitialLabState::from_document(&persisted);
    persisted
        .save_story_json(&stories_dir.join("button.story.json"))
        .expect("write persisted story");

    let config = super::lab_app_config::LabAppConfig::new(stories_dir, Vec::new());
    let lab = cx.new(|cx| ComponentLab::new(config, cx));
    lab.update(cx, |lab, cx| {
        lab.select_story("ui-kit.button".to_owned(), cx);
        lab.set_viewport("stale-viewport", cx);
        lab.set_theme("stale-theme", cx);
        lab.set_motion("stale-motion", cx);
        lab.set_layout_min_width(1600.0, cx);
        if lab.matrix_mode {
            lab.toggle_matrix(cx);
        }

        lab.reload_documents(cx);

        assert_eq!(lab.selected_viewport_id, expected.viewport_id);
        assert_eq!(lab.selected_theme_id, expected.theme_id);
        assert_eq!(lab.selected_motion_id, expected.motion_id);
        assert_eq!(lab.matrix_mode, expected.matrix_mode);
        assert_eq!(lab.layout_constraints, expected.layout_constraints);
        assert_eq!(lab.save_status.as_deref(), Some("Reloaded story JSON"));
        lab.sync_layout_state();
        assert_eq!(lab.save_status.as_deref(), Some("Reloaded story JSON"));
    });
}

#[cfg(feature = "visual-capture")]
#[gpui::test]
async fn stateful_preview_survives_parent_layout_redraw(cx: &mut TestAppContext) {
    let temporary = tempfile::tempdir().expect("temporary story directory");
    let config =
        super::lab_app_config::LabAppConfig::new(temporary.path().join("stories"), Vec::new());
    let lab = cx.new(|cx| ComponentLab::new(config, cx));

    lab.update(cx, |lab, cx| {
        lab.select_story("ui-kit.color-picker".to_owned(), cx);
        let original_entity_id = lab
            .retained_stateful_preview_id()
            .expect("color picker has a retained preview entity");

        lab.set_layout_padding(24.0, cx);

        assert_eq!(
            lab.retained_stateful_preview_id(),
            Some(original_entity_id),
            "an unrelated parent redraw must not recreate the color picker"
        );
    });
}

#[cfg(feature = "visual-capture")]
#[gpui::test]
async fn switching_showcase_stories_releases_inactive_showcase(cx: &mut TestAppContext) {
    let temporary = tempfile::tempdir().expect("temporary story directory");
    let config =
        super::lab_app_config::LabAppConfig::new(temporary.path().join("stories"), Vec::new());
    let lab = cx.new(|cx| ComponentLab::new(config, cx));

    lab.update(cx, |lab, cx| {
        lab.select_story("ui-kit.command-palette".to_owned(), cx);
        assert_eq!(lab.ui_showcases.len(), 1);
        assert!(lab.ui_showcases.contains_key("ui-kit.command-palette"));

        lab.select_story("ui-kit.accessibility".to_owned(), cx);
        assert_eq!(lab.ui_showcases.len(), 1);
        assert!(lab.ui_showcases.contains_key("ui-kit.accessibility"));
    });
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
    !matches!(
        story_preview_kind(
            story_id,
            UI_KIT_EXPORTED_COMPONENT_STORY_IDS.contains(&story_id),
            showcase_section_for_story_id(story_id).is_some(),
            true,
        ),
        StoryPreviewKind::Missing | StoryPreviewKind::RendererFallback
    )
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
    assert_eq!(first, "lab-story-ui~2dkit~2ebutton");
    assert_eq!(first, second);

    assert_eq!(id_fragment("ui-kit.button"), "ui~2dkit~2ebutton");
    assert_ne!(lab_id(&["story", "a.b"]), lab_id(&["story", "a-b"]));
    assert_ne!(id_fragment("a.b"), id_fragment("a-b"));
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

#[test]
fn prop_strings_are_cached_from_documents() {
    let mut story = ComponentStory::new("ui-kit.button", "gpui-ui-kit", "Button", "A button");
    let mut variant = StoryProp::new(
        "variant",
        "Variant",
        StoryPropValue::Text("Primary".into()),
    );
    variant.options = vec!["Primary".to_string(), "Ghost".to_string()];
    story.props.push(variant);
    let mut documents = BTreeMap::new();
    documents.insert("ui-kit.button".to_string(), StoryDocument::new(story));
    let cached = ComponentLab::build_prop_strings(&documents);
    assert_eq!(cached.len(), 1);
    let entry = &cached[&("ui-kit.button".to_string(), "variant".to_string())];
    assert_eq!(entry.story_id, "ui-kit.button");
    assert_eq!(entry.name, "variant");
    assert_eq!(entry.label, "Variant");
    assert_eq!(entry.option_label("Ghost"), "Ghost");
    assert_eq!(entry.option_label("Missing"), "Missing");
}

#[test]
fn cache_locks_recover_from_poison() {
    let mutex = Mutex::new(7u32);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = mutex.lock().unwrap();
        panic!("intentional poison for lock_recover");
    }));
    assert!(mutex.is_poisoned());
    assert_eq!(*lock_recover(&mutex), 7);
    *lock_recover(&mutex) = 8;
    assert_eq!(*lock_recover(&mutex), 8);
}

#[test]
fn prop_number_labels_are_cached_and_formatted() {
    assert_eq!(prop_number_label(0.25), "0.25");
    assert_eq!(prop_number_label(0.25), prop_number_label(0.25));
    assert_eq!(prop_number_label(1.0), "1.00");
    assert_eq!(
        prop_value_label(&StoryPropValue::Number(2.5)),
        prop_number_label(2.5)
    );
}

#[test]
fn sidebar_window_cases() {
    assert_eq!(sidebar_window(0, 10, 200), (0, 10));
    assert_eq!(sidebar_window(0, 0, 200), (0, 0));
    assert_eq!(sidebar_window(5, 10, 0), (0, 10));
    assert_eq!(sidebar_window(250, 500, 200), (150, 350));
    assert_eq!(sidebar_window(0, 500, 200), (0, 200));
    assert_eq!(sidebar_window(499, 500, 200), (300, 500));
    assert_eq!(sidebar_window(9999, 500, 200), (300, 500));
}

#[test]
fn story_preview_kind_covers_builtin_handlers() {
    let extra_mesh = ["px.mesh_plot.mesh_only", "px.mesh_plot.surface3d"];
    for story_id in crate::BUILTIN_RENDERER_STORY_IDS
        .iter()
        .copied()
        .chain(crate::PX_CHART_STORY_IDS.iter().copied())
        .chain(extra_mesh)
    {
        let kind = story_preview_kind(
            story_id,
            UI_KIT_EXPORTED_COMPONENT_STORY_IDS.contains(&story_id),
            showcase_section_for_story_id(story_id).is_some(),
            true,
        );
        assert!(
            !matches!(
                kind,
                StoryPreviewKind::Missing | StoryPreviewKind::RendererFallback
            ),
            "{story_id} has no preview handler: {kind:?}"
        );
    }
}

#[test]
fn story_preview_kind_fallbacks_and_precedence() {
    assert_eq!(
        story_preview_kind("ui-kit.button", false, false, false),
        StoryPreviewKind::Button
    );
    assert_eq!(
        story_preview_kind("ui-kit.button-set", true, true, true),
        StoryPreviewKind::ExportedUiKit
    );
    assert_eq!(
        story_preview_kind("ui-kit.buttons", false, true, true),
        StoryPreviewKind::Showcase
    );
    assert_eq!(
        story_preview_kind("px.mesh_plot.custom", false, false, false),
        StoryPreviewKind::MeshPlot
    );
    assert_eq!(
        story_preview_kind("other-thing", false, false, true),
        StoryPreviewKind::RendererFallback
    );
    assert_eq!(
        story_preview_kind("other-thing", false, false, false),
        StoryPreviewKind::Missing
    );
    assert_eq!(
        story_preview_kind("", false, false, false),
        StoryPreviewKind::Missing
    );
}

#[test]
fn exported_story_families_cover_all_exported_ids() {
    // Base ids such as `ui-kit.button` are in the exported set but are
    // claimed by explicit preview arms before the exported guard runs.
    for story_id in UI_KIT_EXPORTED_COMPONENT_STORY_IDS.iter().copied() {
        let family = exported_story_family(story_id);
        let explicit =
            story_preview_kind(story_id, true, false, false) != StoryPreviewKind::ExportedUiKit;
        assert!(
            family != ExportedStoryFamily::Unknown || explicit,
            "{story_id} reaches the exported renderer with no family"
        );
    }
    assert_eq!(
        exported_story_family("ui-kit.button-set"),
        ExportedStoryFamily::Feedback
    );
    assert_eq!(
        exported_story_family("ui-kit.toggle"),
        ExportedStoryFamily::Input
    );
    assert_eq!(
        exported_story_family("ui-kit.badge"),
        ExportedStoryFamily::Display
    );
    assert_eq!(
        exported_story_family("ui-kit.tabs-component"),
        ExportedStoryFamily::Navigation
    );
    for story_id in ["", "ui-kit.nope", "ui-kit.button", "px.line"] {
        assert_eq!(
            exported_story_family(story_id),
            ExportedStoryFamily::Unknown,
            "{story_id}"
        );
    }
}

#[test]
fn showcase_sections_cover_known_stories_and_reject_edges() {
    for (story_id, section) in [
        ("ui-kit.buttons", ShowcaseSection::Buttons),
        ("ui-kit.text", ShowcaseSection::Text),
        ("ui-kit.form-controls", ShowcaseSection::FormControls),
        ("ui-kit.dialog", ShowcaseSection::Dialog),
        ("ui-kit.tree-view", ShowcaseSection::TreeView),
        ("ui-kit.accessibility", ShowcaseSection::Accessibility),
    ] {
        assert_eq!(showcase_section_for_story_id(story_id), Some(section));
    }
    for story_id in [
        "",
        "ui-kit.button",
        "ui-kit.buttons ",
        "UI-KIT.BUTTONS",
        "ui-kit.buttons/ui",
        "px.line",
    ] {
        assert_eq!(showcase_section_for_story_id(story_id), None, "{story_id}");
    }
}

#[test]
fn deep_links_round_trip() {
    let link = encode_lab_deep_link(
        "ui-kit.button",
        &[("variant", "primary"), ("label", "Hello World & Co")],
    );
    let parsed = parse_lab_deep_link(&link).expect("round trip");
    assert_eq!(parsed.story_id, "ui-kit.button");
    assert_eq!(
        parsed.props,
        vec![
            ("variant".to_string(), "primary".to_string()),
            ("label".to_string(), "Hello World & Co".to_string()),
        ]
    );
    assert_eq!(encode_lab_deep_link("px.line", &[]), "?story=px.line");
}

#[test]
fn deep_links_reject_bad_input() {
    assert_eq!(parse_lab_deep_link(""), None);
    assert_eq!(parse_lab_deep_link("?"), None);
    assert_eq!(parse_lab_deep_link("?story"), None);
    assert_eq!(parse_lab_deep_link("?story="), None);
    assert_eq!(parse_lab_deep_link("?prop.variant=primary"), None);
    assert_eq!(parse_lab_deep_link("?story=%ZZ"), None);
    assert_eq!(
        parse_lab_deep_link("story=px.line")
            .expect("bare query")
            .story_id,
        "px.line"
    );
}

#[test]
fn deep_link_prop_coercion_matches_declared_types() {
    assert_eq!(
        coerce_prop_value(&StoryPropValue::Bool(false), "on"),
        StoryPropValue::Bool(true)
    );
    assert_eq!(
        coerce_prop_value(&StoryPropValue::Bool(true), "maybe"),
        StoryPropValue::Bool(true)
    );
    assert_eq!(
        coerce_prop_value(&StoryPropValue::Number(1.0), "2.5"),
        StoryPropValue::Number(2.5)
    );
    assert_eq!(
        coerce_prop_value(&StoryPropValue::Number(1.0), "NaN"),
        StoryPropValue::Number(1.0)
    );
    assert_eq!(
        coerce_prop_value(&StoryPropValue::Number(1.0), "junk"),
        StoryPropValue::Number(1.0)
    );
    assert_eq!(
        coerce_prop_value(&StoryPropValue::Text("a".into()), "b"),
        StoryPropValue::Text("b".into())
    );
}
