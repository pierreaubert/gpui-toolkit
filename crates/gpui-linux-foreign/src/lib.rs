//! Foreign-parent editor embedding for Linux.
//!
//! GPUI editors hosted inside host-owned X11 windows (VST3, CLAP-x11)
//! and Wayland surfaces (CLAP-wayland). The window-creation hooks live
//! in the `gpui_linux` fork backend (X11 child-window path, Wayland
//! subsurface path); this crate owns the editor lifecycle, frame pump,
//! resize policy, and the C ABI the host calls.
//!
//! Contract: `docs/foreign-parent-editor-embedding.md` §10 (Linux row).
//! Host call order per backend: `create` → `request_frame` on a timer →
//! `resize`/`set_active`/`set_hovered`/input as needed → `destroy` once.
//!
//! The [`geometry`] and [`input`] modules are host-portable pure logic;
//! [`editor`], [`x11`], [`wayland`], and [`ffi`] need Linux.

pub mod geometry;
pub mod input;

#[cfg(any(target_os = "linux", test))]
mod lifecycle;

#[cfg(target_os = "linux")]
pub mod editor;
#[cfg(target_os = "linux")]
pub mod ffi;
#[cfg(target_os = "linux")]
pub mod wayland;
#[cfg(target_os = "linux")]
pub mod x11;
