//! Host-portable policy helpers for the Windows foreign-parent binding.
//!
//! Pure geometry and editor-id validation shared by the C ABI
//! ([`super::foreign_parent`], Windows-only) and host-side unit tests:
//! drawable-size math (contract §5), create/resize argument validation,
//! and editor-id allocation. No `HWND`, no GPU, no GPUI dependency.

// Rust guideline compliant 2026-02-21

use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(any(target_os = "windows", test))]
thread_local! {
    static CREATING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Prevents nested use of the backend's single pending-window handoff.
#[cfg(any(target_os = "windows", test))]
pub(crate) struct CreationGuard(std::marker::PhantomData<std::rc::Rc<()>>);

#[cfg(any(target_os = "windows", test))]
impl CreationGuard {
    pub(crate) fn enter() -> Option<Self> {
        CREATING
            .try_with(|flag| {
                if flag.get() {
                    None
                } else {
                    flag.set(true);
                    Some(Self(std::marker::PhantomData))
                }
            })
            .ok()
            .flatten()
    }
}

#[cfg(any(target_os = "windows", test))]
impl Drop for CreationGuard {
    fn drop(&mut self) {
        let _ = CREATING.try_with(|flag| flag.set(false));
    }
}

/// Validated host geometry for an attach or resize: physical size plus
/// the backing scale it was derived with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForeignGeometry {
    /// Child width in physical pixels, always `>= 1`.
    pub width: i32,
    /// Child height in physical pixels, always `>= 1`.
    pub height: i32,
    /// Host backing scale, always finite and positive.
    pub scale: f32,
}

/// Validate host geometry, rejecting the call on bad input.
///
/// `width`/`height` are logical pixels and `scale` is the backing scale;
/// the physical size is `round(logical × scale)` per axis, clamped to
/// `i32` with a minimum of 1 — a zero host size never produces a zero
/// drawable (contract §5). Returns `None` when any input is non-finite
/// or the scale is non-positive.
#[must_use]
pub fn validate_geometry(width: f32, height: f32, scale: f32) -> Option<ForeignGeometry> {
    if !width.is_finite() || !height.is_finite() || !scale.is_finite() {
        return None;
    }
    if scale <= 0.0 {
        return None;
    }
    let (physical_width, physical_height) = drawable_size(width, height, scale);
    Some(ForeignGeometry {
        width: physical_width.min(i32::MAX as u32) as i32,
        height: physical_height.min(i32::MAX as u32) as i32,
        scale,
    })
}

/// Drawable size: `max(1, round(logical × scale))` per axis.
///
/// The rounding matches the swapchain sizing the backend applies; the
/// `max(1, …)` keeps zero-area hosts from producing zero drawables.
#[must_use]
pub fn drawable_size(width: f32, height: f32, scale: f32) -> (u32, u32) {
    let axis = |logical: f32| (logical * scale).round().clamp(1.0, u32::MAX as f32) as u32;
    (axis(width), axis(height))
}

/// Editor ids start at 1; 0 and negatives are always invalid.
#[must_use]
pub fn valid_editor_id(editor_id: i64) -> Option<u64> {
    u64::try_from(editor_id).ok().filter(|id| *id != 0)
}

static NEXT_EDITOR_ID: AtomicU64 = AtomicU64::new(1);

/// Allocate a fresh nonzero editor id.
pub fn allocate_editor_id() -> u64 {
    loop {
        // Ids only need process-local uniqueness; 2^64 creates cannot
        // happen, but 0 stays reserved as "invalid" regardless.
        let id = NEXT_EDITOR_ID.fetch_add(1, Ordering::Relaxed);
        if id != 0 {
            return id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_creation_rejection_does_not_unlock_outer_creation() {
        let outer = CreationGuard::enter().unwrap();
        assert!(CreationGuard::enter().is_none());
        assert!(CreationGuard::enter().is_none());
        drop(outer);
        assert!(CreationGuard::enter().is_some());
    }

    #[test]
    fn creation_guard_releases_after_panic() {
        let result = std::panic::catch_unwind(|| {
            let _guard = CreationGuard::enter().unwrap();
            panic!("factory panic");
        });
        result.unwrap_err();
        assert!(CreationGuard::enter().is_some());
    }

    #[test]
    fn valid_geometry_rounds_to_physical() {
        let geometry = validate_geometry(800.0, 600.0, 2.0).expect("valid input");
        assert_eq!(
            geometry,
            ForeignGeometry {
                width: 1600,
                height: 1200,
                scale: 2.0,
            }
        );
    }

    #[test]
    fn fractional_geometry_rounds_per_axis() {
        let geometry = validate_geometry(100.4, 100.5, 1.0).expect("valid input");
        assert_eq!((geometry.width, geometry.height), (100, 101));
    }

    #[test]
    fn tiny_geometry_clamps_to_one_pixel() {
        let geometry = validate_geometry(0.1, 0.1, 0.5).expect("valid input");
        assert_eq!((geometry.width, geometry.height), (1, 1));
    }

    #[test]
    fn non_finite_or_bad_scale_geometry_is_rejected() {
        assert!(validate_geometry(f32::NAN, 600.0, 1.0).is_none());
        assert!(validate_geometry(800.0, f32::INFINITY, 1.0).is_none());
        assert!(validate_geometry(800.0, 600.0, f32::NAN).is_none());
        assert!(validate_geometry(800.0, 600.0, f32::INFINITY).is_none());
        assert!(validate_geometry(800.0, 600.0, 0.0).is_none());
        assert!(validate_geometry(800.0, 600.0, -2.0).is_none());
    }

    #[test]
    fn zero_or_negative_sizes_clamp_to_one_pixel() {
        let geometry = validate_geometry(0.0, -50.0, 1.0).expect("clamped, not rejected");
        assert_eq!((geometry.width, geometry.height), (1, 1));
    }

    #[test]
    fn drawable_size_never_returns_zero() {
        assert_eq!(drawable_size(0.0, 0.0, 1.0), (1, 1));
        assert_eq!(drawable_size(1920.0, 1080.0, 1.5), (2880, 1620));
    }

    #[test]
    fn editor_ids_reject_zero_and_negatives() {
        assert_eq!(valid_editor_id(0), None);
        assert_eq!(valid_editor_id(-1), None);
        assert_eq!(valid_editor_id(7), Some(7));
    }

    #[test]
    fn allocated_ids_are_nonzero_and_increasing() {
        let first = allocate_editor_id();
        let second = allocate_editor_id();
        assert_ne!(first, 0);
        assert_ne!(second, 0);
        assert_ne!(first, second);
    }
}
