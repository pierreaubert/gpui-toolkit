import unittest

from gpui_toolkit.commands import CommandResult, CommandStatus


class CommandResultTests(unittest.TestCase):
    def test_host_command_outcomes_are_typed_and_immutable(self):
        result = CommandResult.from_wire(
            "capabilities", {"ok": True, "capabilities": ["commands"]},
        )
        self.assertEqual(result.status, CommandStatus.SUCCEEDED)
        self.assertTrue(result.ok)
        self.assertEqual(result.data["capabilities"], ["commands"])
        with self.assertRaises(TypeError):
            result.data["extra"] = True

    def test_unsupported_command_is_distinct_from_failure(self):
        result = CommandResult.from_wire(
            "unknown", {"ok": False, "unsupported": True, "error": "not installed"},
        )
        self.assertEqual(result.status, CommandStatus.UNSUPPORTED)
        self.assertFalse(result.ok)
        self.assertEqual(result.error, "not installed")
