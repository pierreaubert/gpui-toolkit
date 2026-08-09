//! Registry for embedder-provided custom GPU draw callbacks (MeshPlot).
//! Local patch on top of zed v1.9.0 — see PATCHES.md.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    REGISTRY.with(|r| {
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
    fn ids_are_unique_and_monotonic() {
        let a = register_custom_draw(Rc::new(Stub));
        let b = register_custom_draw(Rc::new(Stub));
        assert_ne!(a, b);
        unregister_custom_draw(a);
        unregister_custom_draw(b);
    }
}
