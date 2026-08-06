import unittest
from gpui_toolkit.accessibility import AriaProps, AriaRole, FocusGroup, FocusDirection
class AccessibilityTests(unittest.TestCase):
 def test_aria_and_focus_specs_are_typed(self):
  self.assertEqual(AriaProps(AriaRole.SLIDER,value_now=1,value_min=0,value_max=2).to_spec()["role"],"slider")
  self.assertEqual(FocusGroup("tabs",FocusDirection.HORIZONTAL).to_spec()["direction"],"horizontal")
 def test_invalid_semantics_fail_early(self):
  with self.assertRaises(ValueError): AriaProps(AriaRole.HEADING,level=7)
  with self.assertRaises(ValueError): FocusGroup("")
if __name__ == "__main__": unittest.main()
