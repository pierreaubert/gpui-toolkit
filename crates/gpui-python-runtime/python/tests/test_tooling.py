import unittest
from gpui_toolkit.tooling import DesignTokenOperation, DesignTokenValidationReport
class ToolingDeclarationsTests(unittest.TestCase):
 def test_host_operation_and_report_contract(self):
  self.assertEqual(DesignTokenOperation("validate", input="{}").to_spec()["operation"], "validate")
  self.assertTrue(DesignTokenValidationReport(1, "report", True, (), 1, 2, "").passed)
 def test_contract_rejects_invalid_shapes(self):
  with self.assertRaises(ValueError): DesignTokenOperation("validate")
  with self.assertRaises(ValueError): DesignTokenValidationReport(1, "report", True, ("bad",), 0, 0, "")
if __name__ == "__main__": unittest.main()
