import unittest

from gpui_toolkit.commands import CommandResult
from gpui_toolkit.scaffolder import ScaffoldOptions, ScaffoldPreview


class ScaffolderContractTests(unittest.TestCase):
    def test_non_destructive_request_is_serializable(self):
        self.assertTrue(ScaffoldOptions("demo", "/tmp").to_spec()["dry_run"])

    def test_name_cannot_escape_target_directory(self):
        for name in ("", "../demo", "a/b"):
            with self.assertRaises(ValueError):
                ScaffoldOptions(name, "/tmp")

    def test_preview_returns_the_native_file_plan(self):
        preview = ScaffoldPreview.from_command(CommandResult.from_wire("preview", {
            "ok": True, "app_dir": "/tmp/demo", "package_name": "demo", "title": "Demo",
            "files": ["/tmp/demo/Cargo.toml", "/tmp/demo/src/main.rs"],
        }))
        self.assertEqual(preview.app.package_name, "demo")
        self.assertEqual(preview.files[1].name, "main.rs")
