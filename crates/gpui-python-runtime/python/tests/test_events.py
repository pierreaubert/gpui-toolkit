import unittest
from gpui_toolkit.events import Click, Selection, ValueChange, specialize

class EventTests(unittest.TestCase):
    def test_specializes_semantic_payloads(self):
        click = specialize({"id":"e1","sequence":1,"node_id":"run","event":"click","payload":{"modifiers":["shift"]}})
        self.assertIsInstance(click, Click)
        self.assertEqual(click.event, "click")
        self.assertEqual(click.modifiers, ("shift",))
        selected = specialize({"id":"e2","node_id":"rows","event":"select","payload":{"row_id":"r1"}})
        self.assertIsInstance(selected, Selection)
        self.assertEqual(selected.selected_id, "r1")
        changed = specialize({"id":"e3","node_id":"gain","event":"change","payload":{"value":0.5}})
        self.assertIsInstance(changed, ValueChange)
        self.assertEqual(changed.value, 0.5)
