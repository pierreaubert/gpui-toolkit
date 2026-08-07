import unittest
from gpui_toolkit.miniapp import MiniAppCommand, MiniAppConfig
from gpui_toolkit import App, section
class MiniAppTests(unittest.TestCase):
 def test_configuration_matches_native_defaults(self):
  spec = MiniAppCommand(MiniAppConfig("Demo"), "root").to_spec()
  self.assertEqual((spec["config"]["width"], spec["config"]["height"]), (900.0, 700.0))
  self.assertEqual(spec["config"]["app_name"], "Demo")
 def test_invalid_window_is_rejected(self):
  with self.assertRaises(ValueError): MiniAppConfig("Demo", 0, 1)
  with self.assertRaises(ValueError): MiniAppConfig("Demo", initial_theme="solarized")
 def test_config_drives_the_host_ir_shell(self):
  config = MiniAppConfig("Speaker Studio", 1280, 840, app_name="Studio", scrollable=False, with_theme=True, with_i18n=True)
  spec = App(sections=[section("root", "Root", {"kind": "text", "id": "content", "text": "Ready"})], miniapp=config).to_spec()
  self.assertEqual((spec["title"], spec["width"], spec["height"]), ("Speaker Studio", 1280, 840))
  self.assertEqual(spec["miniapp"], config.to_spec())
if __name__ == "__main__": unittest.main()
