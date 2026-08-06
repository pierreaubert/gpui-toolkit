import importlib.util
import pathlib
import subprocess
import sys
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("check_python_surface", ROOT / "scripts/check_python_surface.py")
surface = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(surface)


class PythonSurfaceRegistryTests(unittest.TestCase):
    def test_registry_covers_every_first_party_consumer_crate(self):
        data = surface.load_manifest()
        self.assertEqual(surface.validate(data), [])
        inventory = {entry["id"] for entry in data["inventory"]["crate"]}
        self.assertEqual(inventory, surface.first_party_crates())

    def test_normal_registry_check_is_green_before_full_parity_gate(self):
        self.assertEqual(surface.main([]), 0)
        self.assertEqual(surface.main(["--strict"]), 1)

    def test_declared_paths_resolve_without_eval(self):
        data = surface.load_manifest()
        for capability in data["capability"]:
            self.assertIsNotNone(surface.resolve_python_path(capability["python_path"]))

    def test_design_requirement_matrix_exposes_remaining_scope(self):
        data = surface.load_manifest()
        self.assertNotIn("ui.forms", surface.requirements_without_capability(data))
        self.assertNotIn("platform.adapters", surface.requirements_without_capability(data))

    def test_runtime_capability_descriptors_match_manifest(self):
        data = surface.load_manifest()
        self.assertEqual(surface.capability_descriptor_ids(), {entry["id"] for entry in data["capability"]})

    def test_generated_capability_table_is_current(self):
        result = subprocess.run([sys.executable, str(ROOT / "scripts/generate_python_capabilities.py"), "--check"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
