use super::types::ThemeTransitionEasing;
use serde::{Deserialize, Serialize};

/// Theme transition settings shared by frontends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeTransition {
    pub duration_ms: u16,
    pub easing: ThemeTransitionEasing,
    pub cross_fade: bool,
}

impl ThemeTransition {
    pub fn disabled() -> Self {
        Self {
            duration_ms: 0,
            easing: ThemeTransitionEasing::Linear,
            cross_fade: false,
        }
    }

    pub fn effective_duration_ms(self, reduce_motion: bool) -> u16 {
        if reduce_motion { 0 } else { self.duration_ms }
    }
}

impl Default for ThemeTransition {
    fn default() -> Self {
        Self {
            duration_ms: 220,
            easing: ThemeTransitionEasing::EaseOut,
            cross_fade: true,
        }
    }
}

impl ThemeTransition {
    /// Whether the transition animates under the given reduced-motion flag.
    ///
    /// Reduced motion always wins: animations are off even when `cross_fade`
    /// is set, giving frontends a single gate for animated previews.
    pub fn is_animated(self, reduce_motion: bool) -> bool {
        self.cross_fade && self.effective_duration_ms(reduce_motion) > 0
    }

    /// Eased 0..1 cross-fade progress for an animated theme preview.
    ///
    /// Returns `1.0` (switch instantly) when the transition is disabled or
    /// reduced motion is requested; otherwise applies the configured easing
    /// to `elapsed_ms / duration_ms`, clamped to `0..=1`.
    pub fn preview_progress(self, elapsed_ms: u16, reduce_motion: bool) -> f32 {
        if !self.is_animated(reduce_motion) {
            return 1.0;
        }
        let duration = self.duration_ms.max(1) as f32;
        let t = (elapsed_ms as f32 / duration).clamp(0.0, 1.0);
        match self.easing {
            ThemeTransitionEasing::Linear => t,
            ThemeTransitionEasing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            ThemeTransitionEasing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
        }
    }
}
