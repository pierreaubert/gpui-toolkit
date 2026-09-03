import unittest
import importlib.util
import inspect
import json
from array import array
from pathlib import Path

from gpui_toolkit import App, data, px, scene3d as s3, section, ui


class Scene3DTests(unittest.TestCase):
    def test_lines_and_meshes_bind_arraydata_without_inline_geometry(self):
        line_points = data.ArrayData.from_buffer(
            array("f", [0.0, 0.0, 0.0, 1.0, 0.5, 0.0]),
            shape=(2, 3),
            dtype="f32",
            id="scene-line-points",
        )
        vertices = data.ArrayData.from_buffer(
            array("d", [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
            shape=(3, 3),
            dtype="f64",
            id="scene-mesh-vertices",
        )
        indices = data.ArrayData.from_buffer(
            array("I", [0, 1, 2]),
            shape=(1, 3),
            dtype="u32",
            id="scene-mesh-indices",
        )
        scalar = data.ArrayData.from_buffer(
            array("f", [0.0, 0.5, 1.0]),
            shape=(3,),
            dtype="f32",
            id="scene-mesh-scalar",
        )

        lines = s3.lines("path", [s3.line_strip("trace", line_points)]).to_spec()
        mesh = s3.mesh(
            "triangle",
            vertices,
            indices,
            scalar_values=scalar,
        ).to_spec()
        scene = s3.scene("resources", [
            s3.lines("path", [s3.line_strip("trace", line_points)]),
            s3.mesh("triangle", vertices, indices, scalar_values=scalar),
        ]).to_spec()

        self.assertEqual(lines["strips"][0]["points"]["kind"], "array_data")
        self.assertEqual(mesh["vertices"]["kind"], "array_data")
        self.assertEqual(mesh["indices"]["kind"], "array_data")
        self.assertEqual(mesh["scalar_field"]["values"]["kind"], "array_data")
        self.assertNotIn("values", mesh["vertices"])
        self.assertNotIn("values", mesh["indices"])
        self.assertEqual(scene["children"][0]["strips"][0]["points"]["id"], line_points.id)
        self.assertEqual(scene["children"][1]["vertices"]["id"], vertices.id)

        with self.assertRaisesRegex(ValueError, r"shape \[points, 3\]"):
            s3.line_strip("bad", scalar).to_spec()

    def test_scene_convenience_constructors_are_strict(self):
        for constructor in (s3.surface, s3.line_strip, s3.lines, s3.mesh, s3.light, s3.scene):
            self.assertFalse(
                any(
                    parameter.kind is inspect.Parameter.VAR_KEYWORD
                    for parameter in inspect.signature(constructor).parameters.values()
                )
            )
        with self.assertRaises(TypeError):
            s3.surface("field", [[1.0]], unsupported=True)

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
        points = data.Dataset.from_mapping(
            {"x": [1.0, 2.0], "y": [3.0, 4.0]},
            id="scene-test-points",
        )
        app = App(
            title="Demo",
            sections=[
                section(
                    "overview",
                    "Overview",
                    ui.vstack(
                        [
                            ui.heading("Demo"),
                            ui.card([px.scatter("points").data(points).x("x").y("y")]),
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
        self.assertEqual(chart["kind"], "px_chart_v2")
        self.assertEqual(chart["chart"], "scatter")
        self.assertNotIn("values", chart["data"]["source"])

    def test_chart_series_preserve_resource_identity_and_axis_metadata(self):
        response = data.Dataset.from_mapping(
            {
                "frequency": [20.0, 100.0, 20.0, 100.0],
                "level": [-3.0, 1.0, 0.0, 0.0],
                "series": ["Measured", "Measured", "Target", "Target"],
            },
            id="response-series",
        )
        chart = (
            px.line("response").data(response)
            .x("frequency").y("level").series("series")
            .x_label("Frequency (Hz)").y_label("Level (dB)")
            .x_range(20.0, 20_000.0)
            .to_spec()
        )
        self.assertEqual(chart["data"]["source"]["id"], "response-series")
        self.assertEqual(chart["data"]["roles"]["series"], "series")
        self.assertEqual(chart["x_label"], "Frequency (Hz)")

    def test_heatmap_dense_values_remain_outside_ui_ir(self):
        field = data.ArrayData.from_buffer(
            array("d", [0.0, 0.5, 1.0, 0.25]),
            shape=(2, 2),
            dtype="f64",
            id="scene-test-field",
        )
        chart = px.heatmap("field").data(field).aspect_ratio(1.0).to_spec()
        self.assertEqual(chart["data"]["source"]["shape"], [2, 2])
        self.assertEqual(chart["aspect_ratio"], 1.0)
        self.assertNotIn("values", chart["data"]["source"])

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
