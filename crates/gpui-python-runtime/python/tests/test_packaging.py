import importlib.metadata
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch
from pathlib import Path

import gpui_toolkit
from gpui_toolkit.app import _host_binary, _negotiate_capabilities, _validate_python_runtime


ROOT = Path(__file__).resolve().parents[2]
PYPROJECT = ROOT / "pyproject.toml"
CARGO_MANIFEST = ROOT / "Cargo.toml"
WORKSPACE_MANIFEST = ROOT.parents[1] / "Cargo.toml"


class PackagingTests(unittest.TestCase):
    def test_pyproject_has_pep621_package_metadata(self):
        metadata = tomllib.loads(PYPROJECT.read_text())["project"]
        setuptools = tomllib.loads(PYPROJECT.read_text())["tool"]["setuptools"]
        package_find = tomllib.loads(PYPROJECT.read_text())["tool"]["setuptools"]["packages"]["find"]

        self.assertEqual(metadata["name"], "gpui-toolkit")
        self.assertEqual(metadata["version"], gpui_toolkit.__version__)
        self.assertEqual(metadata["requires-python"], ">=3.10")
        self.assertEqual(metadata["license"]["text"], "ISC")
        self.assertEqual(metadata["dependencies"], [])
        self.assertEqual(setuptools["package-dir"], {"": "python"})
        self.assertEqual(package_find["where"], ["python"])
        self.assertIn("gpui_toolkit*", package_find["include"])
        self.assertIn("bin/gpui-python-host", tomllib.loads(PYPROJECT.read_text())["tool"]["setuptools"]["package-data"]["gpui_toolkit"])

    def test_python_package_version_matches_rust_crate_version(self):
        pyproject = tomllib.loads(PYPROJECT.read_text())
        cargo = tomllib.loads(CARGO_MANIFEST.read_text())
        workspace = tomllib.loads(WORKSPACE_MANIFEST.read_text())
        version = workspace["workspace"]["package"]["version"]

        self.assertTrue(cargo["package"]["version"].get("workspace"))
        self.assertEqual(pyproject["project"]["version"], version)
        self.assertEqual(gpui_toolkit.__version__, version)

    def test_declared_package_import_surface_is_available(self):
        exports = set(gpui_toolkit.__all__)

        self.assertIn("App", exports)
        self.assertIn("Section", exports)
        self.assertIn("charts", exports)
        self.assertIn("scene3d", exports)
        self.assertIn("ui", exports)
        self.assertIs(gpui_toolkit.App, gpui_toolkit.app.App)
        self.assertTrue(hasattr(gpui_toolkit.scene3d, "SCENE3D_SPEC_SCHEMA_VERSION"))

    def test_native_host_binary_is_a_declared_rust_target(self):
        cargo = tomllib.loads(CARGO_MANIFEST.read_text())
        binaries = {target["name"] for target in cargo.get("bin", [])}
        self.assertIn("gpui-python-host", binaries)

    def test_editable_install_metadata_name_is_consistent_when_installed(self):
        try:
            version = importlib.metadata.version("gpui-toolkit")
        except importlib.metadata.PackageNotFoundError:
            self.skipTest("gpui-toolkit is not installed in this interpreter")

        self.assertEqual(version, gpui_toolkit.__version__)

    def test_package_uses_source_tree_during_unittest_discovery(self):
        package_path = Path(gpui_toolkit.__file__).resolve()

        self.assertEqual(package_path.parent.name, "gpui_toolkit")
        self.assertIn(str(ROOT / "python"), sys.path)

    def test_runtime_fails_before_host_launch_on_unsupported_python(self):
        with patch("gpui_toolkit.app.sys.version_info", (3, 9, 0)):
            with self.assertRaisesRegex(RuntimeError, "Python 3.10"):
                _validate_python_runtime()

    def test_capability_negotiation_fails_before_snapshot_for_missing_requirement(self):
        with self.assertRaisesRegex(RuntimeError, "does not support required"):
            _negotiate_capabilities(["events", "patches"], ["jobs"])
        self.assertEqual(
            _negotiate_capabilities(["events", "patches", "jobs", "effects", "commands"], ["jobs"]),
            ["commands", "effects", "events", "jobs", "patches"],
        )

    def test_configured_missing_host_has_a_clear_error(self):
        with patch.dict("gpui_toolkit.app.os.environ", {"GPUI_TOOLKIT_HOST": "/missing/gpui-python-host"}):
            with self.assertRaisesRegex(RuntimeError, "does not point"):
                _host_binary()

    def test_package_bundled_host_is_preferred_over_path_lookup(self):
        with tempfile.TemporaryDirectory() as temporary:
            package = Path(temporary) / "gpui_toolkit"
            executable_name = "gpui-python-host.exe" if sys.platform == "win32" else "gpui-python-host"
            bundled = package / "bin" / executable_name
            bundled.parent.mkdir(parents=True)
            bundled.write_text("#!/bin/sh\n")
            bundled.chmod(0o755)
            with patch("gpui_toolkit.app.__file__", str(package / "app.py")), patch.dict(
                "gpui_toolkit.app.os.environ", {}, clear=True
            ), patch("gpui_toolkit.app.shutil.which", return_value=None):
                self.assertEqual(Path(_host_binary()).resolve(), bundled.resolve())


if __name__ == "__main__":
    unittest.main()
