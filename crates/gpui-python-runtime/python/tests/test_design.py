import unittest

from gpui_toolkit.design import (
    ConformanceFinding,
    DesignConformanceReport,
    DesignLanguage,
    DesignToken,
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


if __name__ == "__main__":
    unittest.main()
