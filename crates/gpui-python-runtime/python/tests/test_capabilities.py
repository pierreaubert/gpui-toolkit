import unittest
import gpui_toolkit

class CapabilityTests(unittest.TestCase):
    def test_capabilities_are_immutable_and_discoverable(self):
        entries = gpui_toolkit.capabilities()
        self.assertIsInstance(entries, tuple)
        self.assertIn("gpui-d3rs.scene3d", {entry.id for entry in entries})
        self.assertEqual(next(item for item in entries if item.id == "gpui-au.platform").disposition, "platform-unavailable")

if __name__ == "__main__":
    unittest.main()
