import unittest
import importlib.util
import json
from pathlib import Path

from gpui_toolkit import App, charts, scene3d as s3, section, ui


class Scene3DTests(unittest.TestCase):
    def test_surface_spec_matches_rust_shape(self):
        spec = s3.surface(
            "dispersion",
            z=[[1.0, 2.0], [3.0, 4.0]],
            x=[20.0, 20000.0],
            y=[-90.0, 90.0],
            colormap="turbo",
            x_log=True,
            wireframe=True,
            camera=s3.orbit(distance=3.5, azimuth=60.0, elevation=25.0),
            interactions=["orbit", "pan", "zoom", "reset"],
        ).to_spec()

        self.assertEqual(spec["schema_version"], s3.SCENE3D_SPEC_SCHEMA_VERSION)
        self.assertEqual(spec["kind"], "surface")
        self.assertEqual(spec["z"], {"values": [1.0, 2.0, 3.0, 4.0], "width": 2, "height": 2})
        self.assertEqual(spec["camera"]["kind"], "orbit")
        self.assertEqual(spec["interactions"], ["orbit", "pan", "zoom", "reset"])

    def test_scene_accepts_future_mesh_nodes(self):
        scene = s3.scene(
            "model",
            children=[
                s3.mesh(
                    "speaker",
                    vertices=[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)],
                    indices=[0, 1, 2],
                    material=s3.material("#88ccff", opacity=0.8),
                )
            ],
        ).to_spec()

        self.assertEqual(scene["schema_version"], s3.SCENE3D_SPEC_SCHEMA_VERSION)
        self.assertEqual(scene["children"][0]["kind"], "mesh")
        self.assertAlmostEqual(scene["children"][0]["material"]["color"]["b"], 1.0)

    def test_mesh_scalar_field_preserves_association_and_metadata(self):
        spec = s3.mesh(
            "pressure",
            vertices=[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)],
            indices=[0, 1, 2],
            scalar_values=[0.0, 0.5, 1.0],
            scalar_location="vertex",
            colormap="turbo",
            scalar_range=(0.0, 1.0),
            scalar_label="Pressure (Pa)",
        ).to_spec()
        self.assertEqual(spec["scalar_field"]["association"], "vertex")
        self.assertEqual(spec["scalar_field"]["range"]["max"], 1.0)
        self.assertEqual(spec["scalar_field"]["label"], "Pressure (Pa)")

    def test_examples_build_json_specs(self):
        examples_dir = Path(__file__).parents[1] / "examples"
        expected = {
            "surface_dispersion.py": "surface",
            "lines_orbit.py": "lines",
            "mesh_scene.py": None,
        }

        for filename, kind in expected.items():
            with self.subTest(filename=filename):
                path = examples_dir / filename
                module_spec = importlib.util.spec_from_file_location(path.stem, path)
                assert module_spec is not None
                assert module_spec.loader is not None
                module = importlib.util.module_from_spec(module_spec)
                module_spec.loader.exec_module(module)

                spec = module.build_spec()
                json.dumps(spec)
                if kind is None:
                    self.assertIn("children", spec)
                else:
                    self.assertEqual(spec["kind"], kind)

    def test_ui_and_chart_helpers_build_app_ir(self):
        app = App(
            title="Demo",
            sections=[
                section(
                    "overview",
                    "Overview",
                    ui.vstack(
                        [
                            ui.heading("Demo"),
                            ui.card([charts.scatter("points", [1.0, 2.0], [3.0, 4.0])]),
                        ]
                    ),
                )
            ],
        )
        spec = app.to_spec()
        json.dumps(spec)

        self.assertEqual(spec["schema_version"], 1)
        self.assertEqual(spec["sections"][0]["content"]["kind"], "vstack")
        chart = spec["sections"][0]["content"]["children"][1]["children"][0]
        self.assertEqual(chart["kind"], "chart")
        self.assertEqual(chart["chart"], "scatter")

    def test_chart_series_preserve_stable_ids_and_axis_metadata(self):
        chart = charts.line(
            "response",
            [20.0],
            [0.0],
            series=[
                charts.Series("measured", [20.0, 100.0], [-3.0, 1.0], label="Measured"),
                charts.Series("target", [20.0, 100.0], [0.0, 0.0], visible=False),
            ],
            x_label="Frequency (Hz)",
            y_label="Level (dB)",
            x_range=(20.0, 20000.0),
        ).to_spec()
        self.assertEqual(chart["series"][0]["id"], "measured")
        self.assertFalse(chart["series"][1]["visible"])
        self.assertEqual(chart["x_label"], "Frequency (Hz)")

    def test_heatmap_coordinates_and_colorbar_metadata_are_preserved(self):
        chart = charts.heatmap(
            "field",
            [0.0, 0.5, 1.0, 0.25],
            2,
            2,
            x=[20.0, 100.0],
            y=[0.0, 30.0],
            color_label="SPL",
            color_unit="dB",
            color_range=(-20.0, 10.0),
            aspect_ratio=1.0,
        ).to_spec()
        self.assertEqual(chart["x"], [20.0, 100.0])
        self.assertEqual(chart["color_label"], "SPL")
        self.assertEqual(chart["color_range"], [-20.0, 10.0])

    def test_typed_table_preserves_cell_types_and_row_identity(self):
        table = ui.table(
            id="jobs",
            columns=[("name", "Name"), ("progress", "Progress")],
            typed_rows=[{"id": "solve-1", "cells": ["Solve", 0.5]}],
            selected_row="solve-1",
            selection_action="select_job",
        ).to_spec()
        self.assertEqual(table["columns"][1]["id"], "progress")
        self.assertEqual(table["typed_rows"][0]["cells"][1], 0.5)
        self.assertEqual(table["selection_action"], "select_job")

    def test_table_window_contract_preserves_stable_rows(self):
        table = ui.table(
            id="results",
            columns=[{"id": "value", "label": "Value", "sortable": True, "width": 120.0}],
            typed_rows=[{"id": "r0", "cells": [0.0]}],
            row_offset=500,
            row_limit=100,
        ).to_spec()
        self.assertEqual(table["row_offset"], 500)
        self.assertEqual(table["row_limit"], 100)
        self.assertTrue(table["columns"][0]["sortable"])

    def test_path_input_preserves_browse_contract_and_recents(self):
        path = ui.path_input(
            id="model",
            label="Speaker model",
            value="speaker.mlg",
            mode="open_file",
            filters=[("Speaker models", ["mlg", "json"])],
            recent_values=["last-model.mlg"],
            must_exist=True,
            action="set-model",
        ).to_spec()
        self.assertEqual(path["kind"], "path_input")
        self.assertEqual(path["filters"][0]["extensions"], ["mlg", "json"])
        self.assertEqual(path["recent_values"], ["last-model.mlg"])
        self.assertTrue(path["must_exist"])

    def test_python_showcase_is_authored_as_app_ir(self):
        showcase_path = Path(__file__).parents[1] / "showcase.py"
        module_spec = importlib.util.spec_from_file_location("python_showcase", showcase_path)
        assert module_spec is not None
        assert module_spec.loader is not None
        module = importlib.util.module_from_spec(module_spec)
        module_spec.loader.exec_module(module)

        spec = module.build_app().to_spec()
        json.dumps(spec)

        self.assertGreaterEqual(len(spec["sections"]), 6)
        self.assertIn("gpui-px Charts", [section["label"] for section in spec["sections"]])
        self.assertTrue(
            any(section["content"]["kind"] == "vstack" for section in spec["sections"])
        )


if __name__ == "__main__":
    unittest.main()
