import unittest
import json

from gpui_toolkit.commands import CommandResult

from gpui_toolkit.design import (
    ConformanceFinding,
    DesignConformanceReport,
    DesignLanguage,
    DesignPlatform,
    DesignSystemSnapshot,
    DesignToken,
    CornerRadii,
    CornerRadiusStyle,
    SpacingRules,
    InteractionRules,
    ElevationRules,
    AnimationRules,
    TypographyRules,
    LayoutThresholds,
    AudioControlRules,
    ToggleVariant,
    LabelPosition,
    GroupSeparatorStyle,
    MotionSpec,
    export_from_command,
    handoff_from_command,
    import_from_command,
    validation_from_command,
    request_token_validation,
    reports_from_command,
)


class DesignDeclarationTests(unittest.TestCase):
    def test_language_ids_and_platform_aliases_match_rust(self):
        self.assertEqual(DesignLanguage.parse("gtk"), DesignLanguage.ADWAITA)
        self.assertEqual(DesignLanguage.MATERIAL3.label, "Material 3")
        with self.assertRaisesRegex(ValueError, "unknown"):
            DesignLanguage.parse("unknown")

    def test_style_dictionary_tokens_are_validated_and_serialized(self):
        token = DesignToken("color.accent", ("color", "accent"), "#ff00ff", "color")
        self.assertEqual(DesignToken.from_spec(token.to_spec()), token)
        with self.assertRaises(ValueError):
            DesignToken("wrong", ("color", "accent"), "#ff00ff", "color")

    def test_reports_and_motion_values_remain_typed(self):
        self.assertTrue(DesignConformanceReport().passed)
        report = DesignConformanceReport((ConformanceFinding("touch-target", "too small"),))
        self.assertFalse(report.passed)
        with self.assertRaises(ValueError):
            MotionSpec(-1, 0, 0, False, False)

    def test_full_native_design_system_snapshot_validates_rules(self):
        system = DesignSystemSnapshot(
            DesignLanguage.NEUTRAL, DesignPlatform.LINUX,
            CornerRadii(4, 8, 12, 16, CornerRadiusStyle.CIRCULAR), SpacingRules(4, 12, 8, 8, 16, 12),
            InteractionRules(32, 1, 2, 2), ElevationRules(0, 4, 16, .15, 2),
            AnimationRules(200, 100, 400, False, 170, 26), TypographyRules("System", False, 14, 12, 18),
            LayoutThresholds(600, 480, 360, 320, 280, 800, 32, 24), AudioControlRules(-135, 270, 4, 32, 1, (2, 3, 4)),
            ToggleVariant.CAPSULE, LabelPosition.ABOVE, GroupSeparatorStyle.DIVIDER,
        )
        self.assertEqual(system.platform, DesignPlatform.LINUX)
        with self.assertRaises(ValueError):
            SpacingRules(float("nan"), 0, 0, 0, 0, 0)

    def test_native_style_dictionary_export_and_import_results(self):
        wire = {"presets": [{"preset_id": "neutral", "tokens": [{"name": "color.accent", "path": ["color", "accent"], "value": "#336699", "token_type": "color"}]}]}
        exported = export_from_command(CommandResult.from_wire("export", {"ok": True, "output": json.dumps(wire)}))
        self.assertEqual((exported.presets[0].preset_id, exported.presets[0].tokens[0].path), ("neutral", ("color", "accent")))
        imported = import_from_command(CommandResult.from_wire("import", {"ok": True, "preset_count": 1, "token_count": 1, "raw": wire}))
        self.assertEqual((imported.preset_count, imported.token_count), (1, 1))

    def test_native_validation_and_handoff_reports(self):
        validation = validation_from_command(CommandResult.from_wire("validation", {"ok": True, "report": {"schema_version": 1, "report_type": "gpui-design-token-validation", "passed": True, "findings": [], "preset_count": 4, "token_count": 200, "conformance_markdown": "# Conformance"}}))
        self.assertTrue(validation.passed)
        handoff = handoff_from_command(CommandResult.from_wire("handoff", {"ok": True, "report": {"schema_version": 1, "report_type": "gpui-design-tooling-handoff", "crate_name": "gpui-design-tools", "crate_version": "1.0.0", "items": [{"id": "tokens", "title": "Tokens", "artifact_type": "command", "path_or_command": "tool", "status": "implemented", "release_evidence": "tests", "remaining_gap": "none"}, {"id": "figma", "title": "Figma", "artifact_type": "external", "path_or_command": "figma", "status": "external-gate", "release_evidence": "contract", "remaining_gap": "credentials"}]}}))
        self.assertEqual(handoff.item("tokens").status, "implemented")
        self.assertEqual([item.id for item in handoff.blocking_entries], ["figma"])

    def test_validation_request_targets_native_design_command(self):
        class Context:
            def command(self, request_id, command, **arguments):
                self.value = (request_id, command, arguments)
        context = Context()
        request_token_validation(context, "validate", "{}", render_markdown=True)
        self.assertEqual(context.value[1], "design.tokens")
        self.assertEqual(context.value[2]["operation"], "validate")

    def test_native_design_documentation_and_release_reports(self):
        preset = {"preset_id": "apple-hig", "label": "Apple HIG", "language": "AppleHig", "token_count": 1, "grid_unit": 4, "min_touch_target": 44, "base_size": 13, "corner_style": "Continuous", "motion_duration_ms": 200, "reduced_motion_duration_ms": 0}
        case = {"preset_id": "apple-hig", "reduced_motion": False, "report": {"findings": []}, "motion": {"duration_ms": 200, "fast_ms": 100, "slow_ms": 300, "prefer_spring": True}, "token_count": 1}
        documentation = {"schema_version": 1, "report_type": "gpui-design-documentation", "presets": [preset], "conformance": {"cases": [case]}, "markdown": "# Design"}
        token = {"name": "color.accent", "path": ["color", "accent"], "value": "#336699", "token_type": "color"}
        release = {"schema_version": 1, "report_type": "gpui-design-release-presentation", "documentation_report_type": "gpui-design-documentation", "documentation_report": documentation, "assets": [{"id": "apple-hig-screenshot", "title": "Screenshot", "kind": "PresetScreenshot", "path": "shot.png", "status": "CaptureRequired", "release_note_use": "Visual proof"}], "release_notes_markdown": "# Release"}
        reports = reports_from_command(CommandResult.from_wire("reports", {"ok": True, "tokens": {"presets": [{"preset_id": "apple-hig", "tokens": [token]}]}, "documentation": documentation, "release": release}))
        self.assertTrue(reports.documentation.passed)
        self.assertEqual(reports.tokens.presets[0].tokens[0].name, "color.accent")
        self.assertEqual([asset.id for asset in reports.release.blocking_assets], ["apple-hig-screenshot"])


if __name__ == "__main__":
    unittest.main()
