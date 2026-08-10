import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch


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

    def test_mesh_plot_examples_build_versioned_specs(self):
        inline = load_example("mesh_plot_demo.py")
        revolve = load_example("mesh_plot_revolve_demo.py")
        resource = load_example("mesh_plot_resource_demo.py")

        inline_spec = inline.build_app().to_spec()
        revolve_spec = revolve.build_app().to_spec()
        inline_plot = inline_spec["sections"][0]["content"]
        revolve_plot = revolve_spec["sections"][0]["content"]
        self.assertEqual(inline_plot["kind"], "mesh_plot")
        self.assertEqual(inline_plot["spec"]["schema_version"], 1)
        self.assertTrue(inline_plot["spec"]["equal_aspect"])
        self.assertEqual(revolve_plot["spec"]["view"], "axisymmetric_revolve")
        resource_spec = resource.build_app().to_spec()
        resource_plot = resource_spec["sections"][0]["content"]
        self.assertEqual(resource_plot["spec"]["geometry"]["positions"]["resource_id"], "mesh-positions")
        self.assertEqual(resource_plot["spec"]["geometry"]["triangles"]["dtype"], "u32le")
        self.assertEqual(resource_plot["spec"]["field"]["valid"]["dtype"], "bool_bytes")
        self.assertEqual(resource_plot["selection_action"], "resource_mesh_selected")
        json.dumps(inline_spec)
        json.dumps(revolve_spec)
        json.dumps(resource_spec)

    def test_resource_mesh_qa_variables_launch_native_host(self):
        resource = load_example("mesh_plot_resource_demo.py")
        qa_variables = (
            "GPUI_TOOLKIT_QA_CLOSE_AFTER_SECS",
            "GPUI_TOOLKIT_QA_SELECTION_LOG",
            "GPUI_TOOLKIT_QA_AUTO_SELECT",
            "GPUI_TOOLKIT_QA_HOST_SELECTION_LOG",
            "GPUI_TOOLKIT_QA_POINTER_TRACE",
            "GPUI_TOOLKIT_QA_HIT_TRACE",
            "GPUI_TOOLKIT_QA_INNER_HIT_TRACE",
            "GPUI_TOOLKIT_QA_RENDER_TRACE",
            "GPUI_TOOLKIT_QA_LIVE_HIT_TRACE",
            "GPUI_TOOLKIT_QA_POINTER_POINTS",
        )
        for variable in qa_variables:
            with self.subTest(variable=variable):
                with patch.dict("os.environ", {variable: "1"}, clear=True):
                    self.assertTrue(resource._should_run_native_host())


if __name__ == "__main__":
    unittest.main()
