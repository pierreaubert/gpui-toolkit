import json
import tempfile
import unittest
from pathlib import Path

from qa_native_ui_evidence import (
    EvidenceError,
    annotate_pixel_evidence,
    verify_pixel_evidence,
)


def smoke_report() -> dict[str, object]:
    return {
        "schema_version": 3,
        "report_type": "gpui-native-smoke",
        "crate": "gpui-builder",
        "platform": "macos",
        "window_opened": True,
        "render_invoked": True,
        "render_count": 2,
        "state_transition": "collapse-sidebar",
        "state_transition_verified": True,
        "interaction_scope": ["window-open", "render", "collapse-sidebar"],
        "pixel_capture": False,
    }


class NativeUiEvidenceTests(unittest.TestCase):
    def test_annotation_records_validated_pixel_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / "smoke.json"
            screenshot = root / "builder.png"
            artifact.write_text(json.dumps(smoke_report()), encoding="utf-8")
            screenshot.write_bytes(b"not-empty")

            report = annotate_pixel_evidence(
                artifact,
                screenshot,
                128,
                "macos-window",
                "macos",
            )

            self.assertTrue(report["pixel_capture"])
            self.assertEqual(report["pixel_artifact"], "builder.png")
            self.assertEqual(report["pixel_unique_colors"], 128)
            verify_pixel_evidence(artifact, screenshot, "macos")

    def test_annotation_rejects_near_uniform_screenshot(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / "smoke.json"
            screenshot = root / "builder.png"
            artifact.write_text(json.dumps(smoke_report()), encoding="utf-8")
            screenshot.write_bytes(b"not-empty")

            with self.assertRaisesRegex(EvidenceError, "near-uniform"):
                annotate_pixel_evidence(
                    artifact,
                    screenshot,
                    15,
                    "macos-window",
                    "macos",
                )

    def test_verification_rejects_unproven_render(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / "smoke.json"
            screenshot = root / "builder.png"
            report = smoke_report()
            report["render_count"] = 1
            report["pixel_capture"] = True
            report["pixel_artifact"] = screenshot.name
            report["pixel_unique_colors"] = 128
            report["pixel_capture_transport"] = "macos-window"
            artifact.write_text(json.dumps(report), encoding="utf-8")
            screenshot.write_bytes(b"not-empty")

            with self.assertRaisesRegex(EvidenceError, "render_count"):
                verify_pixel_evidence(artifact, screenshot, "macos")


if __name__ == "__main__":
    unittest.main()
