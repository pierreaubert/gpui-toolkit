import unittest
import contextlib
import io
import json

from gpui_toolkit.commands import CommandResult
from gpui_toolkit import SessionContext
from gpui_toolkit.themes import (
    CommunityThemeImport, ThemeAppearance, ThemeModePreference, ThemeSchedule, ThemeTransition, TimeOfDay,
)


class ThemeDeclarationsTests(unittest.TestCase):
    def test_schedule_matches_host_wraparound_behavior(self):
        schedule = ThemeSchedule(TimeOfDay(20, 0), TimeOfDay(7, 0))
        self.assertEqual(schedule.resolve_at_minutes(22 * 60), ThemeAppearance.LIGHT)
        self.assertEqual(schedule.resolve_at_minutes(12 * 60), ThemeAppearance.DARK)

    def test_preferences_and_transitions_are_validated(self):
        preference = ThemeModePreference("scheduled", ThemeSchedule())
        self.assertEqual(preference.resolve(ThemeAppearance.DARK, 8 * 60), ThemeAppearance.LIGHT)
        self.assertEqual(ThemeTransition(100).effective_duration_ms(True), 0)
        with self.assertRaises(ValueError): ThemeModePreference("scheduled")
        with self.assertRaises(ValueError): TimeOfDay(24, 0)

    def test_community_import_requires_native_validation_result(self):
        imported = CommunityThemeImport('{"manifest": {}, "theme": {}}')
        self.assertTrue(imported.json)
        entry = CommunityThemeImport.gallery_entry_from_command(CommandResult.from_wire("theme", {
            "ok": True, "id": "community", "display_name": "Community", "tags": ["dark"],
            "accessibility": "standard", "appearance": "dark",
        }))
        self.assertEqual(entry.id, "community")

    def test_community_import_activation_is_a_host_command(self):
        imported = CommunityThemeImport('{"manifest": {}, "theme": {}}')
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            imported.activate(SessionContext(), "activate-theme")
        self.assertEqual(json.loads(output.getvalue()), {
            "type": "command", "request_id": "activate-theme",
            "command": "themes.community_activate",
            "arguments": {"input": imported.json},
        })
        active = CommunityThemeImport.active_theme_from_command(CommandResult.from_wire("activate-theme", {
            "ok": True, "id": "community", "display_name": "Community", "tags": ["dark"],
            "accessibility": "standard", "appearance": "dark", "active": True,
        }))
        self.assertTrue(active.active)
        self.assertEqual(active.entry.id, "community")


if __name__ == "__main__":
    unittest.main()
