import importlib.util
import tempfile
from unittest.mock import patch
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "sync_versions.py"
SPEC = importlib.util.spec_from_file_location("sync_versions", SCRIPT)
assert SPEC and SPEC.loader
sync_versions = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(sync_versions)


class SyncVersionsTests(unittest.TestCase):
    def test_validate_version(self):
        sync_versions.validate_version("1.2.3")
        sync_versions.validate_version("1.2.3-rc.1")
        with self.assertRaises(ValueError):
            sync_versions.validate_version("v1.2.3")

    def test_check_rejects_tag_mismatch(self):
        with self.assertRaises(ValueError):
            sync_versions.check(expected="0.9.15", tag="v0.9.14")

    def test_synchronize_updates_all_manifests(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = (
                root / "Cargo.toml",
                root / "pyproject.toml",
                root / "runtime-pyproject.toml",
                root / "__init__.py",
            )
            paths[0].write_text("[workspace.package]\nversion = \"1.0.0\"\n", encoding="utf-8")
            paths[1].write_text("[project]\nversion = \"1.0.0\"\n", encoding="utf-8")
            paths[2].write_text("[project]\nversion = \"1.0.0\"\n", encoding="utf-8")
            paths[3].write_text('__version__ = "1.0.0"\n', encoding="utf-8")
            with patch.object(sync_versions, "FILES", paths), patch.object(
                sync_versions, "ROOT", root
            ):
                sync_versions.synchronize("9.8.7")
                self.assertEqual(sync_versions.check(), "9.8.7")


if __name__ == "__main__":
    unittest.main()
