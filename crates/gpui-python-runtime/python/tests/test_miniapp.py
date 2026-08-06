import unittest
from gpui_toolkit.miniapp import MiniAppCommand, MiniAppConfig
class MiniAppTests(unittest.TestCase):
 def test_configuration_matches_native_defaults(self):
  spec = MiniAppCommand(MiniAppConfig("Demo"), "root").to_spec()
  self.assertEqual((spec["config"]["width"], spec["config"]["height"]), (900.0, 700.0))
  self.assertEqual(spec["config"]["app_name"], "Demo")
 def test_invalid_window_is_rejected(self):
  with self.assertRaises(ValueError): MiniAppConfig("Demo", 0, 1)
if __name__ == "__main__": unittest.main()
