import unittest

from gpui_toolkit.themes import (
    ThemeAppearance, ThemeModePreference, ThemeSchedule, ThemeTransition, TimeOfDay,
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


if __name__ == "__main__":
    unittest.main()
