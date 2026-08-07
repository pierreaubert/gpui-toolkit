import json
import tempfile
import unittest
from pathlib import Path

from qa_desktop_accessibility import CHECKS, REPORT_TYPE, main, markdown, report, validate_contracts


class DesktopAccessibilityEvidenceTests(unittest.TestCase):
    def test_repository_contracts_are_current(self):
        validate_contracts(Path(__file__).resolve().parents[2])

    def test_report_is_deterministic_and_keeps_screen_readers_manual(self):
        first = json.dumps(report(), sort_keys=True)
        second = json.dumps(report(), sort_keys=True)
        self.assertEqual(first, second)
        body = json.loads(first)
        self.assertEqual(body["report_type"], REPORT_TYPE)
        self.assertTrue(body["automated_release_ready"])
        self.assertEqual(body["native_screen_reader_qa"], "manual-required")
        self.assertEqual(len(body["checks"]), len(CHECKS))

    def test_report_covers_every_required_desktop_dimension(self):
        ids = {check["id"] for check in CHECKS}
        self.assertEqual(
            ids,
            {
                "pointer-activation",
                "keyboard-activation-navigation",
                "focus-order-restoration",
                "disabled-state",
                "accessible-names-actions",
                "native-adapter-parity",
                "reduced-motion",
                "high-contrast",
            },
        )

    def test_markdown_names_limitations(self):
        text = markdown(report())
        self.assertIn("Native adapter payload parity", text)
        self.assertIn("manual-required", text)
        self.assertIn("does not claim VoiceOver", text)


if __name__ == "__main__":
    unittest.main()
