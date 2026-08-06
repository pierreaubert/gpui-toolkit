import unittest
from gpui_toolkit.lab import ComponentStory, StoryProp, ViewportPreset

class LabDeclarationsTests(unittest.TestCase):
    def test_story_declaration_serializes_native_shape(self):
        story = ComponentStory("kit.button", "gpui-ui-kit", "Button", "A button", (StoryProp("variant", "Variant", "primary", "choice", ("primary",)),), (ViewportPreset("phone", "Phone", 320, 640),))
        self.assertEqual(story.to_spec()["props"][0]["value"]["type"], "choice")
    def test_invalid_story_declarations_fail_early(self):
        with self.assertRaises(ValueError): StoryProp("x", "X", "a", "choice")
        with self.assertRaises(ValueError): ViewportPreset("x", "X", 0, 1)

if __name__ == "__main__": unittest.main()
