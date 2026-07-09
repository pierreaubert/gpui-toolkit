#![allow(clippy::cloned_ref_to_slice_refs)]
use super::builtin::builtin_story_has_renderer;
use super::builtin::builtin_story_registry;
use super::builtin::builtin_story_renderers;
use super::component_lab_conformance_finding::ComponentLabConformanceFinding;
use super::component_lab_conformance_report::{
    ComponentLabConformanceReport, ensure_component_lab_conformance_passed,
};
use super::component_story::ComponentStory;
use super::consts::BUILTIN_RENDERER_STORY_IDS;
use super::consts::PX_CHART_STORY_IDS;
use super::consts::PX_CHART_STORY_TYPES;
use super::consts::UI_KIT_EXPORTED_COMPONENT_STORY_IDS;
use super::consts::UI_KIT_EXPORTED_COMPONENT_STORY_TYPES;
use super::default::default_theme_presets;
use super::latest::latest_rust_source_modified;
use super::responsive_preview_matrix::ResponsivePreviewMatrix;
use super::story_document::StoryDocument;
use super::story_renderer_kind::StoryRendererKind;
use super::types::StoryPropValue;
use super::types::reload_live_preview_state;
use super::visual_regression_manifest::{
    COMPONENT_LAB_VISUAL_DIFF_REPORT_TYPE, COMPONENT_LAB_VISUAL_DIFF_SCHEMA_VERSION,
    COMPONENT_LAB_VISUAL_MANIFEST_SCHEMA_VERSION, ComponentLabVisualCase,
    ComponentLabVisualDiffStatus, ComponentLabVisualManifest,
};
use gpui_design_tools::{DesignTokenFormat, DesignTokenValidationReport, export_design_tokens};
use image::{Rgba, RgbaImage};
use std::collections::BTreeSet;
use std::time::SystemTime;

#[path = "tests/misc.rs"]
mod misc;

#[test]
fn builtin_registry_covers_requested_crates() {
    let registry = builtin_story_registry().unwrap();
    assert!(registry.story("ui-kit.button").is_some());
    assert!(registry.story("ui-kit.status").is_some());
    assert!(registry.story("ui-kit.command-palette").is_some());
    assert!(registry.story("ui-kit.accessibility").is_some());
    assert!(registry.story("px.line").is_some());
    assert!(registry.story("px.heatmap").is_some());
    assert!(registry.story("px.treemap").is_some());
    assert!(registry.story("px.surface3d").is_some());
    assert!(registry.story("audio-kit.potentiometer").is_some());
    assert!(registry.story("audio-kit.vertical-slider").is_some());
    assert!(registry.story("audio-kit.volume-knob").is_some());
    assert!(registry.story("audio-kit.horizontal-meter").is_some());
    assert!(registry.story("audio-kit.spectrum-axis").is_some());
}

#[test]
fn builtin_registry_has_renderer_coverage() {
    let registry = builtin_story_registry().unwrap();
    for story in registry.stories() {
        assert!(
            builtin_story_has_renderer(&story.id),
            "missing renderer coverage for {}",
            story.id
        );
    }
}

#[test]
fn builtin_renderer_registry_covers_story_ids() {
    let renderers = builtin_story_renderers().unwrap();
    assert_eq!(renderers.len(), BUILTIN_RENDERER_STORY_IDS.len());
    for story_id in BUILTIN_RENDERER_STORY_IDS {
        assert!(
            renderers.renderer(story_id).is_some(),
            "missing renderer metadata for {story_id}"
        );
    }
}

#[test]
fn builtin_renderer_metadata_is_typed() {
    let renderers = builtin_story_renderers().unwrap();
    let button = renderers.renderer("ui-kit.button").unwrap();
    assert_eq!(button.kind, StoryRendererKind::Component);
    assert!(button.interactive);
    assert!(button.matrix_preview);

    let showcase = renderers.renderer("ui-kit.table").unwrap();
    assert_eq!(showcase.kind, StoryRendererKind::Showcase);
    assert!(!showcase.matrix_preview);

    let chart = renderers.renderer("px.surface3d").unwrap();
    assert_eq!(chart.kind, StoryRendererKind::Chart);
    assert!(!chart.interactive);

    let audio = renderers.renderer("audio-kit.volume-knob").unwrap();
    assert_eq!(audio.kind, StoryRendererKind::Audio);
    assert!(audio.interactive);
}

#[test]
fn exported_ui_kit_component_types_have_bespoke_stories() {
    let registry = builtin_story_registry().unwrap();
    let renderers = builtin_story_renderers().unwrap();
    let mut story_ids_from_types = Vec::new();
    for (component_type, story_id) in UI_KIT_EXPORTED_COMPONENT_STORY_TYPES {
        story_ids_from_types.push(*story_id);
        let story = registry
            .story(story_id)
            .unwrap_or_else(|| panic!("missing component story for {component_type}"));
        assert_eq!(story.crate_name, "gpui-ui-kit");
        assert!(
            !story.props.is_empty(),
            "{component_type} story {story_id} must expose editable prop metadata"
        );
        let renderer = renderers
            .renderer(story_id)
            .unwrap_or_else(|| panic!("missing component renderer for {component_type}"));
        assert_eq!(renderer.kind, StoryRendererKind::Component);
    }
    assert_eq!(story_ids_from_types, UI_KIT_EXPORTED_COMPONENT_STORY_IDS);
}

#[test]
fn px_stories_expose_responsive_fill_prop() {
    let registry = builtin_story_registry().unwrap();
    for story_id in PX_CHART_STORY_IDS {
        let story = registry
            .story(story_id)
            .unwrap_or_else(|| panic!("missing px chart story {story_id}"));
        assert!(
            story.props.iter().any(|prop| prop.name == "fill"),
            "{} must expose the fill/fixed sizing toggle",
            story.id
        );
    }
}

#[test]
fn px_stories_have_responsive_rendered_conformance() {
    let registry = builtin_story_registry().unwrap();
    let renderers = builtin_story_renderers().unwrap();
    let mut story_ids_from_types = Vec::new();
    for (chart_type, story_id) in PX_CHART_STORY_TYPES {
        story_ids_from_types.push(*story_id);
        let story = registry
            .story(story_id)
            .unwrap_or_else(|| panic!("missing px chart story for {chart_type}"));
        assert!(story.conformance.responsive, "{}", story.id);
        assert!(!story.conformance.rendered.allow_scroll, "{}", story.id);
        assert!(
            story.conformance.rendered.min_width <= 390.0,
            "{} must fit mobile width",
            story.id
        );
        assert!(
            story.conformance.rendered.min_height <= 844.0,
            "{} must fit mobile height",
            story.id
        );
        let renderer = renderers
            .renderer(story_id)
            .unwrap_or_else(|| panic!("missing px chart renderer for {chart_type}"));
        assert_eq!(renderer.kind, StoryRendererKind::Chart);
        assert!(renderer.matrix_preview, "{}", story.id);
    }
    assert_eq!(story_ids_from_types, PX_CHART_STORY_IDS);

    let px_story_ids = registry
        .stories()
        .filter(|story| story.crate_name == "gpui-px")
        .map(|story| story.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected_ids = PX_CHART_STORY_IDS.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(px_story_ids, expected_ids);
}

#[test]
fn responsive_matrix_crosses_viewports_and_themes() {
    let registry = builtin_story_registry().unwrap();
    let story = registry.story("audio-kit.meter").unwrap();
    let matrix = ResponsivePreviewMatrix::for_story(story);
    assert_eq!(
        matrix.cells.len(),
        story.viewports.len() * story.themes.len()
    );
}

#[test]
fn visual_manifest_expands_renderer_backed_stories_for_ci_screenshots() {
    let stories = builtin_story_registry().unwrap();
    let renderers = builtin_story_renderers().unwrap();
    let manifest =
        ComponentLabVisualManifest::from_registries(&stories, &renderers, "target/lab-visual");

    assert_eq!(
        manifest.schema_version,
        COMPONENT_LAB_VISUAL_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(manifest.case_count, manifest.cases.len());
    assert!(manifest.case_count >= BUILTIN_RENDERER_STORY_IDS.len());

    let button_cases = manifest
        .cases
        .iter()
        .filter(|case| case.story_id == "ui-kit.button")
        .collect::<Vec<_>>();
    let button = stories.story("ui-kit.button").unwrap();
    assert_eq!(
        button_cases.len(),
        button.viewports.len() * button.themes.len()
    );
    assert!(
        button_cases
            .iter()
            .all(|case| case.capture_id.starts_with("ui-kit-button__"))
    );
    assert!(button_cases.iter().all(|case| {
        case.baseline_path
            .starts_with("target/lab-visual/baseline/")
            && case.actual_path.starts_with("target/lab-visual/actual/")
            && case.diff_path.starts_with("target/lab-visual/diff/")
    }));

    let showcase_cases = manifest
        .cases
        .iter()
        .filter(|case| case.story_id == "ui-kit.table")
        .collect::<Vec<_>>();
    assert_eq!(showcase_cases.len(), 1);
    assert_eq!(showcase_cases[0].renderer_kind, StoryRendererKind::Showcase);
}

#[test]
fn visual_manifest_markdown_table_is_ci_attachable() {
    let stories = builtin_story_registry().unwrap();
    let renderers = builtin_story_renderers().unwrap();
    let manifest =
        ComponentLabVisualManifest::from_registries(&stories, &renderers, "target/lab-visual");
    let markdown = manifest.to_markdown_table();

    assert!(markdown.contains("| capture | story | viewport | theme | baseline | actual | diff |"));
    assert!(markdown.contains("`ui-kit-button__"));
    assert!(markdown.contains("target/lab-visual/baseline"));
    assert!(markdown.contains("target/lab-visual/diff"));
}

#[test]
fn visual_manifest_diff_compares_png_captures_and_writes_diff() {
    let temp = tempfile::tempdir().unwrap();
    let baseline_path = temp.path().join("baseline.png");
    let actual_path = temp.path().join("actual.png");
    let diff_path = temp.path().join("diff").join("case.png");

    let baseline = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 0, 255]));
    baseline.save(&baseline_path).unwrap();
    let mut actual = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 0, 255]));
    actual.put_pixel(1, 1, Rgba([255, 0, 0, 255]));
    actual.save(&actual_path).unwrap();

    let manifest = ComponentLabVisualManifest {
        schema_version: COMPONENT_LAB_VISUAL_MANIFEST_SCHEMA_VERSION,
        case_count: 1,
        cases: vec![visual_case(&baseline_path, &actual_path, &diff_path)],
    };

    let report = manifest.diff_captures(0);
    assert_eq!(
        report.schema_version,
        COMPONENT_LAB_VISUAL_DIFF_SCHEMA_VERSION
    );
    assert_eq!(report.report_type, COMPONENT_LAB_VISUAL_DIFF_REPORT_TYPE);
    assert!(!report.passed);
    assert_eq!(report.case_count, 1);
    assert_eq!(report.compared_count, 1);
    assert_eq!(report.failed_count, 1);
    assert_eq!(
        report.cases[0].status,
        ComponentLabVisualDiffStatus::Different
    );
    assert_eq!(report.cases[0].changed_pixels, 1);
    assert_eq!(report.cases[0].total_pixels, 4);
    assert_eq!(report.cases[0].max_channel_delta, 255);
    assert!(diff_path.exists());

    let markdown = report.to_markdown_table();
    assert!(markdown.contains(COMPONENT_LAB_VISUAL_DIFF_REPORT_TYPE));
    assert!(markdown.contains("different"));
}

#[test]
fn visual_manifest_diff_passes_with_threshold_and_reports_missing_actual() {
    let temp = tempfile::tempdir().unwrap();
    let baseline_path = temp.path().join("baseline.png");
    let actual_path = temp.path().join("actual.png");
    let missing_actual_path = temp.path().join("missing.png");
    let diff_path = temp.path().join("diff").join("case.png");
    let missing_diff_path = temp.path().join("diff").join("missing.png");

    let baseline = RgbaImage::from_pixel(1, 1, Rgba([8, 8, 8, 255]));
    baseline.save(&baseline_path).unwrap();
    let actual = RgbaImage::from_pixel(1, 1, Rgba([9, 8, 8, 255]));
    actual.save(&actual_path).unwrap();

    let passing_manifest = ComponentLabVisualManifest {
        schema_version: COMPONENT_LAB_VISUAL_MANIFEST_SCHEMA_VERSION,
        case_count: 1,
        cases: vec![visual_case(&baseline_path, &actual_path, &diff_path)],
    };
    let passing_report = passing_manifest.diff_captures(1);
    assert!(passing_report.passed);
    assert_eq!(passing_report.failed_count, 0);
    assert_eq!(
        passing_report.cases[0].status,
        ComponentLabVisualDiffStatus::Passed
    );

    let missing_manifest = ComponentLabVisualManifest {
        schema_version: COMPONENT_LAB_VISUAL_MANIFEST_SCHEMA_VERSION,
        case_count: 1,
        cases: vec![visual_case(
            &baseline_path,
            &missing_actual_path,
            &missing_diff_path,
        )],
    };
    let missing_report = missing_manifest.diff_captures(0);
    assert!(!missing_report.passed);
    assert_eq!(
        missing_report.cases[0].status,
        ComponentLabVisualDiffStatus::MissingActual
    );
}

#[test]
fn stories_include_motion_presets() {
    let registry = builtin_story_registry().unwrap();
    let story = registry.story("ui-kit.button").unwrap();
    assert!(story.motions.iter().any(|motion| motion.id == "system"));
    assert!(
        story
            .motions
            .iter()
            .any(|motion| motion.id == "reduced" && motion.reduced_motion)
    );
}

fn visual_case(
    baseline_path: &std::path::Path,
    actual_path: &std::path::Path,
    diff_path: &std::path::Path,
) -> ComponentLabVisualCase {
    ComponentLabVisualCase {
        capture_id: "fixture".to_string(),
        story_id: "ui-kit.fixture".to_string(),
        renderer_kind: StoryRendererKind::Component,
        viewport_id: "desktop".to_string(),
        viewport_width: 2,
        viewport_height: 2,
        theme_id: "light".to_string(),
        design: "neutral".to_string(),
        reduced_motion: false,
        interactive: false,
        baseline_path: baseline_path.to_string_lossy().to_string(),
        actual_path: actual_path.to_string_lossy().to_string(),
        diff_path: diff_path.to_string_lossy().to_string(),
    }
}

#[test]
fn stories_include_metadata_items() {
    let registry = builtin_story_registry().unwrap();
    let story = registry.story("ui-kit.button").unwrap();
    assert!(story.metadata.iter().any(|item| item.id == "crate"));
    assert!(story.metadata.iter().any(|item| item.id == "story"));
    assert!(
        story
            .metadata
            .iter()
            .any(|item| item.id == "renderer" && item.value == "Component")
    );
    assert!(
        story
            .metadata
            .iter()
            .all(|item| !item.label.trim().is_empty() && !item.value.trim().is_empty())
    );

    let custom = ComponentStory::new("custom.story", "custom", "Custom", "Custom story");
    assert!(
        custom
            .metadata
            .iter()
            .any(|item| item.id == "renderer" && item.value == "Metadata-only")
    );
}

#[test]
fn default_theme_presets_cover_design_languages() {
    let presets = default_theme_presets();
    for language in ["neutral", "apple_hig", "material3", "fluent"] {
        assert!(
            presets.iter().any(|preset| preset.design == language),
            "missing {language} design preset"
        );
    }
}

#[test]
fn designer_story_json_round_trips() {
    let story = builtin_story_registry()
        .unwrap()
        .story("ui-kit.button")
        .unwrap()
        .clone();
    let mut doc = StoryDocument::new(story);
    doc.set_prop_value("label", StoryPropValue::Text("Apply".into()))
        .unwrap();
    let json = serde_json::to_string(&doc).unwrap();
    let parsed: StoryDocument = serde_json::from_str(&json).unwrap();
    assert_eq!(
        parsed.story.props[0].value,
        StoryPropValue::Text("Apply".into())
    );
    assert!(parsed.story.metadata.iter().any(|item| item.id == "story"));
}

#[test]
fn live_preview_reload_loads_story_documents_and_tokens() {
    let tmp = tempfile::tempdir().unwrap();
    let stories_dir = tmp.path().join("stories");
    std::fs::create_dir_all(&stories_dir).unwrap();

    let story = builtin_story_registry()
        .unwrap()
        .story("ui-kit.button")
        .unwrap()
        .clone();
    StoryDocument::new(story)
        .save_story_json(&stories_dir.join("button.story.json"))
        .unwrap();

    let token_path = tmp.path().join("tokens.json");
    std::fs::write(
        &token_path,
        export_design_tokens(DesignTokenFormat::StyleDictionaryJson).unwrap(),
    )
    .unwrap();

    let reload =
        reload_live_preview_state(&stories_dir, &[token_path.clone()], SystemTime::UNIX_EPOCH)
            .unwrap()
            .expect("first load should see files");
    assert_eq!(reload.story_documents.len(), 1);
    assert_eq!(reload.token_reports.len(), 1);
    assert!(reload.token_reports[0].report.passed);

    let unchanged =
        reload_live_preview_state(&stories_dir, &[token_path], reload.latest_modified).unwrap();
    assert!(unchanged.is_none());
}

#[test]
fn latest_rust_source_modified_ignores_target_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(target_dir.join("generated.rs"), "fn generated() {}").unwrap();

    assert_eq!(
        latest_rust_source_modified(tmp.path()).unwrap(),
        SystemTime::UNIX_EPOCH
    );

    let src_dir = tmp.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("lib.rs"), "pub fn real_source() {}").unwrap();

    assert!(latest_rust_source_modified(tmp.path()).unwrap() > SystemTime::UNIX_EPOCH);
}

#[test]
fn conformance_report_to_markdown_is_allocation_efficient() {
    let token_report = DesignTokenValidationReport {
        schema_version: gpui_design_tools::DESIGN_TOKEN_VALIDATION_REPORT_SCHEMA_VERSION,
        report_type: std::borrow::Cow::Borrowed(
            gpui_design_tools::DESIGN_TOKEN_VALIDATION_REPORT_TYPE,
        ),
        passed: true,
        findings: Vec::new(),
        preset_count: 2,
        token_count: 12,
        conformance_markdown: "All tokens passed.".to_string(),
    };
    let finding = ComponentLabConformanceFinding::new(
        "accessibility",
        "missing-label",
        Some("ui-kit.button"),
        "Button needs a label",
    );
    let report = ComponentLabConformanceReport::new(3, &token_report, vec![finding]);
    let markdown = report.to_markdown();
    assert!(markdown.contains("stories: 3"));
    assert!(markdown.contains("tokens: 12"));
    assert!(markdown.contains("missing-label"));
    assert!(markdown.contains("Button needs a label"));
    assert!(!markdown.contains("No component-lab findings"));
}

#[test]
fn ensure_conformance_passed_reports_failures() {
    let token_report = DesignTokenValidationReport {
        schema_version: gpui_design_tools::DESIGN_TOKEN_VALIDATION_REPORT_SCHEMA_VERSION,
        report_type: std::borrow::Cow::Borrowed(
            gpui_design_tools::DESIGN_TOKEN_VALIDATION_REPORT_TYPE,
        ),
        passed: true,
        findings: Vec::new(),
        preset_count: 1,
        token_count: 5,
        conformance_markdown: String::new(),
    };
    let finding = ComponentLabConformanceFinding::new(
        "render",
        "overflow",
        Some("px.line"),
        "Chart overflows mobile width",
    );
    let failing = ComponentLabConformanceReport::new(1, &token_report, vec![finding]);
    assert!(!failing.passed());
    assert!(ensure_component_lab_conformance_passed(&failing).is_err());

    let passing = ComponentLabConformanceReport::new(1, &token_report, Vec::new());
    assert!(passing.passed());
    assert!(ensure_component_lab_conformance_passed(&passing).is_ok());
}
