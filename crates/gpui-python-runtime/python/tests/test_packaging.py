import importlib.metadata
import sys
import tomllib
import unittest
from pathlib import Path

import gpui_toolkit


ROOT = Path(__file__).resolve().parents[2]
PYPROJECT = ROOT / "pyproject.toml"
CARGO_MANIFEST = ROOT / "Cargo.toml"


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

    def test_python_package_version_matches_rust_crate_version(self):
        pyproject = tomllib.loads(PYPROJECT.read_text())
        cargo = tomllib.loads(CARGO_MANIFEST.read_text())

        self.assertEqual(pyproject["project"]["version"], cargo["package"]["version"])
        self.assertEqual(gpui_toolkit.__version__, cargo["package"]["version"])

    def test_declared_package_import_surface_is_available(self):
        exports = set(gpui_toolkit.__all__)

        self.assertIn("App", exports)
        self.assertIn("Section", exports)
        self.assertIn("charts", exports)
        self.assertIn("scene3d", exports)
        self.assertIn("ui", exports)
        self.assertIs(gpui_toolkit.App, gpui_toolkit.app.App)
        self.assertTrue(hasattr(gpui_toolkit.scene3d, "SCENE3D_SPEC_SCHEMA_VERSION"))

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


if __name__ == "__main__":
    unittest.main()
