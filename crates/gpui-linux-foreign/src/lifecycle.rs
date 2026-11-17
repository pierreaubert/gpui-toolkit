//! Typed local ownership, backend checks and release-mode thread affinity.

// Rust guideline compliant 2026-02-21

use gpui_foreign_parent::{EditorHandle, EditorHandleExhausted, OwnedEditorRegistry};
use std::cell::Cell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::OnceLock;
use std::thread::{ThreadId, current};

/// Native parent kind associated with an editor token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForeignBackend {
    /// Host X11 window (VST3, CLAP-x11).
    X11,
    /// Host Wayland surface (CLAP-wayland).
    Wayland,
}

/// Opaque numeric editor token; never dereference or free this address.
#[derive(Debug)]
#[repr(C)]
pub struct LinuxForeignContext {
    _private: [u8; 0],
}

pub(crate) fn context_handle(context: *mut LinuxForeignContext) -> Option<EditorHandle> {
    EditorHandle::from_usize(context.addr())
}

pub(crate) fn context_from_handle(handle: EditorHandle) -> *mut LinuxForeignContext {
    std::ptr::without_provenance_mut(handle.as_usize())
}

pub(crate) fn id_handle(id: u64) -> Option<EditorHandle> {
    EditorHandle::from_usize(usize::try_from(id).ok()?)
}

pub(crate) struct ThreadAffinity {
    owner: OnceLock<ThreadId>,
}
impl ThreadAffinity {
    pub(crate) const fn new() -> Self {
        Self {
            owner: OnceLock::new(),
        }
    }
    pub(crate) fn permits_create(&self) -> bool {
        self.owner
            .get()
            .is_none_or(|owner| *owner == current().id())
    }
    pub(crate) fn claim(&self) -> bool {
        *self.owner.get_or_init(|| current().id()) == current().id()
    }
    pub(crate) fn is_current(&self) -> bool {
        self.owner
            .get()
            .is_some_and(|owner| *owner == current().id())
    }
    /// Whether a different thread has claimed affinity (vs. nobody has).
    #[cfg_attr(
        not(target_os = "linux"),
        allow(dead_code, reason = "used by the Linux editor registry")
    )]
    pub(crate) fn claimed_elsewhere(&self) -> bool {
        self.owner
            .get()
            .is_some_and(|owner| *owner != current().id())
    }
}

thread_local! {
    static ATTACHING: Cell<bool> = const { Cell::new(false) };
}

/// Rejects nested use of the backend's one-shot pending-parent handoffs.
pub(crate) struct AttachGuard(PhantomData<Rc<()>>);
impl AttachGuard {
    pub(crate) fn enter() -> Option<Self> {
        ATTACHING
            .try_with(|flag| {
                if flag.get() {
                    None
                } else {
                    flag.set(true);
                    Some(Self(PhantomData))
                }
            })
            .ok()
            .flatten()
    }
}
impl Drop for AttachGuard {
    fn drop(&mut self) {
        let _ = ATTACHING.try_with(|flag| flag.set(false));
    }
}

pub(crate) struct EditorState<T, P> {
    // Declaration order keeps the app pin alive through editor resource drops.
    value: T,
    backend: ForeignBackend,
    closed: Cell<bool>,
    _pin: P,
}
impl<T, P> Deref for EditorState<T, P> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T, P> EditorState<T, P> {
    pub(crate) fn is_closed(&self) -> bool {
        self.closed.get()
    }
}

pub(crate) struct LocalEditors<T, P> {
    slots: OwnedEditorRegistry<EditorState<T, P>>,
}
impl<T, P> LocalEditors<T, P> {
    pub(crate) fn new() -> Self {
        Self {
            slots: OwnedEditorRegistry::new(),
        }
    }
    pub(crate) fn insert(
        &self,
        backend: ForeignBackend,
        value: T,
        pin: P,
    ) -> Result<EditorHandle, EditorHandleExhausted> {
        self.slots.insert(EditorState {
            value,
            backend,
            closed: Cell::new(false),
            _pin: pin,
        })
    }
    pub(crate) fn with<R>(
        &self,
        handle: EditorHandle,
        backend: ForeignBackend,
        callback: impl FnOnce(&EditorState<T, P>) -> R,
    ) -> Option<R> {
        self.slots
            .with(handle, |state| {
                if state.backend != backend || state.closed.get() {
                    return None;
                }
                let mut guard = CallbackGuard {
                    closed: &state.closed,
                    completed: false,
                };
                let result = callback(state);
                guard.completed = true;
                Some(result)
            })
            .flatten()
    }
    pub(crate) fn close(&self, handle: EditorHandle, backend: ForeignBackend) -> bool {
        self.slots
            .with(handle, |state| {
                if state.backend != backend {
                    return false;
                }
                state.closed.set(true);
                self.slots.close(handle)
            })
            .unwrap_or(false)
    }
}
struct CallbackGuard<'a> {
    closed: &'a Cell<bool>,
    completed: bool,
}
impl Drop for CallbackGuard<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.closed.set(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::{Arc, Barrier};

    #[test]
    fn tokens_decode_without_dereferencing_caller_memory() {
        assert!(context_handle(std::ptr::null_mut()).is_none());
        assert!(id_handle(0).is_none());
        let editors = LocalEditors::new();
        let handle = editors.insert(ForeignBackend::X11, 7, ()).unwrap();
        let context = context_from_handle(handle);
        assert_eq!(context_handle(context), Some(handle));
        assert_eq!(
            id_handle(u64::try_from(context.addr()).unwrap()),
            Some(handle)
        );
        let forged = std::ptr::without_provenance_mut(usize::MAX);
        assert!(
            editors
                .with(context_handle(forged).unwrap(), ForeignBackend::X11, |_| ())
                .is_none()
        );
        assert!(editors.close(handle, ForeignBackend::X11));
        assert!(
            editors
                .with(
                    context_handle(context).unwrap(),
                    ForeignBackend::X11,
                    |_| ()
                )
                .is_none()
        );
    }

    #[test]
    fn wrong_backend_cannot_invoke_or_close_an_editor() {
        let editors = LocalEditors::new();
        let handle = editors.insert(ForeignBackend::X11, 7, ()).unwrap();
        assert!(
            editors
                .with(handle, ForeignBackend::Wayland, |_| panic!(
                    "wrong backend invoked"
                ))
                .is_none()
        );
        assert!(!editors.close(handle, ForeignBackend::Wayland));
        assert_eq!(
            editors.with(handle, ForeignBackend::X11, |state| **state),
            Some(7)
        );
        editors.close(handle, ForeignBackend::X11);
    }

    #[test]
    fn close_defers_native_resource_and_app_pin_drop_until_return() {
        struct Native(Rc<RefCell<Vec<&'static str>>>, std::rc::Weak<()>);
        impl Drop for Native {
            fn drop(&mut self) {
                assert!(self.1.upgrade().is_some());
                self.0.borrow_mut().push("native");
            }
        }
        struct Pin(Rc<RefCell<Vec<&'static str>>>, Rc<()>);
        impl Drop for Pin {
            fn drop(&mut self) {
                assert_eq!(*self.0.borrow(), ["native"]);
                self.0.borrow_mut().push("pin");
                assert_eq!(Rc::strong_count(&self.1), 1);
            }
        }
        let drops = Rc::new(RefCell::new(Vec::new()));
        let pin = Rc::new(());
        let weak = Rc::downgrade(&pin);
        let editors = LocalEditors::new();
        let handle = editors
            .insert(
                ForeignBackend::Wayland,
                Native(drops.clone(), weak.clone()),
                Pin(drops.clone(), pin),
            )
            .unwrap();
        editors
            .with(handle, ForeignBackend::Wayland, |state| {
                assert!(!state.is_closed());
                assert!(editors.close(handle, ForeignBackend::Wayland));
                assert!(state.is_closed());
                assert!(drops.borrow().is_empty());
                assert!(weak.upgrade().is_some());
                assert!(
                    editors
                        .with(handle, ForeignBackend::Wayland, |_| ())
                        .is_none()
                );
            })
            .unwrap();
        assert_eq!(*drops.borrow(), ["native", "pin"]);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn caught_nested_panic_closes_the_outer_active_lease() {
        let editors = LocalEditors::new();
        let handle = editors.insert(ForeignBackend::X11, 7, ()).unwrap();
        editors
            .with(handle, ForeignBackend::X11, |state| {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    editors.with::<()>(handle, ForeignBackend::X11, |_| panic!("nested callback"))
                }));
                result.unwrap_err();
                assert!(state.is_closed());
                assert!(editors.with(handle, ForeignBackend::X11, |_| ()).is_none());
            })
            .unwrap();
    }

    #[test]
    fn repeated_cycles_and_queued_stale_tokens_cannot_close_replacements() {
        let editors = LocalEditors::new();
        let mut previous = None;
        for value in 0..1000 {
            let handle = editors.insert(ForeignBackend::X11, value, ()).unwrap();
            if let Some(old) = previous {
                assert_ne!(handle, old);
                assert!(!editors.close(old, ForeignBackend::X11));
            }
            assert_eq!(
                editors.with(handle, ForeignBackend::X11, |state| **state),
                Some(value)
            );
            assert!(editors.close(handle, ForeignBackend::X11));
            previous = Some(handle);
        }
    }

    #[test]
    fn affinity_rejects_a_different_thread_in_all_profiles() {
        let affinity = Arc::new(ThreadAffinity::new());
        assert!(affinity.permits_create());
        assert!(!affinity.is_current());
        assert!(affinity.claim());
        let clone = affinity.clone();
        std::thread::spawn(move || {
            assert!(!clone.permits_create());
            assert!(!clone.claim());
            assert!(!clone.is_current());
        })
        .join()
        .unwrap();
        assert!(affinity.is_current());
    }

    #[test]
    fn concurrent_claim_has_exactly_one_owner() {
        let affinity = Arc::new(ThreadAffinity::new());
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let affinity = affinity.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    affinity.claim()
                })
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .filter(|claimed| *claimed)
                .count(),
            1
        );
    }

    #[test]
    fn nested_attach_rejection_keeps_the_outer_guard_and_panic_releases_it() {
        let result = catch_unwind(|| {
            let guard = AttachGuard::enter().unwrap();
            assert!(AttachGuard::enter().is_none());
            assert!(AttachGuard::enter().is_none());
            drop(guard);
            let _guard = AttachGuard::enter().unwrap();
            panic!("root factory");
        });
        result.unwrap_err();
        assert!(AttachGuard::enter().is_some());
    }
}
