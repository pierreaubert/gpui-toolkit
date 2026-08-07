import unittest
from gpui_toolkit.effects import ConfirmDialog, EffectResult, EffectStatus, Notification, choose_file, open_url, open_with_system, reveal_path
from gpui_toolkit.app import App, SessionContext
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
 def test_path_effects_are_typed(self):
  cx=Context(); open_with_system(cx,"open","/tmp/result.json"); reveal_path(cx,"reveal","/tmp")
  self.assertEqual(cx.calls[-2:],[("open","open_with_system",{"path":"/tmp/result.json"}),("reveal","reveal_path",{"path":"/tmp"})])
 def test_host_outcomes_are_normalized_to_typed_statuses(self):
  success = EffectResult.from_wire("copy", {"ok": True, "text": "copied"})
  cancelled = EffectResult.from_wire("open", {"ok": True, "cancelled": True})
  unsupported = EffectResult.from_wire("effect", {"ok": False, "error": "unsupported effect: reveal"})
  self.assertTrue(success.ok); self.assertEqual(success.data["text"], "copied")
  with self.assertRaises(TypeError): success.data["text"] = "changed"
  self.assertEqual(cancelled.status, EffectStatus.CANCELLED)
  self.assertEqual(unsupported.status, EffectStatus.UNSUPPORTED)
 def test_app_effect_callback_receives_typed_result(self):
  class Handler(App):
   seen = None
   def on_effect_result(self, request_id, result, context): self.seen = result
  app = Handler(); app._handle_effect_result("open", {"ok": True, "paths": ["model.json"]}, SessionContext())
  self.assertIsInstance(app.seen, EffectResult)
  self.assertEqual(app.seen.status, EffectStatus.SUCCEEDED)
if __name__ == "__main__": unittest.main()
