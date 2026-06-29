use serde::{Deserialize, Serialize};

/// Time of day used by scheduled appearance switching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
}

impl TimeOfDay {
    pub const fn new(hour: u8, minute: u8) -> Self {
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

impl Default for TimeOfDay {
    fn default() -> Self {
        Self::new(0, 0)
    }
}
