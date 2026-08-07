import importlib.util
import json
from pathlib import Path
import unittest


EXAMPLES = Path(__file__).parents[1] / "examples"


def load_example(name: str):
    path = EXAMPLES / name
    module_spec = importlib.util.spec_from_file_location(path.stem, path)
    assert module_spec is not None
    assert module_spec.loader is not None
    module = importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(module)
    return module


class PythonExampleTests(unittest.TestCase):
    def test_spinorama_demo_contains_rust_plot_sections(self):
        module = load_example("spinorama_demo.py")
        spec = module.build_app().to_spec()

        self.assertEqual(
            [section["label"] for section in spec["sections"]],
            ["Overview", "CEA2034", "Horizontal SPL", "Vertical SPL", "Contour", "Surface 3D"],
        )
        self.assertEqual(spec["sections"][1]["content"]["children"][1]["children"][0]["chart"], "line")
        self.assertEqual(spec["sections"][4]["content"]["children"][1]["children"][0]["chart"], "contour")
        self.assertEqual(spec["sections"][5]["content"]["children"][1]["kind"], "scene3d")
        json.dumps(spec)

    def test_surface3d_demo_matches_the_three_rust_modes(self):
        module = load_example("surface3d_demo.py")

        self.assertEqual(module.build_spec("sinc")["id"], "sinc")
        self.assertEqual(module.build_spec("spinorama")["id"], "spinorama")
        self.assertTrue(module.build_spec("saddle")["wireframe"])
        with self.assertRaises(ValueError):
            module.build_spec("unknown")
        json.dumps(module.build_app().to_spec())

    def test_chart_gallery_serializes_all_sections(self):
        module = load_example("chart_gallery.py")
        spec = module.build_app().to_spec()

        self.assertEqual(len(spec["sections"]), 5)
        self.assertEqual(spec["sections"][2]["label"], "Grids")
        self.assertEqual(spec["sections"][3]["label"], "Categories")
        json.dumps(spec)


if __name__ == "__main__":
    unittest.main()
