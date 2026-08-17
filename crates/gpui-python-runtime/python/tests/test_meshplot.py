import unittest
import math
import json

from gpui_toolkit import meshplot, ui
from gpui_toolkit.resources import ResourceStore


class MeshPlotTests(unittest.TestCase):
    def setUp(self):
        self.geometry = meshplot.geometry(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], [[0, 1, 2]], id="m"
        )

    def test_frozen_spec_round_trip(self):
        field = meshplot.scalar_field([0.0, 1.0, 2.0], unit="Pa")
        spec = meshplot.plot(self.geometry, field, mode="scalar_fill")
        self.assertEqual(spec.to_spec()["kind"], "mesh_plot")
        self.assertEqual(spec.to_spec()["geometry"]["id"], "m")
        self.assertEqual(spec.to_spec()["field"]["unit"], "Pa")

    def test_ui_helper_preserves_selection_action(self):
        node = ui.mesh_plot(meshplot.plot(self.geometry), selection_action="select")
        self.assertEqual(node.kind, "mesh_plot")
        self.assertEqual(node.to_spec()["selection_action"], "select")

    def test_scalar_field_rejects_unknown_association(self):
        with self.assertRaises(ValueError):
            meshplot.scalar_field([1.0], association="edge")

    def test_resource_geometry_has_explicit_position_and_index_handles(self):
        store = ResourceStore(1024)
        positions = store.put_mesh_array(
            "positions", [[0.0, 0.0, 0.0]], shape=(1, 3), dtype="f64le"
        )
        triangles = store.put_mesh_array(
            "triangles", [[0, 1, 2]], shape=(1, 3), dtype="u32le"
        )
        spec = meshplot.resource_geometry_from_resources(positions, triangles).to_spec()
        self.assertEqual(spec["positions"]["resource_id"], "positions")
        self.assertEqual(spec["positions"]["dtype"], "f64le")
        self.assertEqual(spec["triangles"]["resource_id"], "triangles")
        self.assertEqual(spec["triangles"]["dtype"], "u32le")

    def test_resource_geometry_rejects_legacy_whole_resource_form(self):
        with self.assertRaisesRegex(ValueError, "triangles_resource_id"):
            meshplot.resource_geometry("geometry", 1)

    def test_resource_geometry_rejects_mixed_whole_and_split_handles(self):
        geometry = meshplot.MeshGeometry(
            (),
            (),
            resource_id="geometry",
            generation=1,
            positions_resource_id="positions",
            positions_generation=1,
            triangles_resource_id="triangles",
            triangles_generation=1,
        )
        with self.assertRaisesRegex(ValueError, "whole-geometry"):
            geometry.to_spec()

    def test_resource_ids_and_masks_use_typed_handles(self):
        geometry = meshplot.resource_geometry(
            "positions",
            1,
            triangles_resource_id="triangles",
            triangles_generation=1,
            vertex_ids_resource_id="vertex_ids",
            vertex_ids_generation=1,
            cell_ids_resource_id="cell_ids",
            cell_ids_generation=1,
        )
        field = meshplot.resource_field(
            "values", 1, valid_resource_id="mask", valid_generation=1
        )
        geometry_spec = geometry.to_spec()
        field_spec = field.to_spec()
        self.assertEqual(geometry_spec["vertex_ids"]["dtype"], "u64le")
        self.assertEqual(geometry_spec["cell_ids"]["resource_id"], "cell_ids")
        self.assertEqual(field_spec["valid"]["dtype"], "bool_bytes")

    def test_inline_explicit_mask_accepts_nan_without_nonstandard_json(self):
        field = meshplot.scalar_field(
            [0.0, math.nan, 2.0], valid=[True, False, True]
        )
        spec = field.to_spec()
        json.dumps(spec, allow_nan=False)
        self.assertEqual(spec["values"], [0.0, 0.0, 2.0])
        self.assertEqual(spec["valid"], [True, False, True])

    def test_inline_mask_nan_policy_builds_an_explicit_validity_mask(self):
        field = meshplot.scalar_field([0.0, math.nan, 2.0])
        spec = meshplot.plot(
            self.geometry,
            field,
            mode="scalar_fill",
            missing_value_policy="mask_nan",
        ).to_spec()
        self.assertEqual(spec["field"]["values"], [0.0, 0.0, 2.0])
        self.assertEqual(spec["field"]["valid"], [True, False, True])

    def test_partial_revolve_settings_round_trip(self):
        spec = meshplot.plot(
            self.geometry,
            view="axisymmetric_revolve",
            revolve=meshplot.revolve(
                start_angle=0.25,
                sweep_angle=1.5,
                segments=32,
                end_caps=True,
            ),
        ).to_spec()
        self.assertEqual(
            spec["revolve"],
            {
                "radial": "x",
                "axial": "z",
                "start_angle": 0.25,
                "sweep_angle": 1.5,
                "segments": 32,
                "end_caps": True,
            },
        )

    def test_axes_configuration_round_trip_normalizes_ranges(self):
        spec = meshplot.plot(
            self.geometry,
            axes={
                "horizontal_label": "distance",
                "vertical_label": "height",
                "unit": "m",
                "x_range": (0, 2),
                "y_range": [-1.0, 3.0],
                "show_grid": False,
            },
        ).to_spec()
        self.assertEqual(
            spec["axes"],
            {
                "horizontal_label": "distance",
                "vertical_label": "height",
                "unit": "m",
                "x_range": [0.0, 2.0],
                "y_range": [-1.0, 3.0],
                "show_grid": False,
            },
        )

    def test_axes_configuration_rejects_invalid_values(self):
        for axes in (
            "invalid",
            {"horizontal_label": 1},
            {"x_range": [1.0, 1.0]},
            {"y_range": [float("nan"), 1.0]},
            {"show_grid": 1},
            {"future": True},
        ):
            with self.subTest(axes=axes):
                with self.assertRaises(ValueError):
                    meshplot.plot(self.geometry, axes=axes).to_spec()

    def test_interactions_use_supported_preset_or_explicit_disable(self):
        self.assertEqual(
            meshplot.plot(self.geometry).to_spec()["interactions"],
            ["pan", "zoom", "inspect", "select", "reset", "fit"],
        )
        self.assertEqual(
            meshplot.plot(self.geometry, interactions=[]).to_spec()["interactions"],
            [],
        )
        self.assertEqual(
            meshplot.plot(self.geometry, interactions=["pan"]).to_spec()["interactions"],
            ["pan"],
        )


if __name__ == "__main__":
    unittest.main()
