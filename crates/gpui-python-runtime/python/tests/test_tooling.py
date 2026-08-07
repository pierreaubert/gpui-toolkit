import unittest

from gpui_toolkit.commands import CommandResult
from gpui_toolkit.tooling import (
    DesignTokenOperation,
    DesignTokenOperationKind,
    DesignTokenValidationReport,
    DesignToolingHandoffReport,
    ImportedDesignTokens,
)


class ToolingDeclarationsTests(unittest.TestCase):
    def test_host_operation_and_report_contract(self):
        self.assertEqual(
            DesignTokenOperation("validate", input="{}").to_spec()["operation"],
            "validate",
        )
        self.assertTrue(DesignTokenValidationReport(1, "report", True, (), 1, 2, "").passed)

    def test_contract_rejects_invalid_shapes(self):
        with self.assertRaises(ValueError):
            DesignTokenOperation("validate")
        with self.assertRaises(ValueError):
            DesignTokenValidationReport(1, "report", True, ("bad",), 0, 0, "")

    def test_native_operation_results_are_typed(self):
        imported = ImportedDesignTokens.from_command(CommandResult.from_wire(
            "import", {"ok": True, "preset_count": 1, "token_count": 2, "raw": {"presets": []}},
        ))
        self.assertEqual(imported.token_count, 2)
        self.assertEqual(
            DesignTokenOperation(DesignTokenOperationKind.HANDOFF).to_spec()["operation"],
            "handoff",
        )
        handoff = DesignToolingHandoffReport.from_command(CommandResult.from_wire(
            "handoff", {"ok": True, "report": {
                "schema_version": 1, "report_type": "handoff", "crate_name": "tools",
                "crate_version": "0.9", "items": [{"id": "export"}],
            }},
        ))
        self.assertEqual(handoff.items[0]["id"], "export")
