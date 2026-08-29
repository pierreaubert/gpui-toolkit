//! Registry for embedder-provided custom GPU draw callbacks (MeshPlot).
//! Local patch on top of zed v1.9.0 — see PATCHES.md.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set by the wgpu renderer (`gpui_wgpu`) when it initializes, meaning
/// `WgpuCustomDraw` primitives registered in this process will actually be
/// dispatched. Chart elements probe this to pick GPU vs CPU rasterization.
static WGPU_CUSTOM_DRAW_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Whether the active renderer dispatches `WgpuCustomDraw` primitives.
pub fn wgpu_custom_draw_available() -> bool {
    WGPU_CUSTOM_DRAW_AVAILABLE.load(Ordering::Acquire)
}

/// Called by `gpui_wgpu::WgpuRenderer` on init. Not app API.
#[doc(hidden)]
pub fn set_wgpu_custom_draw_available(available: bool) {
    WGPU_CUSTOM_DRAW_AVAILABLE.store(available, Ordering::Release);
}

/// Opaque identifier for a registered [`CustomDraw`] callback.
pub type CustomDrawId = u64;

/// Platform-agnostic handle. Platform renderers downcast via `as_any`
/// to their own subtrait (e.g. `gpui_wgpu::WgpuCustomDraw`,
/// `gpui_macos::MetalCustomDraw`).
pub trait CustomDraw: 'static {
    /// Downcast helper for platform renderers.
    fn as_any(&self) -> &dyn Any;
}

thread_local! {
    static REGISTRY: RefCell<(CustomDrawId, HashMap<CustomDrawId, Rc<dyn CustomDraw>>)> =
        RefCell::new((1, HashMap::new()));
}

/// Register a custom draw callback and return its id. Main-thread only
/// (the registry is `thread_local` with `Rc`, matching GPUI's threading model).
pub fn register_custom_draw(draw: Rc<dyn CustomDraw>) -> CustomDrawId {
    REGISTRY.with(|r| {
        let mut r = r.borrow_mut();
        let id = r.0;
        r.0 += 1;
        r.1.insert(id, draw);
        id
    })
}

/// Remove a previously registered custom draw callback.
pub fn unregister_custom_draw(id: CustomDrawId) {
    // A custom-draw owner can be retained by another thread-local cache. If
    // that cache is dropped after `REGISTRY` during thread teardown, `with`
    // would panic because the registry has already been destroyed. There is
    // nothing left to unregister in that case, so cleanup is deliberately a
    // no-op.
    let _ = REGISTRY.try_with(|r| {
        r.borrow_mut().1.remove(&id);
    });
}

/// Look up a registered custom draw callback.
///
/// `pub` (not `pub(crate)`) so the platform renderer crates (`gpui_wgpu`,
/// `gpui_macos`) — which are separate crates — can resolve ids while drawing.
pub fn lookup_custom_draw(id: CustomDrawId) -> Option<Rc<dyn CustomDraw>> {
    REGISTRY.with(|r| r.borrow().1.get(&id).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Stub;
    impl CustomDraw for Stub {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn register_lookup_unregister_roundtrip() {
        let id = register_custom_draw(Rc::new(Stub));
        assert!(lookup_custom_draw(id).is_some());
        unregister_custom_draw(id);
        assert!(lookup_custom_draw(id).is_none());
    }

    #[test]
    fn wgpu_custom_draw_flag_roundtrip() {
        assert!(!wgpu_custom_draw_available());
        set_wgpu_custom_draw_available(true);
        assert!(wgpu_custom_draw_available());
        set_wgpu_custom_draw_available(false);
        assert!(!wgpu_custom_draw_available());
    }

    #[test]
    fn ids_are_unique_and_monotonic() {
        let a = register_custom_draw(Rc::new(Stub));
        let b = register_custom_draw(Rc::new(Stub));
        assert_ne!(a, b);
        unregister_custom_draw(a);
        unregister_custom_draw(b);
    }

    #[test]
    fn unregister_ignores_a_registry_already_destroyed_during_tls_teardown() {
        struct LateRegistration(CustomDrawId);

        impl Drop for LateRegistration {
            fn drop(&mut self) {
                unregister_custom_draw(self.0);
            }
        }

        thread_local! {
            // Initialize this cache before `REGISTRY`; thread-local destructors
            // run in reverse initialization order, reproducing a retained custom
            // draw registration that outlives the registry on its thread.
            static LATE_REGISTRATION: RefCell<Option<LateRegistration>> = const { RefCell::new(None) };
        }

        let worker = std::thread::spawn(|| {
            LATE_REGISTRATION.with(|late_registration| {
                let id = register_custom_draw(Rc::new(Stub));
                *late_registration.borrow_mut() = Some(LateRegistration(id));
            });
        });

        assert!(worker.join().is_ok());
    }
}
