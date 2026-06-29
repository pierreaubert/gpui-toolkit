use super::time_of_day::TimeOfDay;
use super::types::ThemeAppearance;
use serde::{Deserialize, Serialize};

/// Local schedule for apps that do not rely only on the operating system setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeSchedule {
    #[serde(default = "ThemeSchedule::default_light_start")]
    pub light_start: TimeOfDay,
    #[serde(default = "ThemeSchedule::default_dark_start")]
    pub dark_start: TimeOfDay,
}

impl ThemeSchedule {
    pub const fn new(light_start: TimeOfDay, dark_start: TimeOfDay) -> Self {
        Self {
            light_start,
            dark_start,
        }
    }

    pub const fn default_light_start() -> TimeOfDay {
        TimeOfDay::new(7, 0)
    }

    pub const fn default_dark_start() -> TimeOfDay {
        TimeOfDay::new(18, 0)
    }

    pub fn resolve_at_minutes(self, minutes_after_midnight: u16) -> ThemeAppearance {
        let minute = minutes_after_midnight % (24 * 60);
        let light_start = self.light_start.minutes_after_midnight();
        let dark_start = self.dark_start.minutes_after_midnight();

        if light_start == dark_start {
            return ThemeAppearance::Dark;
        }

        let is_light = if light_start < dark_start {
            minute >= light_start && minute < dark_start
        } else {
            minute >= light_start || minute < dark_start
        };

        if is_light {
            ThemeAppearance::Light
        } else {
            ThemeAppearance::Dark
        }
    }
}

impl Default for ThemeSchedule {
    fn default() -> Self {
        Self::new(Self::default_light_start(), Self::default_dark_start())
    }
}
