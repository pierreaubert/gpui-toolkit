use super::theme_schedule::ThemeSchedule;
use super::types::ThemeAppearance;
use serde::{Deserialize, Serialize};

/// Per-app theme mode override.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ThemeModePreference {
    #[default]
    FollowSystem,
    Light,
    Dark,
    Scheduled {
        schedule: ThemeSchedule,
    },
}

impl ThemeAppearance {
    /// Map a platform dark-mode flag to an appearance.
    ///
    /// Wiring point for OS appearance listeners: call the platform's
    /// dark-mode callback and feed its flag through here (or through
    /// [`ThemeModePreference::resolve_live`]) instead of branching inline.
    pub fn from_system_dark_flag(is_dark: bool) -> Self {
        if is_dark {
            Self::Dark
        } else {
            Self::Light
        }
    }
}

impl ThemeModePreference {
    pub fn resolve(
        &self,
        system_appearance: ThemeAppearance,
        minutes_after_midnight: u16,
    ) -> ThemeAppearance {
        match self {
            Self::FollowSystem => system_appearance,
            Self::Light => ThemeAppearance::Light,
            Self::Dark => ThemeAppearance::Dark,
            Self::Scheduled { schedule } => schedule.resolve_at_minutes(minutes_after_midnight),
        }
    }

    /// Resolve against a live OS dark-mode flag.
    ///
    /// This is the single call site OS appearance listeners should invoke:
    /// `system_dark` comes from the platform callback, `minutes_after_midnight`
    /// only matters for [`ThemeModePreference::Scheduled`].
    pub fn resolve_live(&self, system_dark: bool, minutes_after_midnight: u16) -> ThemeAppearance {
        self.resolve(
            ThemeAppearance::from_system_dark_flag(system_dark),
            minutes_after_midnight,
        )
    }

    /// Whether this preference tracks the OS setting.
    pub fn follows_system(&self) -> bool {
        matches!(self, Self::FollowSystem)
    }
}
