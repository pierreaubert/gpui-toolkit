//! Single-window lease slot for the AU binding.
//!
//! GPUI owns the `AuWindow` box (`Box<dyn PlatformWindow>` returned from
//! `AuPlatform::open_window`), so this slot cannot own the window the
//! way [`OwnedEditorRegistry`](gpui_foreign_parent::OwnedEditorRegistry)
//! owns editors on other platforms. It provides the same two safety
//! properties for the single-instance AU case (each AU appex is its own
//! process):
//!
//! - Checked resolution with leases: [`with_au_window`] leases the
//!   window for one callback. [`unregister_au_window`] closes the slot
//!   immediately (later calls resolve to "no window"), but destruction
//!   deferred through [`defer_until_idle`] — notably dropping the
//!   `AuContext`, whose `AppCell` keeps the window box alive — waits
//!   until the last active lease returns. A host that destroys the view
//!   reentrantly from inside an FFI-triggered callback therefore cannot
//!   drop the window under a live borrow.
//! - Main-thread confinement in every build profile: foreign threads
//!   resolve to "no window" instead of dereferencing `!Send` state.
//!   The slot itself is thread-local, so each thread only ever sees its
//!   own (empty, off-main) slot.

use super::au_window::AuWindow;
use std::cell::RefCell;
use std::ptr::NonNull;

#[cfg(not(test))]
fn is_main_thread() -> bool {
    use objc::{
        class, msg_send,
        runtime::{BOOL, YES},
        sel, sel_impl,
    };
    // SAFETY: `NSThread` and its `isMainThread` class method always
    // exist; the message takes no arguments and returns a `BOOL`.
    unsafe {
        let is_main: BOOL = msg_send![class!(NSThread), isMainThread];
        is_main == YES
    }
}

#[cfg(test)]
fn is_main_thread() -> bool {
    // Unit tests run on harness threads; the slot confinement under test
    // is the lease/close logic, not AppKit thread identity.
    true
}

/// Runs once the last lease drains after close (or immediately when idle).
type DeferredDrop = Box<dyn FnOnce()>;

#[derive(Default)]
struct AuWindowSlot {
    window: Option<NonNull<AuWindow>>,
    leases: usize,
    closing: bool,
    deferred: Vec<DeferredDrop>,
}

thread_local! {
    static AU_WINDOW: RefCell<AuWindowSlot> = RefCell::new(AuWindowSlot::default());
}

fn drain_deferred(slot: &mut AuWindowSlot) {
    if slot.leases == 0 && slot.closing {
        let deferred = std::mem::take(&mut slot.deferred);
        for drop_deferred in deferred {
            drop_deferred();
        }
    }
}

struct LeaseGuard;

impl Drop for LeaseGuard {
    fn drop(&mut self) {
        AU_WINDOW.with(|slot| {
            let mut slot = slot.borrow_mut();
            slot.leases = slot.leases.saturating_sub(1);
            drain_deferred(&mut slot);
        });
    }
}

/// Execute a callback with a reference to the current AU window, if any.
///
/// The window is leased for the callback: a reentrant
/// [`unregister_au_window`] closes the slot but deferred destruction
/// (see [`defer_until_idle`]) waits for this call to return. Off the
/// main thread this resolves to `None` in every build profile.
pub(crate) fn with_au_window<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&AuWindow) -> R,
{
    if !is_main_thread() {
        return None;
    }
    let window = AU_WINDOW.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.closing {
            return None;
        }
        let window = slot.window?;
        slot.leases = slot.leases.saturating_add(1);
        Some(window)
    })?;
    let _lease = LeaseGuard;
    // SAFETY: `window` names the live `AuWindow` boxed by GPUI in
    // `AuPlatform::open_window`; the lease above defers the `AuContext`
    // (and with it the `AppCell` keeping that box alive) past this call,
    // and slot use is confined to the main thread.
    Some(f(unsafe { window.as_ref() }))
}

/// Register the window in the global slot.
/// Called from `AuPlatform::open_window` after Boxing.
pub(crate) fn register_au_window(boxed: &AuWindow) {
    assert!(
        is_main_thread(),
        "AuWindow must only be used on the main thread"
    );
    let ptr = NonNull::from(boxed);
    AU_WINDOW.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.window = Some(ptr);
        slot.closing = false;
    });
    let msg = format!("SOTF AuWindow: registered at {ptr:p}");
    crate::helpers::nslog_verbose(msg.as_bytes());
}

/// Close the slot (called during destroy). Later [`with_au_window`]
/// calls resolve to `None`; deferred destruction runs once active
/// leases drain.
pub(crate) fn unregister_au_window() {
    if !is_main_thread() {
        return;
    }
    AU_WINDOW.with(|slot| {
        let mut slot = slot.borrow_mut();
        slot.closing = true;
        slot.window = None;
        drain_deferred(&mut slot);
    });
}

/// Run `drop_deferred` now when no lease is active, or once the last
/// active lease returns after close. Used to drop the `AuContext`
/// without freeing the window under a live callback.
pub(crate) fn defer_until_idle(drop_deferred: impl FnOnce() + 'static) {
    if !is_main_thread() {
        // No lease can be active on this thread's (empty) slot; a
        // cross-thread destroy drops immediately, matching the previous
        // behavior for the context box itself.
        drop_deferred();
        return;
    }
    AU_WINDOW.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.leases == 0 {
            drop(slot);
            drop_deferred();
        } else {
            slot.deferred.push(Box::new(drop_deferred));
        }
    });
}
