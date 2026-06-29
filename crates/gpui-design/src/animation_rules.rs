use serde::Serialize;

/// Animation timing rules.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct AnimationRules {
    /// Default transition duration in milliseconds.
    pub duration_ms: u32,
    /// Fast transition (hover, press) in milliseconds.
    pub fast_ms: u32,
    /// Slow transition (page, modal) in milliseconds.
    pub slow_ms: u32,
    /// Whether to prefer spring physics over eased curves.
    pub prefer_spring: bool,
    /// Spring stiffness (used when prefer_spring is true).
    pub spring_stiffness: f32,
    /// Spring damping (used when prefer_spring is true).
    pub spring_damping: f32,
}

impl AnimationRules {
    pub fn new(
        duration_ms: u32,
        fast_ms: u32,
        slow_ms: u32,
        prefer_spring: bool,
        spring_stiffness: f32,
        spring_damping: f32,
    ) -> Self {
        assert!(duration_ms > 0, "duration_ms must be > 0");
        assert!(fast_ms > 0, "fast_ms must be > 0");
        assert!(slow_ms > 0, "slow_ms must be > 0");
        assert!(spring_stiffness > 0.0, "spring_stiffness must be > 0");
        assert!(spring_damping > 0.0, "spring_damping must be > 0");
        Self {
            duration_ms,
            fast_ms,
            slow_ms,
            prefer_spring,
            spring_stiffness,
            spring_damping,
        }
    }
}
