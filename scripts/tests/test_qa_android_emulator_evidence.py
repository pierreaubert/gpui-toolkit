import hashlib
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path

from qa_android_emulator_evidence import EvidenceError, create_report, validate_report


def arguments(directory: Path) -> Namespace:
    before = directory / "before.png"
    after = directory / "after.png"
    accessibility = directory / "accessibility.xml"
    before.write_bytes(b"before-pixels")
    after.write_bytes(b"after-pixels")
    accessibility.write_text("<hierarchy><node content-desc='Button'/></hierarchy>")
    return Namespace(
        before=before,
        after=after,
        accessibility=accessibility,
        device_name="Pixel 9",
        serial="emulator-5554",
        api_level=36,
        abi="arm64-v8a",
        package="org.spinorama.gpui.showcase",
        activity="dev.gpui.mobile.GpuiActivity",
        launch_pid=42,
        launch_time_ms=512,
        accessibility_node_count=8,
        accessible_named_node_count=3,
        before_unique_colors=128,
        after_unique_colors=130,
        pixel_width=1080,
        pixel_height=2424,
        source_revision="a" * 40,
        source_dirty=False,
        adb="Android Debug Bridge version 1.0.41",
        java="openjdk 21",
        rustc="rustc 1.90.0",
    )


class AndroidEmulatorEvidenceTests(unittest.TestCase):
    def test_just_recipe_passes_optional_serial_without_empty_array(self) -> None:
        justfile = Path(__file__).parents[2] / "justfile"
        recipe = justfile.read_text().split("qa-android-emulator serial='':", 1)[1]
        recipe = recipe.split("\n\n", 1)[0]
        self.assertIn('"{{serial}}"', recipe)
        self.assertNotIn("args[@]", recipe)

    def test_create_report_captures_touch_and_accessibility(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            args = arguments(Path(directory))
            report = create_report(args)
            self.assertTrue(report["render_changed_after_touch"])
            self.assertEqual(report["accessible_named_node_count"], 3)
            self.assertIn("TalkBack-navigation-and-actions", report["manual_required"])
            validate_report(report, args.before, args.after, args.accessibility)

    def test_rejects_unchanged_pixels(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            args = arguments(Path(directory))
            args.after.write_bytes(args.before.read_bytes())
            with self.assertRaisesRegex(EvidenceError, "render_changed_after_touch"):
                create_report(args)

    def test_rejects_missing_named_accessibility_nodes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            args = arguments(Path(directory))
            args.accessible_named_node_count = 0
            with self.assertRaisesRegex(EvidenceError, "accessible_named_node_count"):
                create_report(args)

    def test_detects_tampered_accessibility_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            args = arguments(Path(directory))
            report = create_report(args)
            self.assertEqual(
                report["accessibility_sha256"],
                hashlib.sha256(args.accessibility.read_bytes()).hexdigest(),
            )
            args.accessibility.write_text("tampered")
            with self.assertRaisesRegex(EvidenceError, "accessibility_sha256"):
                validate_report(report, args.before, args.after, args.accessibility)


if __name__ == "__main__":
    unittest.main()
