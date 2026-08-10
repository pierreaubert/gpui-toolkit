import unittest

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


if __name__ == "__main__":
    unittest.main()
