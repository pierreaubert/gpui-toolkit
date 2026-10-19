//! Typed UI-thread ownership with non-reused editor tokens.

// Rust guideline compliant 2026-02-21

use std::cell::RefCell;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(1);

/// Non-owning editor token, scoped to its originating registry and library.
///
/// Tokens are never reused within this library instance. Their numeric form can
/// cross a C ABI, but all operations must return to the originating library.
/// A token contains no instance pointer and does not extend an editor lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EditorHandle(NonZeroUsize);

impl EditorHandle {
    /// Decodes a numeric token; zero denotes no editor.
    ///
    /// Decoding does not validate liveness. The registry checks every lookup.
    #[must_use]
    pub fn from_usize(value: usize) -> Option<Self> {
        NonZeroUsize::new(value).map(Self)
    }

    /// Returns the opaque numeric token for ABI transport.
    #[must_use]
    pub fn as_usize(self) -> usize {
        self.0.get()
    }
}

/// The library's finite editor-token space has been exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorHandleExhausted;

impl std::fmt::Display for EditorHandleExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("editor handle space exhausted")
    }
}

impl std::error::Error for EditorHandleExhausted {}

fn allocate_handle(counter: &AtomicUsize) -> Result<EditorHandle, EditorHandleExhausted> {
    let mut value = counter.load(Ordering::Relaxed);
    loop {
        let next = value
            .checked_add(1)
            .filter(|_| value != 0)
            .ok_or(EditorHandleExhausted)?;
        match counter.compare_exchange_weak(value, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return EditorHandle::from_usize(value).ok_or(EditorHandleExhausted),
            Err(current) => value = current,
        }
    }
}

/// Owns typed editor instances on one UI thread.
///
/// Each callback leases an Rc after releasing the map borrow. Closing removes
/// the token immediately; destruction waits for callbacks already in progress.
/// Unknown, stale and other-registry tokens never dereference caller memory.
/// User destructors run outside map borrows. Empty maps release their capacity.
///
/// The registry is neither Send nor Sync, including when T itself is Send.
/// Bindings should keep it in thread-local storage and send only tokens across
/// threads. A token from another thread's registry does not resolve locally.
///
/// ```compile_fail
/// use gpui_foreign_parent::OwnedEditorRegistry;
/// let registry = OwnedEditorRegistry::<u32>::new();
/// std::thread::spawn(move || registry.len());
/// ```
pub struct OwnedEditorRegistry<T> {
    slots: RefCell<HashMap<EditorHandle, Rc<T>>>,
}

impl<T> std::fmt::Debug for OwnedEditorRegistry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedEditorRegistry")
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

impl<T> Default for OwnedEditorRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> OwnedEditorRegistry<T> {
    /// Creates an empty registry on the current thread.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: RefCell::new(HashMap::new()),
        }
    }

    /// Moves an instance into the registry under a fresh token.
    ///
    /// # Errors
    ///
    /// Returns `EditorHandleExhausted` instead of wrapping or reusing a token.
    pub fn insert(&self, instance: T) -> Result<EditorHandle, EditorHandleExhausted> {
        let handle = allocate_handle(&NEXT_HANDLE)?;
        self.slots.borrow_mut().insert(handle, Rc::new(instance));
        Ok(handle)
    }

    /// Invokes a callback while leasing the registered instance.
    ///
    /// Returns None for a closed or unknown token. The callback can register,
    /// close or clear editors, including itself, without invalidating its borrow.
    ///
    /// # Panics
    ///
    /// Propagates callback panics after closing that token. Destructors can panic.
    pub fn with<R>(&self, handle: EditorHandle, callback: impl FnOnce(&T) -> R) -> Option<R> {
        let instance = self.slots.borrow().get(&handle).cloned()?;
        let mut invocation = Invocation {
            registry: self,
            handle,
            completed: false,
        };
        let result = callback(instance.as_ref());
        invocation.completed = true;
        Some(result)
    }

    /// Closes a token immediately, deferring destruction through active callbacks.
    ///
    /// Returns false for unknown/already closed tokens. The last closed entry
    /// also releases the map's retained allocation.
    ///
    /// # Panics
    ///
    /// Propagates instance destructor panics after removing the token.
    pub fn close(&self, handle: EditorHandle) -> bool {
        let removed = {
            let mut slots = self.slots.borrow_mut();
            let removed = slots.remove(&handle);
            if slots.is_empty() {
                *slots = HashMap::new();
            }
            removed
        };
        let found = removed.is_some();
        drop(removed);
        found
    }

    /// Closes every token before running any instance destructors.
    ///
    /// Active callback leases stay valid and new registrations during a
    /// destructor are independent of the removed batch.
    ///
    /// # Panics
    ///
    /// Propagates instance destructor panics after removing the batch.
    pub fn clear(&self) {
        let removed = std::mem::take(&mut *self.slots.borrow_mut());
        drop(removed);
    }

    /// Returns the number of currently registered instances.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.borrow().len()
    }

    /// Returns whether no instances are currently registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

struct Invocation<'a, T> {
    registry: &'a OwnedEditorRegistry<T>,
    handle: EditorHandle,
    completed: bool,
}

impl<T> Drop for Invocation<'_, T> {
    fn drop(&mut self) {
        if !self.completed {
            self.registry.close(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    struct Probe {
        value: u32,
        drops: Rc<Cell<usize>>,
        on_drop: Option<Box<dyn FnOnce()>>,
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
            if let Some(f) = self.on_drop.take() {
                f();
            }
        }
    }
    fn probe(value: u32, drops: &Rc<Cell<usize>>) -> Probe {
        Probe {
            value,
            drops: drops.clone(),
            on_drop: None,
        }
    }

    #[test]
    fn close_during_callback_preserves_instance_until_return() {
        let registry = OwnedEditorRegistry::new();
        let drops = Rc::new(Cell::new(0));
        let handle = registry.insert(probe(11, &drops)).unwrap();
        assert_eq!(
            registry.with(handle, |instance| {
                assert!(registry.close(handle));
                assert!(!registry.close(handle));
                assert!(registry.with(handle, |_| ()).is_none());
                assert_eq!(drops.get(), 0);
                instance.value
            }),
            Some(11)
        );
        assert_eq!(drops.get(), 1);
        assert!(registry.is_empty());
    }

    #[test]
    fn replacement_during_callback_does_not_reuse_old_token() {
        let registry = OwnedEditorRegistry::new();
        let drops = Rc::new(Cell::new(0));
        let old = registry.insert(probe(1, &drops)).unwrap();
        let new = registry
            .with(old, |instance| {
                assert!(registry.close(old));
                let new = registry.insert(probe(2, &drops)).unwrap();
                assert_ne!(old, new);
                assert_eq!(instance.value, 1);
                assert_eq!(registry.with(new, |v| v.value), Some(2));
                new
            })
            .unwrap();
        assert_eq!(drops.get(), 1);
        assert!(registry.with(old, |_| ()).is_none());
        registry.close(new);
        assert_eq!(drops.get(), 2);
    }

    #[test]
    fn nested_leases_and_clear_close_before_drop() {
        let registry = OwnedEditorRegistry::new();
        let drops = Rc::new(Cell::new(0));
        let a = registry.insert(probe(1, &drops)).unwrap();
        let b = registry.insert(probe(2, &drops)).unwrap();
        registry
            .with(a, |first| {
                registry
                    .with(b, |second| {
                        registry.clear();
                        assert!(registry.is_empty());
                        assert_eq!(drops.get(), 0);
                        assert_eq!((first.value, second.value), (1, 2));
                    })
                    .unwrap();
                assert_eq!(drops.get(), 1);
            })
            .unwrap();
        assert_eq!(drops.get(), 2);
    }

    #[test]
    fn destructor_can_reenter_and_register() {
        let registry = Rc::new(OwnedEditorRegistry::new());
        let drops = Rc::new(Cell::new(0));
        let weak = Rc::downgrade(&registry);
        let counts = drops.clone();
        let handle = registry
            .insert(Probe {
                value: 1,
                drops: drops.clone(),
                on_drop: Some(Box::new(move || {
                    let registry = weak.upgrade().unwrap();
                    assert!(registry.is_empty());
                    registry.insert(probe(2, &counts)).unwrap();
                })),
            })
            .unwrap();
        assert!(registry.close(handle));
        assert_eq!(registry.len(), 1);
        registry.clear();
        assert_eq!(drops.get(), 2);
    }

    #[test]
    fn panic_closes_token_without_poisoning_new_registration() {
        let registry = OwnedEditorRegistry::new();
        let drops = Rc::new(Cell::new(0));
        let old = registry.insert(probe(1, &drops)).unwrap();
        let new = Cell::new(None);
        let result = catch_unwind(AssertUnwindSafe(|| {
            registry.with(old, |_| {
                registry.close(old);
                new.set(Some(registry.insert(probe(2, &drops)).unwrap()));
                panic!("callback panic")
            })
        }));
        result.unwrap_err();
        assert_eq!(drops.get(), 1);
        assert!(registry.with(old, |_| ()).is_none());
        let new = new.get().unwrap();
        assert_eq!(registry.with(new, |v| v.value), Some(2));
        registry.close(new);
    }

    #[test]
    fn panic_without_explicit_close_invalidates_token() {
        let registry = OwnedEditorRegistry::new();
        let drops = Rc::new(Cell::new(0));
        let handle = registry.insert(probe(1, &drops)).unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            registry.with::<()>(handle, |_| panic!("callback panic"))
        }));
        result.unwrap_err();
        assert!(registry.is_empty());
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn foreign_registry_and_numeric_garbage_are_rejected() {
        let a = OwnedEditorRegistry::new();
        let b = OwnedEditorRegistry::new();
        let first = a.insert(11u32).unwrap();
        let second = b.insert(22u32).unwrap();
        assert_ne!(first, second);
        assert!(a.with(second, |v| *v).is_none());
        assert!(!b.close(first));
        assert!(
            a.with(EditorHandle::from_usize(usize::MAX).unwrap(), |v| *v)
                .is_none()
        );
        assert!(EditorHandle::from_usize(0).is_none());
        assert_eq!(a.with(first, |v| *v), Some(11));
    }

    #[test]
    fn token_alone_crosses_threads_without_moving_non_send_state() {
        let registry = OwnedEditorRegistry::new();
        let value = Rc::new(Cell::new(7));
        let handle = registry.insert(value.clone()).unwrap();
        std::thread::spawn(move || {
            let registry = OwnedEditorRegistry::<Rc<Cell<u32>>>::new();
            assert!(registry.with(handle, |v| v.get()).is_none());
            assert!(!registry.close(handle));
        })
        .join()
        .unwrap();
        assert_eq!(registry.with(handle, |v| v.get()), Some(7));
        registry.close(handle);
    }

    #[test]
    fn exhaustion_never_wraps_or_allocates_zero() {
        let counter = AtomicUsize::new(usize::MAX - 1);
        assert_eq!(
            allocate_handle(&counter).unwrap().as_usize(),
            usize::MAX - 1
        );
        assert_eq!(allocate_handle(&counter), Err(EditorHandleExhausted));
        assert_eq!(allocate_handle(&counter), Err(EditorHandleExhausted));
        assert_eq!(
            allocate_handle(&AtomicUsize::new(0)),
            Err(EditorHandleExhausted)
        );
    }

    #[test]
    fn repeated_close_releases_instances_and_empty_map_storage() {
        let registry = OwnedEditorRegistry::new();
        let drops = Rc::new(Cell::new(0));
        let mut previous = None;
        for value in 0..1000 {
            let handle = registry.insert(probe(value, &drops)).unwrap();
            if let Some(old) = previous {
                assert_ne!(old, handle);
                assert!(registry.with(old, |_| ()).is_none());
            }
            assert!(registry.close(handle));
            assert_eq!(registry.slots.borrow().capacity(), 0);
            previous = Some(handle);
        }
        assert_eq!(drops.get(), 1000);
    }
}
