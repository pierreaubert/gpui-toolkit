import unittest
from gpui_toolkit.effects import ConfirmDialog, Notification, choose_file, open_url
class Context:
 def __init__(self): self.calls=[]
 def effect(self, request_id, effect, **arguments): self.calls.append((request_id,effect,arguments))
class EffectTests(unittest.TestCase):
 def test_overlay_effects_emit_host_contracts(self):
  cx=Context(); Notification("Done").send(cx,"n"); ConfirmDialog("Delete").send(cx,"c")
  self.assertEqual(cx.calls[0][1],"notification"); self.assertEqual(cx.calls[1][2]["confirm_label"],"Confirm")
 def test_file_and_url_effects_are_typed(self):
  cx=Context(); choose_file(cx,"f",filters=("wav",)); open_url(cx,"u","https://example.test")
  self.assertEqual(cx.calls[0][2]["filters"],["wav"])
  with self.assertRaises(ValueError): open_url(cx,"u","")
if __name__ == "__main__": unittest.main()
