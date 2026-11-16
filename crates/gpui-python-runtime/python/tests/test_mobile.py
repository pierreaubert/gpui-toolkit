"""Python-side mobile generator contracts (manifest, options, preflight)."""

import importlib.util
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

from gpui_toolkit.app import Event, SessionContext
from gpui_toolkit.mobile import MobileManifest, validate_app
from gpui_toolkit.scaffolder import ScaffoldOptions

EXAMPLES = Path(__file__).parents[1] / "examples"


def load_mobile_example():
    path = EXAMPLES / "mobile_signal_lab.py"
    module_spec = importlib.util.spec_from_file_location(path.stem, path)
    assert module_spec is not None
    assert module_spec.loader is not None
    module = importlib.util.module_from_spec(module_spec)
    sys.modules[path.stem] = module
    try:
        module_spec.loader.exec_module(module)
    finally:
        sys.modules.pop(path.stem, None)
    return module

MANIFEST = """
[app]
name = "demo"
title = "Demo"
version = "0.1.0"
bundle_identifier = "com.example.demo"
entry = "main_app.py"

[python]
version = "3.15.2"

[toolkit]
root = "../gpui-toolkit"
"""


class MobileManifestTests(unittest.TestCase):
    def test_manifest_parses_identity_fields(self):
        manifest = MobileManifest.from_toml(MANIFEST)
        self.assertEqual(manifest.bundle_identifier, "com.example.demo")
        self.assertEqual(manifest.entry, "main_app.py")
        self.assertEqual(manifest.python_version, "3.15.2")

    def test_manifest_rejects_missing_fields(self):
        with self.assertRaises(ValueError):
            MobileManifest.from_toml('[app]\nname = "demo"\n')

    def test_scaffold_options_carry_template_and_platform_flags(self):
        spec = ScaffoldOptions(
            "demo", "/tmp", template="python-mobile", no_android=True
        ).to_spec()
        self.assertEqual(spec["template"], "python-mobile")
        self.assertTrue(spec["no_android"])
        self.assertFalse(spec["no_ios"])

    def test_scaffold_options_reject_unknown_template(self):
        with self.assertRaises(ValueError):
            ScaffoldOptions("demo", "/tmp", template="tabs")

    def test_validate_app_returns_the_ir_spec(self):
        with tempfile.TemporaryDirectory() as directory:
            entry = Path(directory) / "main_app.py"
            entry.write_text(
                "from gpui_toolkit import App, section, ui\n\n\n"
                "def build_app():\n"
                "    return App(title='Fixture', sections=[section(\n"
                "        'main', 'Main', ui.text('hello'))])\n",
                encoding="utf-8",
            )
            spec = validate_app(entry)
        self.assertEqual(spec["title"], "Fixture")
        self.assertEqual(len(spec["sections"]), 1)


class MobileSignalLabExampleTests(unittest.TestCase):
    def test_waveform_columns_match_known_samples(self):
        module = load_mobile_example()
        columns = module.waveform_columns(
            amplitude=1.25, cycles=1.0, waveform="triangle", samples=5
        )

        self.assertEqual(columns["x"], [0.0, 0.25, 0.5, 0.75, 1.0])
        self.assertEqual(columns["y"], [0.0, 1.25, 0.0, -1.25, 0.0])
        self.assertEqual(module.waveform_peak(columns), 1.25)

    def test_waveform_rejects_unknown_value(self):
        module = load_mobile_example()
        with self.assertRaisesRegex(ValueError, "unsupported waveform"):
            module.waveform_columns(waveform="sawtooth")

    def test_build_app_declares_touch_sections_and_dataset(self):
        module = load_mobile_example()
        app = module.build_app()
        spec = app.to_spec()

        self.assertEqual(spec["title"], "Mobile Signal Lab")
        self.assertEqual(
            [section["id"] for section in spec["sections"]],
            ["overview", "signal", "settings"],
        )
        self.assertEqual([resource.id for resource in app.resources], ["mobile-wave"])
        json.dumps(spec)

    def test_commit_replaces_dataset_and_preview_only_patches_controls(self):
        module = load_mobile_example()
        app = module.build_app()
        original_generation = app.wave.generation

        preview_output = io.BytesIO()
        app.on_action(
            Event("preview-1", 1, "amplitude", "change", "preview_amplitude", {"value": 0.5}),
            SessionContext(output=preview_output),
        )
        self.assertEqual(app.amplitude, 0.5)
        self.assertEqual(app.wave.generation, original_generation)
        preview_messages = [
            json.loads(line) for line in preview_output.getvalue().splitlines()
        ]
        self.assertEqual([message["type"] for message in preview_messages], ["acknowledged", "patch"])

        commit_output = io.BytesIO()
        app.on_action(
            Event("commit-1", 2, "amplitude", "change", "set_amplitude", {"value": 0.75}),
            SessionContext(output=commit_output),
        )
        self.assertEqual(app.wave.generation, original_generation + 1)
        commit_messages = [
            json.loads(line) for line in commit_output.getvalue().splitlines()
        ]
        patch = next(message for message in commit_messages if message["type"] == "patch")
        peak_op = next(op for op in patch["ops"] if op["id"] == "peak")
        self.assertEqual(peak_op["value"], "0.75")

    def test_invalid_waveform_is_rejected_without_mutating_state(self):
        module = load_mobile_example()
        app = module.build_app()
        output = io.BytesIO()

        app.on_action(
            Event("bad-1", 1, "waveform", "change", "set_waveform", {"value": "sawtooth"}),
            SessionContext(output=output),
        )

        self.assertEqual(app.waveform, "sine")
        messages = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(messages[0]["type"], "rejected")
        self.assertEqual(messages[0]["code"], "invalid_waveform")


if __name__ == "__main__":
    unittest.main()
