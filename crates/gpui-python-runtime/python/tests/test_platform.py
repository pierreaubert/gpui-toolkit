import os
import unittest
from unittest.mock import patch

from gpui_toolkit.platform import (
    ANDROID_HOST,
    AU_EMBEDDING,
    IOS_HOST,
    UnsupportedCapability,
    capabilities,
    require_capability,
)


class PlatformCapabilityTests(unittest.TestCase):
    def test_every_optional_adapter_is_importable(self):
        self.assertEqual({item.id for item in capabilities()}, {"au_embedding", "ios_host", "android_host"})
        self.assertEqual(AU_EMBEDDING.supported_platforms, ("darwin",))
        self.assertEqual(IOS_HOST.supported_platforms, ("ios",))
        self.assertEqual(ANDROID_HOST.supported_platforms, ("android",))

    def test_unavailable_adapter_raises_a_typed_error(self):
        with patch.dict(os.environ, {"GPUI_TOOLKIT_PLATFORM_CAPABILITIES": ""}, clear=False):
            with self.assertRaises(UnsupportedCapability):
                require_capability("ios_host")

    def test_enabled_adapter_is_returned_without_crossing_native_handles(self):
        with patch.dict(os.environ, {"GPUI_TOOLKIT_PLATFORM_CAPABILITIES": "ios_host"}, clear=False):
            self.assertIs(require_capability("ios_host"), IOS_HOST)


if __name__ == "__main__":
    unittest.main()
