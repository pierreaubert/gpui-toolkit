import unittest

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


if __name__ == "__main__":
    unittest.main()
