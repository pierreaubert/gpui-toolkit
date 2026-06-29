/// Minimum module size in pixels below which animation is triggered.
pub(super) const MIN_MODULE_PX: f32 = 2.0;

/// Quiet-zone width in modules on each side of the QR matrix.
pub(super) const QUIET_ZONE: usize = 4;

/// Clamp scroll range so that at least one module stays visible.
pub(super) fn clamped_scroll_range(total_modules: usize, viewport_modules: f32) -> f32 {
    (total_modules as f32 - viewport_modules)
        .max(0.0)
        .min(total_modules.saturating_sub(1) as f32)
}

/// Cubic ease-in-out for smooth animation.
pub(super) fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::clamped_scroll_range;

    #[test]
    fn test_scroll_range_clamped() {
        // viewport larger than total -> no scrolling
        assert_eq!(clamped_scroll_range(10, 15.0), 0.0);
        // normal case
        assert_eq!(clamped_scroll_range(10, 3.0), 7.0);
        // viewport < 1 module -> keep at least one module visible
        assert_eq!(clamped_scroll_range(10, 0.0), 9.0);
        // single module -> no scrolling
        assert_eq!(clamped_scroll_range(1, 0.5), 0.0);
    }
}
