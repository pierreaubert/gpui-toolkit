import unittest
import contextlib
import io
import json
from gpui_toolkit import SessionContext
from gpui_toolkit.commands import CommandResult
from gpui_toolkit.layout import AccessibilityMetadata, Axis, BandToggleRow, ChassisFooter, ChassisHeader, ChassisSection, CollapsePreference, Container, DisplayTier, KnobRow, KnobSize, KnobSlot, LayoutPreferences, LayoutState, LayoutViewport, RatioPreference, ReadoutTileRow, ResetLayout, SetCollapsed, SetRatio, Sizing, Slot, ToggleCollapsed, snapshot_matrix_from_command, solve, solve_matrix, solved_chassis_from_command, solved_layout_from_command, solve_chassis, to_spec
class LayoutDeclarationsTests(unittest.TestCase):
 def test_tree_is_host_serializable(self):
  root = Container("root", Axis.HORIZONTAL, Sizing.flex(), (Slot("panel", Sizing.fixed(100), collapsible=True, collapse_label="Panel", display_tiers=(DisplayTier("full", 80),)),))
  self.assertEqual(to_spec(root)["children"][0]["kind"], "slot")
  self.assertEqual(Axis.HORIZONTAL.cross(), Axis.VERTICAL)
 def test_invalid_layout_declarations_fail_early(self):
  with self.assertRaises(ValueError): Sizing.flex(weight=0)
  with self.assertRaises(ValueError): Slot("x", Sizing.flex(), collapsible=True)
 def test_layout_state_reduces_to_solver_preferences(self):
  state = LayoutState()
  state.apply(SetRatio("panel", Axis.HORIZONTAL, .4))
  state.apply(SetCollapsed("panel", True))
  self.assertEqual(state.ratio_for("panel", Axis.HORIZONTAL), .4)
  self.assertTrue(state.is_collapsed("panel"))
  state.apply(ToggleCollapsed("panel"))
  self.assertFalse(state.is_collapsed("panel"))
  self.assertEqual(state.preferences().ratios[0].ratio, .4)
  state.apply(ResetLayout())
  self.assertEqual(state.preferences(), LayoutPreferences())
 def test_chassis_solver_command_and_result_are_typed(self):
  output = io.StringIO()
  with contextlib.redirect_stdout(output):
   solve_chassis(SessionContext(), "solve", 100, (
    ChassisSection("main", 80, 120, 1, "01", "Main", rows=(KnobRow("controls", (KnobSlot("gain", 2, "Gain", KnobSize.SM, True),)), BandToggleRow("enabled", "Enabled"))),
    ChassisSection("side", 50, 80, 0, "02", "Meter", rows=(ReadoutTileRow("level", "Level"),)),
   ), ChassisHeader("SOTF", "Plugin", "Subtitle"), ChassisFooter(("48 kHz", "1.2 ms"), "build 1"))
  command = json.loads(output.getvalue())
  self.assertEqual(command["command"], "builder.solve_chassis")
  self.assertEqual(command["arguments"]["sections"][0]["rows"][0]["knobs"][0]["size"], "sm")
  self.assertEqual(command["arguments"]["footer"]["ticks"], ["48 kHz", "1.2 ms"])
  solved = solved_chassis_from_command(CommandResult.from_wire("solve", {"ok": True, "sections": [
   {"id": "main", "width": 100, "visible": True}, {"id": "side", "width": 0, "visible": False},
  ]}))
  self.assertTrue(solved[0].visible)
  self.assertFalse(solved[1].visible)
 def test_recursive_solver_command_and_reports_are_typed(self):
  root = Container("root", Axis.HORIZONTAL, Sizing.flex(), (
   Slot("panel", Sizing.fractional(.25, 50, 200), collapsible=True, collapse_label="Panel"),
   Container("content", Axis.VERTICAL, Sizing.flex(), (Slot("body", Sizing.text_measured("hello", 16), display_tiers=(DisplayTier("full", 40),)),)),
  ))
  output = io.StringIO()
  preferences = LayoutPreferences((RatioPreference("panel", Axis.HORIZONTAL, .5),), (CollapsePreference("panel", False),))
  with contextlib.redirect_stdout(output): solve(SessionContext(), "tree", root, 400, 300, preferences, 7, (AccessibilityMetadata("panel", "region", "Panel", "Resizable panel"),))
  command = json.loads(output.getvalue())
  self.assertEqual(command["command"], "builder.solve")
  self.assertEqual(command["arguments"]["root"]["children"][1]["children"][0]["sizing"]["kind"], "text")
  self.assertEqual(command["arguments"]["preferences"]["ratios"][0]["ratio"], .5)
  self.assertEqual(command["arguments"]["accessibility"][0]["label"], "Panel")
  solved = solved_layout_from_command(CommandResult.from_wire("tree", {
   "ok": True,
   "solved": {"id": "root", "width": 400, "height": 300, "visible": True, "active_tier": None, "collapse_label": None, "resolved_axis": "horizontal", "children": [
    {"id": "panel", "width": 200, "height": 300, "visible": True, "active_tier": None, "collapse_label": "Panel", "resolved_axis": None, "children": []},
   ]},
   "validation": {"clean": True, "error_count": 0, "warning_count": 0, "issues": [], "report": "layout validation: clean\n"},
   "inspection": {"declaration_report": "layout inspection:\n", "solved_report": "solved inspection:\n"},
   "debug": {"report": "root size=400x300 visible\n", "warnings": []},
   "collapsed_tabs": [{"id": "hidden", "label": "Hidden"}],
   "accessibility": {"id": "root", "role": "group", "label": None, "description": None, "visible": True, "collapsed": False, "active_tier": None, "children": [
    {"id": "panel", "role": "region", "label": "Panel", "description": "Resizable panel", "visible": True, "collapsed": False, "active_tier": None, "children": []},
   ]},
  }))
  self.assertEqual(solved.root.find("panel").width, 200)
  self.assertTrue(solved.validation.clean)
  self.assertIn("layout inspection", solved.inspection.declaration_report)
  self.assertEqual(solved.collapsed_tabs[0].label, "Hidden")
  self.assertEqual(solved.accessibility.find("panel").description, "Resizable panel")
 def test_snapshot_matrix_and_retained_results_are_typed(self):
  root = Container("root", Axis.HORIZONTAL, Sizing.flex(), (Slot("body", Sizing.flex()),))
  output = io.StringIO()
  with contextlib.redirect_stdout(output): solve_matrix(SessionContext(), "matrix", root, (LayoutViewport("wide", 800, 600), LayoutViewport("narrow", 320, 480)))
  command = json.loads(output.getvalue())
  self.assertEqual(command["command"], "builder.solve_matrix")
  self.assertTrue(command["arguments"]["include_retained"])
  node = {"id": "root", "width": 800, "height": 600, "visible": True, "active_tier": None, "collapse_label": None, "resolved_axis": "horizontal", "children": []}
  matrix = snapshot_matrix_from_command(CommandResult.from_wire("matrix", {
   "ok": True,
   "snapshots": [{"label": "wide", "width": 800, "height": 600, "root": node, "visible_ids": ["root"], "collapsed_labels": [], "active_tiers": [], "resolved_axes": ["root:horizontal"]}],
   "retained_snapshots": [{"label": "wide", "width": 800, "height": 600, "root": node}],
   "report": "## wide", "markdown": "| viewport |",
  }))
  self.assertEqual(matrix.snapshots[0].visible_ids, ("root",))
  self.assertEqual(matrix.retained_snapshots[0].root.width, 800)
if __name__ == "__main__": unittest.main()
