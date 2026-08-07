import hashlib
import json
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path

from qa_apple_simulator_evidence import EvidenceError, create_report, validate_report


def arguments(screenshot: Path, platform: str = "ios") -> Namespace:
    return Namespace(
        screenshot=screenshot,
        platform=platform,
        device_name="iPhone 16e" if platform == "ios" else "Apple TV 4K",
        runtime="iOS 18 6" if platform == "ios" else "tvOS 26 2",
        device_udid="00000000-0000-0000-0000-000000000000",
        bundle_id="org.spinorama.gpui-showcase",
        launch_pid=42,
        unique_colors=128,
        pixel_width=1170,
        pixel_height=2532,
        source_revision="a" * 40,
        source_dirty=False,
        xcode="Xcode 26.4",
        rustc="rustc 1.90.0",
    )


class AppleSimulatorEvidenceTests(unittest.TestCase):
    def test_create_ios_report_retains_manual_accessibility_scope(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            screenshot = Path(directory) / "ios.png"
            screenshot.write_bytes(b"not-empty-png-fixture")
            report = create_report(arguments(screenshot))

            self.assertEqual(report["platform"], "ios")
            self.assertEqual(
                report["pixel_sha256"], hashlib.sha256(screenshot.read_bytes()).hexdigest()
            )
            self.assertIn("VoiceOver", report["manual_required"])
            validate_report(report, screenshot)

    def test_create_tvos_report_retains_remote_focus_scope(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            screenshot = Path(directory) / "tvos.png"
            screenshot.write_bytes(b"not-empty-png-fixture")
            report = create_report(arguments(screenshot, "tvos"))

            self.assertIn("remote-focus-navigation", report["manual_required"])
            validate_report(report, screenshot)

    def test_rejects_near_blank_capture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            screenshot = Path(directory) / "blank.png"
            screenshot.write_bytes(b"not-empty-png-fixture")
            args = arguments(screenshot)
            args.unique_colors = 2

            with self.assertRaisesRegex(EvidenceError, "pixel_unique_colors"):
                create_report(args)

    def test_rejects_tampered_pixel_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            screenshot = Path(directory) / "ios.png"
            screenshot.write_bytes(b"first")
            report = create_report(arguments(screenshot))
            screenshot.write_bytes(b"tampered")

            with self.assertRaisesRegex(EvidenceError, "pixel_sha256"):
                validate_report(report, screenshot)

    def test_report_is_json_serializable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            screenshot = Path(directory) / "ios.png"
            screenshot.write_bytes(b"not-empty-png-fixture")
            report = create_report(arguments(screenshot))

            self.assertEqual(
                json.loads(json.dumps(report))["report_type"],
                "gpui-apple-simulator-smoke",
            )


if __name__ == "__main__":
    unittest.main()
