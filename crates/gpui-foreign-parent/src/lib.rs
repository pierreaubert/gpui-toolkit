//! Platform-neutral core for foreign-parent editor embedding.
//!
//! This crate implements migration item M1 of
//! `docs/foreign-parent-editor-embedding.md`: the host/guest policy shared
//! by every binding that renders GPUI inside a host-owned view (plugin
//! editors, embedded panels, foreign canvases).
//!
//! Contents:
//! - [`FrameScheduler`]: the common dirty/throttle/resize frame policy
//!   every binding's host pump drives (clean ticks render nothing).
//! - [`OwnedEditorRegistry`]: owned editor instances under non-reused
//!   checked tokens, UI-thread confined with callback leases. (The old
//!   borrowed-pointer `EditorRegistry` was removed once every binding
//!   migrated; see audit-20261113 step 2.)
//! - [`FrameThrottle`]: host-pump overdrive coalescing with health
//!   counters.
//! - [`ResizeDebounce`]: debounced surface reconfiguration that always
//!   converges to the newest host size.
//! - [`CallbackSlot`] / [`TransitionCell`]: take–invoke–restore callback
//!   storage and transition-only state notification.
//! - [`ffi`]: null-safe C-string and two-phase out-buffer helpers for FFI
//!   boundaries.
//!
//! No platform code, no GPU code, no GPUI dependency: bindings translate
//! native handles and events at their boundary and drive these policies.

mod debounce;
mod ffi;
mod owned_registry;
mod scheduler;
mod slot;
pub mod sync;
mod throttle;

#[doc(inline)]
pub use debounce::{DEFAULT_RESIZE_DEBOUNCE_INTERVAL, ResizeDebounce};
#[doc(inline)]
pub use ffi::{ffi_boundary, fill_out_buffer, optional_c_string};
#[doc(inline)]
pub use owned_registry::{EditorHandle, EditorHandleExhausted, OwnedEditorRegistry};
#[doc(inline)]
pub use scheduler::{FrameDecision, FrameScheduler};
#[doc(inline)]
pub use slot::{CallbackSlot, TransitionCell};
#[doc(inline)]
pub use throttle::{DEFAULT_FRAME_MIN_INTERVAL, FrameThrottle};
