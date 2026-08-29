//! Small platform-lifecycle invariants shared with host-runnable regressions.

use std::sync::atomic::{AtomicBool, Ordering};

/// Records a main-thread wake before its pipe notification is sent.
pub(crate) fn mark_main_thread_wake_pending(pending: &AtomicBool) {
    pending.store(true, Ordering::Release);
}

/// Atomically updates the active state and returns whether it changed.
pub(crate) fn active_state_changed(active: &AtomicBool, next: bool) -> bool {
    active.swap(next, Ordering::Relaxed) != next
}

/// Restores a resource taken from an optional slot while an unlocked operation
/// is in progress.
pub(crate) fn restore_taken_value<T>(slot: &mut Option<T>, value: T) {
    debug_assert!(
        slot.is_none(),
        "a taken resource must be restored only once"
    );
    *slot = Some(value);
}

#[cfg(test)]
mod tests {
    use super::{active_state_changed, mark_main_thread_wake_pending, restore_taken_value};
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn main_thread_dispatch_marks_a_pending_wake() {
        let pending = AtomicBool::new(false);
        mark_main_thread_wake_pending(&pending);
        assert!(pending.load(Ordering::Acquire));
    }

    #[test]
    fn active_transition_is_not_lost_when_state_sync_follows() {
        let active = AtomicBool::new(false);
        assert!(active_state_changed(&active, true));
        assert!(active.load(Ordering::Acquire));
        assert!(!active_state_changed(&active, true));
        assert!(active_state_changed(&active, false));
    }

    #[test]
    fn taken_renderer_is_restored_after_the_unlocked_operation() {
        let mut slot = None;
        restore_taken_value(&mut slot, "renderer");
        assert_eq!(slot, Some("renderer"));
    }
}
