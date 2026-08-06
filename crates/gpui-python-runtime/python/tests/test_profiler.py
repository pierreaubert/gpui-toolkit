import unittest
from gpui_toolkit.profiler import AllocationBudget, AllocProbe, AllocSnapshot
class ProfilerTests(unittest.TestCase):
 def test_feature_disabled_shape_matches_native_zero_mode(self):
  self.assertEqual(AllocProbe().sample("render"), AllocSnapshot())
 def test_budgets_enforce_both_dimensions(self):
  budget = AllocationBudget("render", 1, 8)
  self.assertTrue(budget.contains(AllocSnapshot(8, 1)))
  with self.assertRaises(AssertionError): budget.assert_contains(AllocSnapshot(9, 1))
if __name__ == "__main__": unittest.main()
