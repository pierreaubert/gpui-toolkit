//! Pure event-loop stage helpers for the Android backend.
//!
//! The Android JNI event loop (`android::jni::run_event_loop`) is structured
//! as **poll → decode → dispatch → draw**. The stage *decisions* live here as
//! pure, platform-free functions so they are unit-testable on any host; the
//! Android-only glue in `android::jni` / `android::window` maps NDK types onto
//! them and performs the effects.
//!
//! Keeping the predicates here (rather than inline in the loop) is what makes
//! the loop's complexity testable: every branch below has a unit test.

use std::time::Duration;

/// Frame-pump cadence used when momentum scrolling or a fling animation needs
/// continuous frames. Mirrors `android::jni::FRAME_POLL_INTERVAL`.
pub const FRAME_POLL_INTERVAL: Duration = Duration::from_millis(8);

/// Decide how long the event loop may block waiting for OS events.
///
/// * `needs_frame_pump` — momentum scroll / fling animation is active, so the
///   loop must wake at frame cadence.
/// * `next_delayed_task` — how long until the next delayed main-thread task is
///   due, if any.
///
/// Returns `None` to block indefinitely, otherwise the wake timeout.
pub fn frame_poll_timeout(
    needs_frame_pump: bool,
    next_delayed_task: Option<Duration>,
) -> Option<Duration> {
    if needs_frame_pump {
        Some(FRAME_POLL_INTERVAL)
    } else {
        next_delayed_task
    }
}

/// Decide whether the loop should pump a GPUI frame this iteration.
///
/// All conditions must hold: the window is initialised (`init_done`) and the
/// app is foregrounded (`active`), and at least one wake source fired:
/// `force_frame` (first frame after poll), `event_woke` (an OS event arrived),
/// `main_wake` (a main-thread task was dispatched), or `needs_pump`
/// (momentum scroll / fling animation in progress).
pub fn should_pump_frame(
    init_done: bool,
    active: bool,
    force_frame: bool,
    event_woke: bool,
    main_wake: bool,
    needs_pump: bool,
) -> bool {
    init_done && active && (force_frame || event_woke || main_wake || needs_pump)
}

/// Motion-event kind, independent of the NDK input types.
///
/// The Android-only layer maps `android_activity::input::MotionAction` onto
/// this enum so the per-pointer decode below stays testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionKind {
    Down,
    PointerDown,
    Up,
    PointerUp,
    Move,
    Cancel,
    Other,
}

/// Masked touch actions delivered to `AndroidWindow::handle_touch`.
/// These match the `AMOTION_EVENT_ACTION_*` constants used by the window.
pub const TOUCH_ACTION_DOWN: u32 = 0;
pub const TOUCH_ACTION_UP: u32 = 1;
pub const TOUCH_ACTION_MOVE: u32 = 2;

/// Decode one pointer slot of a motion event.
///
/// * `Down` / `Up` / `Cancel` apply to every pointer slot (`Cancel` is
///   delivered as `Up` so gestures terminate cleanly).
/// * `PointerDown` / `PointerUp` apply only to the slot at `pointer_index`.
/// * `Move` applies to every slot.
/// * `Other` (hover, scroll, button, …) is skipped.
///
/// Returns the masked action to dispatch, or `None` to skip the slot.
pub fn decode_motion_slot(kind: MotionKind, pointer_index: usize, slot: usize) -> Option<u32> {
    match kind {
        MotionKind::Down => Some(TOUCH_ACTION_DOWN),
        MotionKind::Up => Some(TOUCH_ACTION_UP),
        MotionKind::Cancel => Some(TOUCH_ACTION_UP),
        MotionKind::PointerDown => (slot == pointer_index).then_some(TOUCH_ACTION_DOWN),
        MotionKind::PointerUp => (slot == pointer_index).then_some(TOUCH_ACTION_UP),
        MotionKind::Move => Some(TOUCH_ACTION_MOVE),
        MotionKind::Other => None,
    }
}

/// Key-event direction, independent of the NDK input types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDirection {
    Press,
    Release,
}

impl KeyDirection {
    /// The `action` integer stored in `android::AndroidKeyEvent`
    /// (`ACTION_DOWN = 0`, `ACTION_UP = 1`).
    pub fn action_int(self) -> i32 {
        match self {
            KeyDirection::Press => 0,
            KeyDirection::Release => 1,
        }
    }
}

/// Decode a raw key-event action integer into a direction.
///
/// Returns `None` for anything other than down/up (e.g. `ACTION_MULTIPLE`),
/// which the loop reports as unhandled so the OS can route it elsewhere.
pub fn decode_key_action(action: i32) -> Option<KeyDirection> {
    match action {
        0 => Some(KeyDirection::Press),
        1 => Some(KeyDirection::Release),
        _ => None,
    }
}

/// Decide whether an IME cursor-anchor update must be sent to Java.
///
/// Returns `true` on the first update (`last` is `None`) or when the caret
/// origin/height changed since the last sent update. The caller records the
/// new bounds only when the JNI update succeeds, so a failed update is
/// retried on the next keystroke instead of being dropped.
pub fn ime_anchor_changed(last: Option<(f32, f32, f32)>, x: f32, y: f32, h: f32) -> bool {
    last != Some((x, y, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_blocks_without_frame_or_delayed_work() {
        assert_eq!(frame_poll_timeout(false, None), None);
        assert_eq!(
            frame_poll_timeout(false, Some(Duration::from_millis(25))),
            Some(Duration::from_millis(25))
        );
        // Frame pump wins over delayed work: momentum needs frame cadence.
        assert_eq!(
            frame_poll_timeout(true, Some(Duration::from_secs(1))),
            Some(FRAME_POLL_INTERVAL)
        );
        assert_eq!(frame_poll_timeout(true, None), Some(FRAME_POLL_INTERVAL));
    }

    #[test]
    fn frame_pump_requires_init_active_and_a_wake_source() {
        // No wake source → no frame.
        assert!(!should_pump_frame(true, true, false, false, false, false));
        // Each wake source alone suffices when initialised and active.
        assert!(should_pump_frame(true, true, true, false, false, false));
        assert!(should_pump_frame(true, true, false, true, false, false));
        assert!(should_pump_frame(true, true, false, false, true, false));
        assert!(should_pump_frame(true, true, false, false, false, true));
        // Uninitialised surface or backgrounded app → no frame.
        assert!(!should_pump_frame(false, true, true, true, true, true));
        assert!(!should_pump_frame(true, false, true, true, true, true));
    }

    #[test]
    fn motion_decode_covers_all_pointer_slots() {
        // Down/Up/Move/Cancel dispatch for every slot.
        for slot in 0..3 {
            assert_eq!(
                decode_motion_slot(MotionKind::Down, 1, slot),
                Some(TOUCH_ACTION_DOWN)
            );
            assert_eq!(
                decode_motion_slot(MotionKind::Up, 1, slot),
                Some(TOUCH_ACTION_UP)
            );
            assert_eq!(
                decode_motion_slot(MotionKind::Move, 1, slot),
                Some(TOUCH_ACTION_MOVE)
            );
            // Cancel terminates the gesture as an up.
            assert_eq!(
                decode_motion_slot(MotionKind::Cancel, 1, slot),
                Some(TOUCH_ACTION_UP)
            );
        }
        // PointerDown/PointerUp dispatch only the changed pointer.
        assert_eq!(
            decode_motion_slot(MotionKind::PointerDown, 1, 1),
            Some(TOUCH_ACTION_DOWN)
        );
        assert_eq!(decode_motion_slot(MotionKind::PointerDown, 1, 0), None);
        assert_eq!(
            decode_motion_slot(MotionKind::PointerUp, 2, 2),
            Some(TOUCH_ACTION_UP)
        );
        assert_eq!(decode_motion_slot(MotionKind::PointerUp, 2, 0), None);
        // Hover/scroll/button actions are skipped.
        assert_eq!(decode_motion_slot(MotionKind::Other, 0, 0), None);
    }

    #[test]
    fn key_decode_round_trips_through_action_int() {
        assert_eq!(
            decode_key_action(KeyDirection::Press.action_int()),
            Some(KeyDirection::Press)
        );
        assert_eq!(
            decode_key_action(KeyDirection::Release.action_int()),
            Some(KeyDirection::Release)
        );
        // ACTION_MULTIPLE and garbage are unhandled.
        assert_eq!(decode_key_action(2), None);
        assert_eq!(decode_key_action(-1), None);
    }

    #[test]
    fn ime_anchor_updates_only_on_change() {
        // First update always goes out.
        assert!(ime_anchor_changed(None, 10.0, 20.0, 16.0));
        // Identical caret → coalesced, no JNI.
        assert!(!ime_anchor_changed(
            Some((10.0, 20.0, 16.0)),
            10.0,
            20.0,
            16.0
        ));
        // Any component change → new update.
        assert!(ime_anchor_changed(
            Some((10.0, 20.0, 16.0)),
            11.0,
            20.0,
            16.0
        ));
        assert!(ime_anchor_changed(
            Some((10.0, 20.0, 16.0)),
            10.0,
            21.0,
            16.0
        ));
        assert!(ime_anchor_changed(
            Some((10.0, 20.0, 16.0)),
            10.0,
            20.0,
            18.0
        ));
        // Zero-size bounds are still a real (changed) position.
        assert!(ime_anchor_changed(None, 0.0, 0.0, 0.0));
        assert!(!ime_anchor_changed(Some((0.0, 0.0, 0.0)), 0.0, 0.0, 0.0));
    }
}
