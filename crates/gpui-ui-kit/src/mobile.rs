//! Mobile interaction primitives shared by GPUI components.

/// Edge insets in visual order.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

/// Pull-to-refresh interaction state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PullToRefreshState {
    Idle,
    Pulling { distance: f32, threshold: f32 },
    Armed,
    Refreshing,
}

impl PullToRefreshState {
    pub fn from_drag(distance: f32, threshold: f32) -> Self {
        if threshold <= 0.0 || distance <= 0.0 {
            Self::Idle
        } else if distance >= threshold {
            Self::Armed
        } else {
            Self::Pulling {
                distance,
                threshold,
            }
        }
    }

    pub fn progress(self) -> f32 {
        match self {
            Self::Idle => 0.0,
            Self::Pulling {
                distance,
                threshold,
            } => (distance / threshold).clamp(0.0, 1.0),
            Self::Armed | Self::Refreshing => 1.0,
        }
    }
}

/// Direction for swipe actions on mobile rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Leading,
    Trailing,
}

/// One row action revealed by a swipe.
#[derive(Debug, Clone, PartialEq)]
pub struct SwipeAction {
    pub id: String,
    pub label: String,
    pub direction: SwipeDirection,
    pub destructive: bool,
}

impl SwipeAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>, direction: SwipeDirection) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            direction,
            destructive: false,
        }
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
}

/// Preview metadata for iOS-style context menus.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextPreview {
    pub title: String,
    pub subtitle: Option<String>,
    pub preferred_width: f32,
    pub preferred_height: f32,
}

impl ContextPreview {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("context preview title must not be empty".to_string());
        }
        if self.preferred_width <= 0.0 || self.preferred_height <= 0.0 {
            return Err("context preview size must be positive".to_string());
        }
        Ok(())
    }
}

/// Dynamic Type scaling policy for fixed-format controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicTypePolicy {
    pub scale_factor: f32,
    pub min_size: f32,
    pub max_size: f32,
}

impl DynamicTypePolicy {
    pub fn resolve(self, base_size: f32) -> f32 {
        (base_size * self.scale_factor).clamp(self.min_size, self.max_size)
    }
}

/// Non-rendering waveform scrubber model.
#[derive(Debug, Clone, PartialEq)]
pub struct WaveformScrubber {
    pub duration_seconds: f32,
    pub position_seconds: f32,
    pub samples: Vec<f32>,
}

impl WaveformScrubber {
    pub fn normalized_position(&self) -> f32 {
        if self.duration_seconds <= 0.0 {
            0.0
        } else {
            (self.position_seconds / self.duration_seconds).clamp(0.0, 1.0)
        }
    }

    pub fn seek_to_fraction(&mut self, fraction: f32) {
        self.position_seconds = self.duration_seconds * fraction.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{ContextPreview, PullToRefreshState, WaveformScrubber};

    #[test]
    fn pull_to_refresh_progress_is_clamped() {
        assert_eq!(PullToRefreshState::from_drag(-2.0, 80.0).progress(), 0.0);
        assert_eq!(PullToRefreshState::from_drag(40.0, 80.0).progress(), 0.5);
        assert_eq!(
            PullToRefreshState::from_drag(120.0, 80.0),
            PullToRefreshState::Armed
        );
    }

    #[test]
    fn waveform_scrubber_maps_fraction_to_position() {
        let mut scrubber = WaveformScrubber {
            duration_seconds: 120.0,
            position_seconds: 0.0,
            samples: vec![0.0, 0.5, 1.0],
        };

        scrubber.seek_to_fraction(0.25);
        assert_eq!(scrubber.position_seconds, 30.0);
        assert_eq!(scrubber.normalized_position(), 0.25);
    }

    #[test]
    fn context_preview_validates_user_visible_shape() {
        let preview = ContextPreview {
            title: "Album".to_string(),
            subtitle: None,
            preferred_width: 320.0,
            preferred_height: 180.0,
        };
        assert!(preview.validate().is_ok());
    }
}
