import unittest
from gpui_toolkit.events import Click, Selection, ValueChange, Viewport, specialize

class EventTests(unittest.TestCase):
    def test_specializes_semantic_payloads(self):
        click = specialize({"id":"e1","sequence":1,"node_id":"run","event":"click","payload":{"modifiers":["shift"]}})
        self.assertIsInstance(click, Click)
        self.assertEqual(click.event, "click")
        self.assertEqual(click.modifiers, ("shift",))
        selected = specialize({"id":"e2","node_id":"rows","event":"select","payload":{"row_id":"r1"}})
        self.assertIsInstance(selected, Selection)

        viewport = specialize({
            "id": "e4", "sequence": 4, "node_id": "chart",
            "event": "viewport_change", "payload": {"x": [1, 2], "y": [3, 4]},
        })
        self.assertIsInstance(viewport, Viewport)
        self.assertEqual(viewport.x_range, (1.0, 2.0))
        self.assertEqual(viewport.y_range, (3.0, 4.0))
        surface_viewport = specialize({
            "id": "surface-camera", "node_id": "surface", "event": "viewport_change",
            "payload": {"camera": {
                "distance": 3.5, "azimuth": 60.0, "elevation": 25.0,
                "target": [0.0, 0.5, 0.0],
            }},
        })
        self.assertIsInstance(surface_viewport, Viewport)
        self.assertEqual(surface_viewport.camera_distance, 3.5)
        self.assertEqual(surface_viewport.camera_angles, (60.0, 25.0))
        self.assertEqual(surface_viewport.camera_target, (0.0, 0.5, 0.0))
        self.assertEqual(selected.selected_id, "r1")
        linked = specialize({
            "id": "e5", "node_id": "treemap", "event": "selection_change",
            "payload": {"keys": ["low", "high"], "row_id": "low"},
        })
        self.assertIsInstance(linked, Selection)
        self.assertEqual(linked.selected_keys, ("low", "high"))
        mesh_selected = specialize({
            "id": "e-mesh", "node_id": "plot", "event": "select", "payload": {
                "plot_id": "pressure-field", "mesh_id": "baffle", "cell_index": 4,
                "cell_id": 99, "vertex_id": 12, "world_position": [1, 2, 3],
                "displayed_value": 42.5, "field_id": "pressure",
            },
        })
        self.assertEqual(mesh_selected.plot_id, "pressure-field")
        self.assertEqual(mesh_selected.mesh_id, "baffle")
        self.assertEqual(mesh_selected.cell_index, 4)
        self.assertEqual(mesh_selected.cell_id, 99)
        self.assertEqual(mesh_selected.vertex_id, 12)
        self.assertEqual(mesh_selected.world_position, (1.0, 2.0, 3.0))
        self.assertEqual(mesh_selected.displayed_value, 42.5)
        self.assertEqual(mesh_selected.field_id, "pressure")
        changed = specialize({"id":"e3","node_id":"gain","event":"change","payload":{"value":0.5}})
        self.assertIsInstance(changed, ValueChange)
        self.assertEqual(changed.value, 0.5)
