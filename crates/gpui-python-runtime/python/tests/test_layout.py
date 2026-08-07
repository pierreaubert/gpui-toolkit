import unittest
import contextlib
import io
import json
from gpui_toolkit import SessionContext
from gpui_toolkit.commands import CommandResult
from gpui_toolkit.layout import Axis, ChassisSection, Container, DisplayTier, Sizing, Slot, solved_chassis_from_command, solve_chassis, to_spec
class LayoutDeclarationsTests(unittest.TestCase):
 def test_tree_is_host_serializable(self):
  root = Container("root", Axis.HORIZONTAL, Sizing.flex(), (Slot("panel", Sizing.fixed(100), collapsible=True, collapse_label="Panel", display_tiers=(DisplayTier("full", 80),)),))
  self.assertEqual(to_spec(root)["children"][0]["kind"], "slot")
  self.assertEqual(Axis.HORIZONTAL.cross(), Axis.VERTICAL)
 def test_invalid_layout_declarations_fail_early(self):
  with self.assertRaises(ValueError): Sizing.flex(weight=0)
  with self.assertRaises(ValueError): Slot("x", Sizing.flex(), collapsible=True)
 def test_chassis_solver_command_and_result_are_typed(self):
  output = io.StringIO()
  with contextlib.redirect_stdout(output):
   solve_chassis(SessionContext(), "solve", 100, (ChassisSection("main", 80, 120, 1), ChassisSection("side", 50, 80, 0)))
  self.assertEqual(json.loads(output.getvalue())["command"], "builder.solve_chassis")
  solved = solved_chassis_from_command(CommandResult.from_wire("solve", {"ok": True, "sections": [
   {"id": "main", "width": 100, "visible": True}, {"id": "side", "width": 0, "visible": False},
  ]}))
  self.assertTrue(solved[0].visible)
  self.assertFalse(solved[1].visible)
if __name__ == "__main__": unittest.main()
