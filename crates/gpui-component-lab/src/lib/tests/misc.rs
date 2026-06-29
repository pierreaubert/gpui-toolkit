use gpui_design_tools :: { DesignTokenValidationReport } ;
use super::super::register;
use super::super::builtin::builtin_story_registry;
use super::super::component_story::ComponentStory;
use super::super::rendered_preview_conformance::RenderedPreviewConformance;
use super::super::story_conformance::StoryConformance;
use super::super::story_document::StoryDocument;
use super::super::story_metadata_item::StoryMetadataItem;
use super::super::story_prop::StoryProp;
use super::super::story_registry::StoryRegistry;
use super::super::theme_preset::ThemePreset;
use super::super::types::StoryPropValue;
use super::super::validate::validate_component_lab_conformance;
    use super::super::*;

    fn passing_token_report() -> DesignTokenValidationReport {
        DesignTokenValidationReport {
            passed: true,
            findings: Vec::new(),
            preset_count: 4,
            token_count: 128,
            conformance_markdown: "| preset | motion | tokens | status | findings |\n\
                                   | --- | --- | ---: | --- | --- |\n"
                .to_string(),
        }
    }

    #[test]
    fn component_lab_conformance_passes_builtin_registry() {
        let registry = builtin_story_registry().unwrap();
        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(report.passed());
        assert!(report.to_markdown().contains("status: pass"));
    }

    #[test]
    fn component_lab_conformance_reports_choice_without_options() {
        let mut registry = StoryRegistry::new();
        registry
            .register(
                ComponentStory::new("test.choice", "test", "Choice", "Choice story").props([
                    StoryProp::new("mode", "Mode", StoryPropValue::Choice("a".into())),
                ]),
            )
            .unwrap();
        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "choice.options")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "choice.value")
        );
    }

    #[test]
    fn component_lab_conformance_reports_first_party_story_without_renderer() {
        let mut registry = StoryRegistry::new();
        registry
            .register(ComponentStory::new(
                "audio-kit.unrendered",
                "gpui-audio-kit",
                "Unrendered",
                "First-party story without a lab renderer",
            ))
            .unwrap();
        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "renderer.coverage")
        );
    }

    #[test]
    fn component_lab_conformance_reports_bad_story_metadata() {
        let mut registry = StoryRegistry::new();
        registry
            .register(
                ComponentStory::new("test.metadata", "test", "Metadata", "Metadata story")
                    .metadata([
                        StoryMetadataItem::new("owner", "Owner", "Design Systems"),
                        StoryMetadataItem::new("owner", "", ""),
                    ]),
            )
            .unwrap();

        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        for id in ["metadata.duplicate", "metadata.label", "metadata.value"] {
            assert!(
                report.findings.iter().any(|finding| finding.id == id),
                "missing {id}"
            );
        }
    }

    #[test]
    fn component_lab_conformance_reports_unknown_theme_design() {
        let mut registry = StoryRegistry::new();
        let mut story = ComponentStory::new("test.theme", "test", "Theme", "Theme story");
        story.themes = vec![ThemePreset::new("unknown", "Unknown", "bogus", false)];
        registry.register(story).unwrap();

        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "themes.design")
        );
    }

    #[test]
    fn component_lab_conformance_reports_rendered_overflow() {
        let mut registry = StoryRegistry::new();
        let story = ComponentStory::new("test.rendered", "test", "Rendered", "Rendered story")
            .conformance(StoryConformance::display(900.0, 120.0));
        registry.register(story).unwrap();

        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "rendered.overflow")
        );
    }

    #[test]
    fn component_lab_conformance_reports_observed_rendered_overflow() {
        let mut registry = StoryRegistry::new();
        let story = ComponentStory::new(
            "test.observed",
            "test",
            "Observed",
            "Observed rendered story",
        )
        .conformance(StoryConformance {
            rendered: RenderedPreviewConformance::new(160.0, 120.0).observed_bounds(410.0, 180.0),
            ..StoryConformance::display(160.0, 120.0)
        });
        registry.register(story).unwrap();

        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "rendered.overflow")
        );
    }

    #[test]
    fn component_lab_conformance_reports_rendered_bounds_order_failure() {
        let mut registry = StoryRegistry::new();
        let story = ComponentStory::new("test.bounds", "test", "Bounds", "Bounds story")
            .conformance(StoryConformance {
                rendered: RenderedPreviewConformance::new(200.0, 120.0)
                    .observed_bounds(160.0, 120.0),
                ..StoryConformance::display(200.0, 120.0)
            });
        registry.register(story).unwrap();

        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "rendered.bounds_order")
        );
    }

    #[test]
    fn component_lab_conformance_reports_touch_target_failure() {
        let mut registry = StoryRegistry::new();
        let story = ComponentStory::new("test.touch", "test", "Touch", "Touch story").conformance(
            StoryConformance::interactive(160.0, 120.0, 24.0, ["Action"]),
        );
        registry.register(story).unwrap();

        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "touch.target")
        );
    }

    #[test]
    fn component_lab_conformance_reports_touch_target_count_failure() {
        let mut registry = StoryRegistry::new();
        let mut conformance =
            StoryConformance::interactive(160.0, 120.0, 48.0, ["Primary", "Secondary"]);
        conformance.touch_target_count = 1;
        let story = ComponentStory::new("test.touch-count", "test", "Touch", "Touch story")
            .conformance(conformance);
        registry.register(story).unwrap();

        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "touch.targets")
        );
    }

    #[test]
    fn component_lab_conformance_reports_touch_rendered_area_failure() {
        let mut registry = StoryRegistry::new();
        let story =
            ComponentStory::new("test.touch-area", "test", "Touch", "Touch story").conformance(
                StoryConformance::interactive(60.0, 60.0, 48.0, ["Primary", "Secondary"]),
            );
        registry.register(story).unwrap();

        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "touch.rendered_area")
        );
    }

    #[test]
    fn component_lab_conformance_reports_focus_metadata_failure() {
        let mut registry = StoryRegistry::new();
        let mut story = ComponentStory::new("test.focus", "test", "Focus", "Focus story");
        story.conformance.focusable_count = 2;
        story.conformance.focus_labels = vec!["Primary".into()];
        registry.register(story).unwrap();

        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "focus.labels")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "focus.touch_target")
        );
    }

    #[test]
    fn component_lab_conformance_reports_focus_duplicate_and_extra_labels() {
        let mut registry = StoryRegistry::new();
        let mut story = ComponentStory::new("test.focus-extra", "test", "Focus", "Focus story");
        story.conformance =
            StoryConformance::interactive(180.0, 120.0, 48.0, ["Primary", "Primary", "Unused"]);
        story.conformance.focusable_count = 2;
        registry.register(story).unwrap();

        let report = validate_component_lab_conformance(&registry, &[], &passing_token_report());
        assert!(!report.passed());
        for id in ["focus.labels_extra", "focus.labels_duplicate"] {
            assert!(
                report.findings.iter().any(|finding| finding.id == id),
                "missing {id}"
            );
        }
    }

    #[test]
    fn component_lab_conformance_reports_fixed_layout_overflow() {
        let mut registry = StoryRegistry::new();
        let story = ComponentStory::new("test.fixed", "test", "Fixed", "Fixed story");
        registry.register(story.clone()).unwrap();
        let mut doc = StoryDocument::new(story);
        doc.layout = serde_json::json!({
            "viewport": "mobile",
            "constraints": {
                "sizing": "fixed",
                "min_width": 900.0,
                "min_height": 300.0,
                "aspect_ratio": 1.6,
                "padding": 16.0
            }
        });
        let report = validate_component_lab_conformance(&registry, &[doc], &passing_token_report());
        assert!(!report.passed());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.id == "layout.overflow")
        );
    }

    #[test]
    fn component_lab_conformance_reports_bad_builder_layout() {
        let mut registry = StoryRegistry::new();
        let story = ComponentStory::new("test.builder", "test", "Builder", "Builder story");
        registry.register(story.clone()).unwrap();
        let mut doc = StoryDocument::new(story);
        doc.layout = serde_json::json!({
            "builder": {
                "horizontal_align": "middle",
                "vertical_align": "center",
                "overflow": "clip",
                "surface": "paper",
                "gap": 100.0,
                "border": "yes"
            }
        });

        let report = validate_component_lab_conformance(&registry, &[doc], &passing_token_report());
        assert!(!report.passed());
        for id in [
            "builder.horizontal_align",
            "builder.overflow",
            "builder.surface",
            "builder.gap",
            "builder.border",
        ] {
            assert!(
                report.findings.iter().any(|finding| finding.id == id),
                "missing {id}"
            );
        }
    }

