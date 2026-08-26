use serde::{Deserialize, Deserializer, Serialize};

/// Time of day used by scheduled appearance switching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
}

impl TimeOfDay {
    pub const fn new(hour: u8, minute: u8) -> Self {
        assert!(hour < 24, "hour must be less than 24");
        assert!(minute < 60, "minute must be less than 60");
        Self { hour, minute }
    }

    pub fn checked_new(hour: u8, minute: u8) -> Option<Self> {
        if hour < 24 && minute < 60 {
            Some(Self { hour, minute })
        } else {
            None
        }
    }

    pub const fn minutes_after_midnight(self) -> u16 {
        self.hour as u16 * 60 + self.minute as u16
    }
}

impl<'de> Deserialize<'de> for TimeOfDay {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireTimeOfDay {
            hour: u8,
            minute: u8,
        }

        let wire = WireTimeOfDay::deserialize(deserializer)?;
        Self::checked_new(wire.hour, wire.minute)
            .ok_or_else(|| serde::de::Error::custom("time of day must be in 00:00..=23:59"))
    }
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self::new(0, 0)
    }
}
