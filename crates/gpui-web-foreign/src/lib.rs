//! Foreign-parent editor embedding for the browser.
//!
//! Implements the wasm binding of `docs/foreign-parent-editor-embedding.md`:
//! GPUI editors that render into page-owned `<canvas>` elements instead of a
//! GPUI-created full-page canvas. The page provides the canvas and its CSS
//! size; this crate installs the `ResizeObserver`, the `requestAnimationFrame`
//! pump, and the pointer/keyboard/composition listeners, and drives the
//! shared host/guest policies (frame throttle, resize debounce,
//! transition-only focus/hover).
//!
//! Renderer providers plug in through [`host::ForeignRenderer`]: the shell
//! forwards frames, resizes, inputs, and composition ops to it. Without a
//! provider the shell runs headless (observers stay live, frames count as
//! dropped); attaching `gpui_web`'s renderer to a page canvas needs the
//! small fork change specified in contract §16.
//!
//! [`input`] holds the DOM mapping tables as pure functions with host-side
//! unit tests; [`host`] (wasm only) owns the live DOM objects.

pub mod input;

#[cfg(any(target_family = "wasm", test))]
mod dispatch;

#[cfg(target_family = "wasm")]
pub mod host;
