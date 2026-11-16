//! Primary-scene bookkeeping for the `UIScene` life cycle.
//!
//! iOS/tvOS 27 require scene-based launching, but a GPUI app has one
//! platform window: the first connected scene claims it, later scenes
//! are logged and ignored, and a disconnect of the primary frees the
//! claim for the next scene. Pure state machine (scene identity is an
//! opaque `usize`, the `UIScene*` address at the FFI boundary) so the
//! claim rules are unit-testable off-device.

// Rust guideline compliant 2026-10-09

/// Outcome of a scene claiming the (single) GPUI window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneClaim {
    /// This scene now owns the GPUI window; attach and launch.
    Primary,
    /// This scene was already the primary (reconnect of same session).
    AlreadyPrimary,
    /// Another scene owns the window; ignore this one.
    Ignored,
}

/// Which scene, if any, currently owns the GPUI window.
#[derive(Debug, Default)]
pub struct PrimaryScene {
    primary: Option<usize>,
}

impl PrimaryScene {
    pub fn new() -> Self {
        Self::default()
    }

    /// Claim the GPUI window for `scene`.
    pub fn claim(&mut self, scene: usize) -> SceneClaim {
        match self.primary {
            None => {
                self.primary = Some(scene);
                SceneClaim::Primary
            }
            Some(existing) if existing == scene => SceneClaim::AlreadyPrimary,
            Some(_) => SceneClaim::Ignored,
        }
    }

    /// Release the claim if `scene` holds it. Returns whether the
    /// claim was released.
    pub fn release(&mut self, scene: usize) -> bool {
        if self.primary == Some(scene) {
            self.primary = None;
            true
        } else {
            false
        }
    }

    /// The scene currently owning the GPUI window, if any.
    pub fn primary(&self) -> Option<usize> {
        self.primary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_scene_claims_primary() {
        let mut scenes = PrimaryScene::new();
        assert_eq!(scenes.claim(0x1000), SceneClaim::Primary);
        assert_eq!(scenes.primary(), Some(0x1000));
    }

    #[test]
    fn second_scene_is_ignored_while_primary_connected() {
        let mut scenes = PrimaryScene::new();
        assert_eq!(scenes.claim(0x1000), SceneClaim::Primary);
        assert_eq!(scenes.claim(0x2000), SceneClaim::Ignored);
        assert_eq!(scenes.primary(), Some(0x1000));
    }

    #[test]
    fn same_scene_reclaim_is_already_primary() {
        let mut scenes = PrimaryScene::new();
        scenes.claim(0x1000);
        assert_eq!(scenes.claim(0x1000), SceneClaim::AlreadyPrimary);
    }

    #[test]
    fn release_only_affects_the_primary() {
        let mut scenes = PrimaryScene::new();
        scenes.claim(0x1000);
        assert!(!scenes.release(0x2000));
        assert_eq!(scenes.primary(), Some(0x1000));
        assert!(scenes.release(0x1000));
        assert_eq!(scenes.primary(), None);
    }

    #[test]
    fn released_primary_lets_next_scene_claim() {
        let mut scenes = PrimaryScene::new();
        scenes.claim(0x1000);
        scenes.release(0x1000);
        assert_eq!(scenes.claim(0x2000), SceneClaim::Primary);
        assert_eq!(scenes.primary(), Some(0x2000));
    }
}
