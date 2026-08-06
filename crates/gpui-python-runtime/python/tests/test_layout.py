import unittest
from gpui_toolkit.layout import Axis, Container, DisplayTier, Sizing, Slot, to_spec
class LayoutDeclarationsTests(unittest.TestCase):
 def test_tree_is_host_serializable(self):
  root = Container("root", Axis.HORIZONTAL, Sizing.flex(), (Slot("panel", Sizing.fixed(100), collapsible=True, collapse_label="Panel", display_tiers=(DisplayTier("full", 80),)),))
  self.assertEqual(to_spec(root)["children"][0]["kind"], "slot")
  self.assertEqual(Axis.HORIZONTAL.cross(), Axis.VERTICAL)
 def test_invalid_layout_declarations_fail_early(self):
  with self.assertRaises(ValueError): Sizing.flex(weight=0)
  with self.assertRaises(ValueError): Slot("x", Sizing.flex(), collapsible=True)
if __name__ == "__main__": unittest.main()
